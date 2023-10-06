#![deny(warnings)]

use anyhow::{anyhow, bail, Context};
use chrono::Datelike;
use clap::Parser;
use serde::Deserialize;
use std::borrow::Cow;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use toml::de::ValueDeserializer;
use toml::value::Date;
use toml::Value;

use crate::provider::openweather::OpenWeather;
use crate::provider::weatherapi::WeatherApi;
use crate::provider_registry::ProviderRegistry;

mod provider;
mod provider_registry;
/// Used as shortcut alias for any boxed future
type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;
/// Shortcut for COW string, either static or on-heap
type CowString = Cow<'static, str>;
/// Default location used to verify provider's configuration by sending dummy request
const DEFAULT_CONFIGURE_LOCATION: &str = "London";
/// Name of config entry with currently active provider
const ACTIVE_ENTRY: &str = "current";

/// Command-line client for weather forecast services
#[derive(clap::Parser)]
struct Cli {
    /// Path to alternative config file
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: CliCmd,
}
/// Weather client command
#[derive(clap::Subcommand)]
enum CliCmd {
    /// Configure specified forecast provider
    ///
    /// Configuration is specified as a sequence of "<name>=<value>" space-separated entries.
    /// If no configuration values are specified, runs in interactive mode
    Configure {
        /// Name of provider to configure
        provider: String,
        /// Configuration parameters specified as "<name>=<value>" arguments
        parameters: Vec<String>,
    },
    /// Get forecast data using specified provider
    Get {
        /// Address of location for which weather is requested
        address: String,
        /// Date of weather forecast; can be either "YYYY-MM-DD" or "now", in latter case corresponds to current local date
        #[arg(short, long, default_value = "now")]
        date: String,
        /// Use specified provider instead of default one
        #[arg(short, long)]
        provider: Option<String>,
        /// Set explicitly specified provider as default one. Works only with '--provider' argument
        #[arg(short, long)]
        set_default: bool,
    },
    /// Clear configuration of specified or all providers
    Clear {
        /// Names of providers whose configurations to clear; specify "all" to clear all providers
        providers: Vec<String>,
    },
}
/// Get today's date as TOML `Date`
///
/// # Returns
/// Today's date as TOML `Date` object
fn date_now() -> Date {
    let date = chrono::Local::now().date_naive();
    Date {
        year: date.year() as u16,
        month: date.month() as u8,
        day: date.day() as u8,
    }
}
/// Read app's configuration at specified path; if path isn't provided, default config path is used
///
/// # Parameters
/// * `path` - optional config path
///
/// # Returns
/// Parsed configuration as TOML table and path to it
async fn read_config(path: Option<PathBuf>) -> anyhow::Result<(toml::Table, PathBuf)> {
    // Fetch path to config file
    let config_path = if let Some(path) = path {
        path
    } else if let Some(path) = dirs::config_dir() {
        path.join("weather-cli").join("config.toml")
    } else if let Some(path) = dirs::home_dir() {
        path.join(".weather-cli.toml")
    } else {
        bail!(
            "Current OS doesn't seem to have notion of either user's config directory or user's home directory. Please use explicit '--config' argument"
        )
    };

    // Read config file itself - if it exists
    let config = if config_path.is_file() {
        let contents = tokio::fs::read_to_string(&config_path)
            .await
            .with_context(|| anyhow!("When reading config file '{}'", config_path.display()))?;
        toml::from_str(&contents)
            .with_context(|| anyhow!("When parsing config file '{}'", config_path.display()))?
    } else if config_path.exists() {
        bail!(
            "Path '{}' exists yet points not to file",
            config_path.display()
        )
    } else {
        toml::Table::new()
    };

    Ok((config, config_path))
}
/// Writes app's configuration at specified path
///
/// # Parameters
/// * `config` - configuration object
/// * `path` - path where to write configuration
async fn write_config(config: toml::Table, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let config_path = path.as_ref();
    // Write config back to file
    if !config_path.is_file() {
        let Some(config_dir_path) = config_path.parent() else {
            // Config path points either to existing file
            // or to some nonexistent location - so it cannot be just root path
            // whose parent would be `None`
            unreachable!()
        };
        tokio::fs::create_dir_all(config_dir_path)
            .await
            .with_context(|| {
                anyhow!(
                    "When creating config directory {}",
                    config_dir_path.display()
                )
            })?;
    }

    tokio::fs::write(
        &config_path,
        toml::to_string_pretty(&config)
            .with_context(|| anyhow!("When serializing configuration data"))?,
    )
    .await
    .with_context(|| anyhow!("When writing configuration to {}", config_path.display()))
}
/// Configures specified provider, either with provided key-value parameters or interactively
async fn configure_provider(
    registry: &ProviderRegistry,
    config: &mut toml::Table,
    provider: String,
    parameters: Vec<String>,
) -> anyhow::Result<()> {
    // Check that provider is valid and get factory
    let factory = registry
        .get(provider.as_str())
        .ok_or_else(|| anyhow!("No such provider: {provider}"))?;
    // Interactive configuration: TODO
    if parameters.is_empty() {
        bail!("Sorry, interactive mode not implemented yet");
    }
    // Generate new config
    let mut new_config = toml::Table::new();

    for param in parameters {
        let (name, value) = param.split_once('=').ok_or_else(|| {
            anyhow!("Argument '{param}' cannot be parsed as '<name>=<value>' parameter")
        })?;
        let value = toml::Value::deserialize(ValueDeserializer::new(value))
            .with_context(|| anyhow!("When parsing value of parameter {param}"))?;

        new_config.insert(name.to_string(), value);
    }
    // Perform simple request to check configuration is actually valid
    {
        let prov_config_error = || || anyhow!("When configuring {provider}");

        let provider = factory
            .create(new_config.clone().into())
            .with_context(prov_config_error())?;

        let _ = provider
            .read_weather(DEFAULT_CONFIGURE_LOCATION.into(), date_now())
            .await
            .with_context(prov_config_error())?;
    }
    // If check succeeded, write new config entry; if config was empty prior to first configure,
    // set new provider as default one
    if config.is_empty() {
        config.insert(ACTIVE_ENTRY.into(), provider.clone().into());
    }
    config.insert(provider, new_config.into());

    Ok(())
}
/// Gets weather forecast using specified provider
#[allow(unused)]
async fn get_forecast(
    registry: &ProviderRegistry,
    config: &mut toml::Table,
    address: String,
    date: String,
    provider: Option<String>,
    set_default: bool,
) -> anyhow::Result<String> {
    // Fetch actual provider name
    let provider_name = if let Some(provider) = provider {
        provider
    } else {
        let entry = config.get(ACTIVE_ENTRY)
            .ok_or_else(|| anyhow!(
                "Active provider not specified. Please use `-sp <provider_name>` to specify new default one"
            ))?;

        entry.as_str().ok_or_else(|| anyhow!(
            "Invalid config entry! Please set new current provider manually via `-sp <provider_name>`")
        )?.to_string()
    };
    // Create factory
    let factory = registry
        .get(provider_name.as_str())
        .ok_or_else(|| anyhow!("No such provider: {provider_name}"))?;
    // Get provider's config
    let config = config
        .get(provider_name.as_str())
        .ok_or_else(|| anyhow!("Missing config for provider '{provider_name}'"))?;
    // Spawn provider
    let provider = factory
        .create(config.clone())
        .with_context(|| anyhow!("When trying to construct provider '{provider_name}'"))?;
    // Parse date
    let date = if date == "now" {
        date_now()
    } else {
        toml::value::Datetime::from_str(&date)
            .with_context(|| anyhow!("When parsing forecast date"))?
            .date
            .ok_or_else(|| anyhow!("Missing actual forecast date"))?
    };

    provider.read_weather(address.into(), date).await
}

