use toml::value::Date;

use crate::{BoxFuture, CowString};
use std::fmt::{self, Formatter};

pub mod openweather;
pub mod weatherapi;

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
    /// Provider instance or error
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
    /// Fetches weather information asynchronously at specified location and UNIX timestamp
    ///
    /// # Parameters
    /// * `location` - name of location for which forecast is required;
    ///     provider would usually use some geolocation service
    /// * `date` - day when weather forecast is needed;
    ///     limitations on future forecasting depend on concrete provider
    ///
    /// # Returns
    /// Boxed future which completes with forecast data or error
    fn get_weather(&self, location: CowString, date: Date) -> BoxFuture<anyhow::Result<String>>;
}
