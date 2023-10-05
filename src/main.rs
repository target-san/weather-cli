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
    let _cli = Cli::parse();

    let mut providers = Selector::new();

    providers.add_provider::<OpenWeather>("openweather");
    providers.add_provider::<WeatherApi>("weatherapi");

    let config: toml::Value = toml::toml! {
        apikey = "banana"
    }
    .into();

    let prov = providers.create("weatherapi", config)?;

    let forecast = prov
        .read_weather(
            // Approx location of London
            51.5072,
            0.1275,
            date_now(),
        )
        .await?;

    println!("{forecast}");

    Ok(())
}