fn clear_providers(
    registry: &ProviderRegistry,
    config: &mut toml::Table,
    providers: Vec<String>,
) -> anyhow::Result<()> {
    // Walk all mentioned providers and remove them
    for prov_name in &providers {
        // "all" means all providers
        if prov_name == "all" {
            for name in registry.keys() {
                config.remove(name.as_ref());
            }
        } else if registry.contains_key(prov_name.as_str()) {
            config.remove(prov_name);
        } else {
            bail!("No such provider: {prov_name}");
        }
    }
    // If there's default entry, and default provider isn't registered,
    // clear it
    if let Some(Value::String(default_entry)) = config.get(ACTIVE_ENTRY) {
        if !config.contains_key(default_entry.as_str()) {
            config.remove(ACTIVE_ENTRY);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let Cli { config, command } = Cli::parse();

    let (mut config, config_path) = read_config(config).await?;
    // Fill in providers registry
    let mut registry = ProviderRegistry::new();

    registry.add_provider::<OpenWeather>("openweather");
    registry.add_provider::<WeatherApi>("weatherapi");
    // Execute CLI command
    match command {
        CliCmd::Configure {
            provider,
            parameters,
        } => configure_provider(&registry, &mut config, provider, parameters).await?,
        CliCmd::Get {
            address,
            date,
            provider,
            set_default,
        } => {
            let forecast =
                get_forecast(&registry, &mut config, address, date, provider, set_default).await?;
            println!("{forecast}");
        }
        CliCmd::Clear { providers } => clear_providers(&registry, &mut config, providers)?,
    }

    // let stub_config = toml::toml! {
    //     apikey = "banana"
    // }
    // .into();

    // let prov = registry
    //     .get("weatherapi")
    //     .ok_or_else(|| anyhow::anyhow!("No such provider"))?
    //     .create(stub_config)?;

    // let forecast = prov
    //     .read_weather(
    //         // Approx location of London
    //         51.5072,
    //         0.1275,
    //         date_now(),
    //     )
    //     .await?;

    // println!("{forecast}");

    write_config(config, config_path).await?;
    // End of processing
    Ok(())
}
