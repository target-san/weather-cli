use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::config::Section;
use crate::utils::restful_get;
use crate::{BoxFuture, CowString};

use super::{Date, ParamDesc, ProviderInfo, WeatherInfo, WeatherKind};
// Convert km/h to m/s
const KM_H_M_S: f32 = 1.0 / 3.6;

pub struct AccuWeather {
    apikey: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiError {
    code: String,
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
        f.write_fmt(format_args!("API error '{}': {}", self.code, self.message))
    }
}

impl std::error::Error for ApiError {}

struct LocationData(Vec<Location>);

impl FromStr for LocationData {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_json::from_str(s)?))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Location {
    key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WeatherData(Vec<Condition>);

impl FromStr for WeatherData {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_json::from_str(s)?))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Condition {
    temperature: ValueEntry,
    relative_humidity: f32,
    wind: Wind,
    cloud_cover: f32,
    precipitation_type: Option<PrecipitationType>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
enum PrecipitationType {
    Rain,
    Snow,
    Ice,
    Mixed,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ValueEntry {
    metric: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Value {
    value: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Wind {
    speed: ValueEntry,
}

impl super::Provider for AccuWeather {
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
            description: "AccuWeather (https://www.accuweather.com/)",
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
            "https://dataservice.accuweather.com/locations/v1/cities/search?apikey={apikey}&q={location}"
        );
        let data_url_head = "http://dataservice.accuweather.com/currentconditions/v1/".to_string();
        let data_url_tail = format!("?apikey={apikey}&details=true");
        let fut = async move {
            // Convert location lookup to location key
            let locations = restful_get::<LocationData, ApiError>(location_url)
                .await
                .with_context(|| anyhow!("Could not obtain location key for {location}"))?
                .0;

            let location_key = locations
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Could not obtain location key for {location}"))?
                .key;

            let data_url = format!("{data_url_head}{location_key}{data_url_tail}");

            let data = restful_get::<WeatherData, ApiError>(data_url)
                .await
                .with_context(|| anyhow!("Could not obtain forecast data"))?;

            let condition = data
                .0
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("No current condition entries"))?;

            let temperature = condition.temperature.metric.value;
            let wind_speed = condition.wind.speed.metric.value * KM_H_M_S;
            let humidity = condition.relative_humidity;

            let weather = match condition.precipitation_type {
                Some(precip) => match precip {
                    PrecipitationType::Snow | PrecipitationType::Ice | PrecipitationType::Mixed => {
                        WeatherKind::Snow
                    }
                    PrecipitationType::Rain => WeatherKind::Rain,
                },
                None => {
                    if condition.cloud_cover > 5.0 {
                        WeatherKind::Clouds
                    } else {
                        WeatherKind::Clear
                    }
                }
            };

            Ok(WeatherInfo {
                weather,
                temperature,
                wind_speed,
                humidity,
            })
        };
        Box::pin(fut)
    }
}
