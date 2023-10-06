use toml::value::Date;

use crate::{BoxFuture, CowString};

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
    fn get_weather(
        &self,
        location: CowString,
        date: Option<Date>,
    ) -> BoxFuture<anyhow::Result<String>>;
}
