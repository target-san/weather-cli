use crate::BoxFuture;
use std::fmt::{self, Formatter};

pub mod openweather;
/// Defines any provider of weather data
///
/// NB: Futures can be unboxed when async traits arrive
pub trait Provider {
    /// Creates new instance of provider, using provided TOML config to configure it
    ///
    /// # Parameters
    /// * `config` - TOML data tree, should be parseable into internal config
    ///
    /// # Returns
    /// Boxed future which completes with provider instance or error
    fn new(config: toml::Value) -> anyhow::Result<Self>
    where
        Self: Sized;
    /// Prints useful info about current provider, usually its config options
    ///
    /// # Parameters
    /// * `f` - destination formatter
    ///
    /// # Returns
    /// Formatting result
    fn help(f: &mut Formatter<'_>) -> fmt::Result
    where
        Self: Sized;
    /// Fetches weather information asynchronously at specified location and date
    ///
    /// # Parameters
    /// * `lat` - lattitude, in range `-90..=90`
    /// * `lon` - longitude, in range `-180..=180`
    /// * `date` - day when weather forecast is needed;
    ///     limitations on future forecasting depend on concrete provider;
    ///     providers usually attempt to get weather at specified day's noon at specified location
    ///
    /// # Returns
    /// Boxed future which completes with forecast data or error
    fn read_weather(&self, lat: f64, lon: f64, time: u64) -> BoxFuture<anyhow::Result<String>>;
}
