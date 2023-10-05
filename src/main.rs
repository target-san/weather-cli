#![deny(warnings)]

use chrono::Datelike;
use clap::Parser;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use toml::value::Date;

use crate::provider::openweather::OpenWeather;
use crate::provider::weatherapi::WeatherApi;
use crate::selector::Selector;

mod provider;
mod selector;
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
    },
    /// Clear configuration of specified or all providers
    Clear {
        /// Names of providers whose configurations to clear; specify "all" to clear all providers
        providers: Vec<String>,
    },
}

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
    let config = if config_path.is_file() {
        toml::from_str(&tokio::fs::read_to_string(&config_path).await?)?
    } else if config_path.exists() {
        anyhow::bail!(
            "Path '{}' exists yet points not to file",
            config_path.display()
        )
    } else {
        toml::Table::new()
    };
    // Execute CLI command
    match command {
        CliCmd::Configure {
            provider: _,
            options: _,
        } => (),
        CliCmd::Get {
            address: _,
            date: _,
        } => (),
        CliCmd::Clear { providers: _ } => (),
    }

    let mut providers = Selector::new();

    providers.add_provider::<OpenWeather>("openweather");
    providers.add_provider::<WeatherApi>("weatherapi");

    let stub_config: toml::Value = toml::toml! {
        apikey = "banana"
    }
    .into();

    let prov = providers.create("weatherapi", stub_config)?;

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
