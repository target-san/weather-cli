use std::fmt::Display;

use toml::value::Date;

use crate::{BoxFuture, CowString};

pub mod openweather;
pub mod weatherapi;
/// Describes kind of weather - clear sky, clouds present or raining
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
            self.weather,
            self.temperature,
            self.wind_speed,
            self.humidity
        ))
    }
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
    ) -> BoxFuture<anyhow::Result<WeatherInfo>>;
}
