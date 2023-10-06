#![deny(warnings)]

use chrono::Datelike;
use clap::Parser;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use toml::value::Date;

use crate::provider::openweather::OpenWeather;
use crate::provider::weatherapi::WeatherApi;
use crate::provider_registry::ProviderRegistry;

mod provider;
mod provider_registry;
/// Used as shortcut alias for any boxed future
type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

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
        /// Configuration options specified as "<name>=<value>" arguments
        options: Vec<String>,
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
fn date_now() -> Date {
    let date = chrono::Local::now().date_naive();
    Date {
        year: date.year() as u16,
        month: date.month() as u8,
        day: date.day() as u8,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let Cli { config, command } = Cli::parse();
    // Fetch path to config file
    let config_path = if let Some(path) = config {
        path
    } else if let Some(path) = dirs::config_dir() {
        path.join("weather-cli").join("config.toml")
    } else if let Some(path) = dirs::home_dir() {
        path.join(".weather-cli.toml")
    } else {
        anyhow::bail!(
            "Current OS doesn't seem to have notion of either user's config directory or user's home directory. Please use explicit '--config' argument"
        )
    };

    let config_path = config_path.canonicalize()?;
    // Read config file itself - if it exists
    let mut config = if config_path.is_file() {
        toml::from_str(&tokio::fs::read_to_string(&config_path).await?)?
    } else if config_path.exists() {
        anyhow::bail!(
            "Path '{}' exists yet points not to file",
            config_path.display()
        )
    } else {
        toml::Table::new()
    };
    // Fill in providers registry
    let mut registry = ProviderRegistry::new();

    registry.add_provider::<OpenWeather>("openweather");
    registry.add_provider::<WeatherApi>("weatherapi");
    // Execute CLI command
    match command {
        CliCmd::Configure {
            provider: _,
            options: _,
        } => (),
        CliCmd::Get {
            address: _,
            date: _,
            provider: _,
            set_default: _,
        } => (),
        CliCmd::Clear { providers } => {
            // Walk all mentioned providers and remove them
            for prov_name in &providers {
                // "all" means all providers
                if prov_name == "all" {
                    for name in registry.keys() {
                        config.remove(name.as_ref());
                    }
                }
                else if registry.contains_key(prov_name.as_str()) {
                    config.remove(prov_name);
                }
                else {
                    anyhow::bail!("No such provider: {prov_name}");
                }
            }
        },
    }

    let stub_config = toml::toml! {
        apikey = "banana"
    }
    .into();

    let prov = registry
        .get("weatherapi")
        .ok_or_else(|| anyhow::anyhow!("No such provider"))?
        .create(stub_config)?;

    let forecast = prov
        .read_weather(
            // Approx location of London
            51.5072,
            0.1275,
            date_now(),
        )
        .await?;

    println!("{forecast}");
    // Write config back to file
    if !config_path.is_file() {
        // NB: unwrap here is safe, since config path points either to existing file
        // or to some nonexistent location - so it cannot be just root path
        // whose parent would be `None`
        tokio::fs::create_dir_all(config_path.parent().unwrap()).await?;
    }

    tokio::fs::write(&config_path, toml::to_string_pretty(&config)?).await?;
    // End of processing
    Ok(())
}
