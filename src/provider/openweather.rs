use crate::{BoxFuture, CowString};
use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use toml::value::Date;

use super::{WeatherInfo, WeatherKind};

pub struct OpenWeather {
    apikey: String,
}

#[derive(Deserialize)]
struct Params {
    apikey: String,
}

#[derive(Deserialize)]
struct ApiError {
    cod: i32,
    message: String,
}

#[derive(Deserialize)]
struct Coords {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct RawResponse {
    main: MainSection,
    wind: WindSection,
    weather: Vec<WeatherSection>,
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
    fn new(config: toml::Value) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let Params { apikey } = Params::deserialize(config)?;
        Ok(Self { apikey })
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

        let data_url = format!("https://api.openweathermap.org/data/2.5/weather?appid={apikey}&units=metric");
        let fut = async move {
            // Transform location into coordinates
            let response = reqwest::get(location_url)
                .await
                .with_context(|| anyhow!("Failed to retrieve location's coordinates"))?;

            let is_ok = response.status().is_success();
            let code = response.status().as_u16();

            let text = response
                .text()
                .await
                .with_context(|| anyhow!("Failed to retrieve location's coordinates"))?;

            if !is_ok {
                let ApiError { cod, message } = serde_json::from_str(&text)
                    .with_context(|| anyhow!("Could not parse response error, HTTP {code}"))?;
                bail!("API error {cod}: {message}");
            }

            let locs: Vec<Coords> = serde_json::from_str(&text).with_context(|| {
                anyhow!("Could not parse location response as array of coordinates")
            })?;

            let Coords { lat, lon } = locs.first().ok_or_else(|| {
                anyhow!("Could not resolve location '{location}' into coordinates")
            })?;
            // Perform actual weather request
            let data_url = format!("{data_url}&lat={lat:.4}&lon={lon:.4}");

            let response = reqwest::get(data_url)
                .await
                .with_context(|| anyhow!("Failed to retrieve weather forecast"))?;

            let is_ok = response.status().is_success();
            let code = response.status().as_u16();

            let text = response
                .text()
                .await
                .with_context(|| anyhow!("Failed to retrieve weather forecast"))?;

            if !is_ok {
                let ApiError { cod, message } = serde_json::from_str(&text)
                    .with_context(|| anyhow!("Could not parse response error, HTTP {code}"))?;
                bail!("API error {cod}: {message}");
            }

            let resp: RawResponse =
                serde_json::from_str(&text).with_context(|| {
                    eprintln!("{text}");
                    anyhow!("Could not parse response")
                })?;
            // Primitive weather resolver = fetch first entry, otherwise unknown
            let weather = if let Some(weather) = resp.weather.first() {
                // Use weather condition codes form https://openweathermap.org/weather-conditions
                match weather.id {
                    200..=299 | 300..=399 | 500..=599 => WeatherKind::Rain,
                    600..=699 => WeatherKind::Snow,
                    800 => WeatherKind::Clear,
                    700..=799 | 801..=809 => WeatherKind::Clouds,
                    _ => WeatherKind::Unknown,
                }
            }
            else {
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
