#![deny(warnings)]

use std::future::Future;
use std::pin::Pin;
use chrono::Datelike;
use toml::value::Date;

use crate::provider::openweather::OpenWeather;
use crate::provider::weatherapi::WeatherApi;
use crate::selector::Selector;

mod provider;
mod selector;
/// Used as shortcut alias for any boxed future
type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

fn date_now() -> Date {
    let date = chrono::Local::now().date_naive();
    Date { year: date.year() as u16, month: date.month() as u8, day: date.day() as u8 }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
            0.0,
            0.0,
            date_now(),
        )
        .await?;

    println!("{forecast}");

    Ok(())
}
