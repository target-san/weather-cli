use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::config::Section;
use crate::utils::restful_get;
use crate::{BoxFuture, CowString};

use super::{Date, ParamDesc, ProviderInfo, WeatherInfo, WeatherKind};

/// OpenWeather provider
pub struct OpenWeather {
    apikey: String,
}

//
// Error handling structures
//

#[derive(Debug, Deserialize)]
struct ApiError {
    cod: i32,
    message: String,
}

impl FromStr for ApiError {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("API error {}: {}", self.cod, self.message))
    }
}

impl std::error::Error for ApiError {}

//
// Location response structures
//

/// Location response root
struct CoordsVec(Vec<Coords>);

impl FromStr for CoordsVec {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_json::from_str(s)?))
    }
}

#[derive(Deserialize)]
struct Coords {
    lat: f64,
    lon: f64,
}

//
// Weather response structures
//

/// Weather response root
#[derive(Deserialize)]
struct WeatherData {
    main: MainSection,
    wind: WindSection,
    weather: Vec<WeatherSection>,
}

impl FromStr for WeatherData {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Deserialize)]
struct MainSection {
    temp: f32,
    humidity: f32,
}

#[derive(Deserialize)]
struct WindSection {
    speed: f32,
}

#[derive(Deserialize)]
struct WeatherSection {
    id: u32,
}

impl super::Provider for OpenWeather {
    fn new(config: &Section) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            apikey: config
                .get("apikey")
                .ok_or_else(|| anyhow!("Missing parameter 'apikey'"))?
                .clone(),
        })
    }

    fn info() -> &'static ProviderInfo
    where
        Self: Sized,
    {
        const INFO: ProviderInfo = ProviderInfo {
            description: "OpenWeather (https://openweathermap.org/); doesn't support specific dates, only current conditions",
            params: &[ParamDesc {
                id: "apikey",
                name: "User's API key",
                description: "used to authenticate user requests",
            }],
        };
        &INFO
    }

    fn get_weather(
        &self,
        location: CowString,
        date: Option<Date>,
    ) -> BoxFuture<anyhow::Result<WeatherInfo>> {
        let apikey = &self.apikey;
        if date.is_some() {
            return Box::pin(async {
                Err(anyhow!(
                    "Sorry, requesting weather for specific date isn't supported"
                ))
            });
        }
        let location_url = format!(
            "https://api.openweathermap.org/geo/1.0/direct?q={location}&limit=1&appid={apikey}"
        );

        let data_url =
            format!("https://api.openweathermap.org/data/2.5/weather?appid={apikey}&units=metric");
        let fut = async move {
            // Transform location into coordinates
            let locs = restful_get::<CoordsVec, ApiError>(location_url)
                .await
                .with_context(|| anyhow!("Could not obtain location's coordinates"))?
                .0;

            let Coords { lat, lon } = locs
                .first()
                .ok_or_else(|| anyhow!("Could not obtain coordinates of location '{location}'"))?;
            // Perform actual weather request
            let data_url = format!("{data_url}&lat={lat:.4}&lon={lon:.4}");

            let resp = restful_get::<WeatherData, ApiError>(data_url)
                .await
                .with_context(|| anyhow!("Could not obtain weather forecast"))?;

            // Primitive weather resolver = fetch first entry, otherwise unknown
            let weather = if let Some(weather) = resp.weather.first() {
                // Use weather condition codes form https://openweathermap.org/weather-conditions
                match weather.id {
                    200..=299 | 300..=399 | 500..=599 => WeatherKind::Rain,
                    600..=699 => WeatherKind::Snow,
                    800 => WeatherKind::Clear,
                    801..=809 => WeatherKind::Clouds,
                    700..=799 => WeatherKind::Fog,
                    _ => WeatherKind::Unknown,
                }
            } else {
                WeatherKind::Unknown
            };

            Ok(WeatherInfo {
                weather,
                temperature: resp.main.temp,
                wind_speed: resp.wind.speed,
                humidity: resp.main.humidity,
            })
        };
        Box::pin(fut)
    }
}
