use std::fmt::Display;

use crate::{config::Section, date::Date, BoxFuture, CowString};

pub mod accuweather;
pub mod openweather;
pub mod weatherapi;
/// Describes kind of weather - clear sky, clouds, raining etc.
#[derive(Debug)]
pub enum WeatherKind {
    Unknown,
    Clear,
    Clouds,
    Fog,
    Rain,
    Snow,
}

impl Display for WeatherKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            WeatherKind::Unknown => "unknown",
            WeatherKind::Clear => "clear",
            WeatherKind::Clouds => "clouds",
            WeatherKind::Fog => "fog",
            WeatherKind::Rain => "raining",
            WeatherKind::Snow => "snow",
        };
        f.write_str(desc)
    }
}
/// Weather information
#[derive(Debug)]
pub struct WeatherInfo {
    /// What kind of weather
    pub weather: WeatherKind,
    /// Temperature, in Celsius degrees
    pub temperature: f32,
    /// Wind speed, in m/s
    pub wind_speed: f32,
    /// Humidity, in percents, 0..=100
    pub humidity: f32,
}

impl Display for WeatherInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Weather: {}\nTemperature: {}°C\nWind speed: {} m/s\nHumidity: {}%",
            self.weather, self.temperature, self.wind_speed, self.humidity
        ))
    }
}
/// Additional information about provider, used to show extended help or validate
/// config parameters
pub struct ProviderInfo {
    /// Detailed description, used when listing providers
    pub description: &'static str,
    /// Parameters this provider requires as its configuration
    pub params: &'static [ParamDesc],
}
/// Parameter description
pub struct ParamDesc {
    /// Parameter identifier, used to specify parameter when creating provider
    pub id: &'static str,
    /// User-friendly name, used to request parameter in interactive mode
    pub name: &'static str,
    /// Parameter description, used when listing providers
    pub description: &'static str,
}
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
    fn new(config: &Section) -> anyhow::Result<Self>
    where
        Self: Sized;
    /// Get additional information about provider
    ///
    /// # Returns
    /// Provider information
    fn info() -> &'static ProviderInfo
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
    ) -> BoxFuture<anyhow::Result<WeatherInfo>>;
}
