#![deny(warnings)]

use anyhow::{anyhow, bail, ensure, Context};
use clap::Parser;
use config::{read_from_file, write_to_file, Config, Section};
use date::Date;
use provider::accuweather::AccuWeather;
use provider::WeatherInfo;
use std::borrow::Cow;
use std::future::{Future, IntoFuture};
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;

use crate::provider::openweather::OpenWeather;
use crate::provider::weatherapi::WeatherApi;
use crate::provider::{ParamDesc, ProviderInfo};
use crate::provider_registry::ProviderRegistry;

mod config;
mod date;
mod provider;
mod provider_registry;
mod utils;

/// Used as shortcut alias for any boxed future
type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;
/// Shortcut for COW string, either static or on-heap
type CowString = Cow<'static, str>;
/// Default location used to verify provider's configuration by sending dummy request
const DEFAULT_CONFIGURE_LOCATION: &str = "London";
/// Name of config entry with currently active provider
const ACTIVE_ENTRY: &str = "current";

fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let Cli { config, command } = Cli::parse();

    let (mut config, config_path) = read_from_file(config)?;
    // Fill in providers registry
    let mut registry = ProviderRegistry::new();

    registry.add_provider::<AccuWeather>("accuweather");
    registry.add_provider::<OpenWeather>("openweather");
    registry.add_provider::<WeatherApi>("weatherapi");
    // Execute CLI command
    match command {
        CliCmd::Configure {
            provider,
            parameters,
        } => {
            configure_provider(&registry, &mut config, provider.clone(), parameters)?;
            println!("Successfully configured provider '{provider}'");
        }
        CliCmd::Get {
            address,
            date,
            provider,
            set_default,
        } => {
            let forecast =
                get_forecast(&registry, &mut config, address, date, provider, set_default)?;
            println!("{forecast}");
        }
        CliCmd::Clear { providers } => clear_providers(&registry, &mut config, providers)?,
        CliCmd::List => list_providers(&registry),
    }
    // If all operations succeeded, write updated config back to file
    write_to_file(&config, config_path)?;
    // End of processing
    Ok(())
}
/// Executes future using lightweight current-thread scheduler
/// 
/// # Parameters
/// * `future` - input object convertible into future which produces `Result`
/// 
/// # Returns
/// Future's execution result
fn run_future<R>(future: impl IntoFuture<Output = anyhow::Result<R>>) -> anyhow::Result<R> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future.into_future())
}

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
    /// List available providers and their configuration parameters
    List,
}
/// Configures specified provider, either with provided key-value parameters or interactively
fn configure_provider(
    registry: &ProviderRegistry,
    config: &mut Config,
    provider: String,
    parameters: Vec<String>,
) -> anyhow::Result<()> {
    // Check that provider is valid and get factory
    let factory = registry
        .get(provider.as_str())
        .ok_or_else(|| anyhow!("No such provider: {provider}"))?;

    let ProviderInfo { params, .. } = factory.info();
    // Generate new config
    let mut new_config = Section::new();
    // Interactive configuration
    if parameters.is_empty() && !params.is_empty() {
        for ParamDesc { id, name, .. } in *params {
            println!("Please enter {name}:");
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer)?;
            new_config.insert(id.to_string(), buffer);
        }
    }
    // Batch configuration
    else {
        for param in parameters {
            let (name, value) = param.split_once('=').ok_or_else(|| {
                anyhow!("Argument '{param}' cannot be parsed as '<name>=<value>' parameter")
            })?;
            // Check that parameter is required by provider
            // NB: Yes, it's a linear search.
            // Doesn't matter here - we have very few parameters,
            // so may be even faster than build dictionary
            ensure!(
                params.iter().any(|param| param.id == name),
                "Parameter '{name}' isn't accepted by provider '{provider}'"
            );

            new_config.insert(name.to_string(), value.to_string());
        }
        // Check that all necessary parameters are present
        for ParamDesc { id, .. } in *params {
            ensure!(
                new_config.contains_key(*id),
                "Parameter '{id}' is required by provider '{provider}'"
            )
        }
    }
    // Perform simple request to check configuration is actually valid
    {
        let prov_config_error = || || anyhow!("When configuring {provider}");

        let provider = factory
            .create(&new_config)
            .with_context(prov_config_error())?;

        let _ = run_future(provider.get_weather(DEFAULT_CONFIGURE_LOCATION.into(), None))
            .with_context(prov_config_error())?;
    }
    // If check succeeded, write new config entry; if config was empty prior to first configure,
    // set new provider as default one
    if config.sections.is_empty() {
        config.globals.insert(ACTIVE_ENTRY.into(), provider.clone());
    }
    config.sections.insert(provider, new_config);

    Ok(())
}
/// Gets weather forecast using specified provider
fn get_forecast(
    registry: &ProviderRegistry,
    config: &mut Config,
    address: String,
    date: String,
    provider: Option<String>,
    set_default: bool,
) -> anyhow::Result<WeatherInfo> {
    // Fetch actual provider name
    let provider_name = if let Some(provider) = provider {
        provider
    } else {
        config.globals.get(ACTIVE_ENTRY)
            .ok_or_else(|| anyhow!(
                "Active provider not specified. Please use `-sp <provider_name>` to specify new default one"
            ))?
            .clone()
    };
    // Create factory
    let factory = registry
        .get(provider_name.as_str())
        .ok_or_else(|| anyhow!("No such provider: {provider_name}"))?;
    // Get provider's config
    let prov_config = config
        .sections
        .get(provider_name.as_str())
        .ok_or_else(|| anyhow!("Missing config for provider '{provider_name}'"))?;
    // Spawn provider
    let provider = factory
        .create(prov_config)
        .with_context(|| anyhow!("When trying to construct provider '{provider_name}'"))?;
    // Parse date
    let date = if date == "now" {
        None
    } else {
        Some(Date::from_str(&date).with_context(|| anyhow!("Could not parse forecast date"))?)
    };

    let result = run_future(provider.get_weather(address.into(), date))
        .with_context(|| anyhow!("When performing forecast request"))?;
    // Set provider as default - if requested
    if set_default {
        config
            .globals
            .insert(ACTIVE_ENTRY.to_string(), provider_name);
    }

    Ok(result)
}

fn clear_providers(
    registry: &ProviderRegistry,
    config: &mut Config,
    providers: Vec<String>,
) -> anyhow::Result<()> {
    // Walk all mentioned providers and remove them
    for prov_name in &providers {
        // "all" means all providers
        if prov_name == "all" {
            for name in registry.keys() {
                config.sections.remove(name.as_ref());
            }
        } else if registry.contains_key(prov_name.as_str()) {
            config.sections.remove(prov_name);
        } else {
            bail!("No such provider: {prov_name}");
        }
    }
    // If there's default entry, and default provider isn't registered,
    // clear it
    if let Some(default_entry) = config.globals.get(ACTIVE_ENTRY) {
        if !config.sections.contains_key(default_entry.as_str()) {
            config.globals.remove(ACTIVE_ENTRY);
        }
    }

    Ok(())
}

fn list_providers(registry: &ProviderRegistry) {
    for (id, factory) in registry.iter() {
        let ProviderInfo {
            description,
            params,
        } = factory.info();
        println!("{id}: {description}");
        if !params.is_empty() {
            println!("  Parameters:");
            for ParamDesc {
                id,
                name,
                description,
            } in *params
            {
                println!("    {id:<16} - {name}, {description}");
            }
        }
        println!();
    }
}
