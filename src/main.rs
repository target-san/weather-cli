#![deny(warnings)]

use std::pin::Pin;
use std::future::Future;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::provider::openweather::OpenWeather;
use crate::selector::Selector;

mod provider;
mod selector;
/// Used as shortcut alias for any boxed future
type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut providers = Selector::new();

    providers.add_provider::<OpenWeather>("openweather");
    let config: toml::Value = toml::toml! {
        apikey = "matumba"
    }.into();

    let prov = providers.create("openweather", config)?;

    let forecast = prov.read_weather(0.0, 0.0, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()).await?;

    println!("{forecast}");

    Ok(())
}
