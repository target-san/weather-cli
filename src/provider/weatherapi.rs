use std::fmt::Display;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::config::Section;
use crate::utils::restful_get;
use crate::{BoxFuture, CowString};

use super::{Date, ParamDesc, ProviderInfo, WeatherInfo, WeatherKind};

/// WeatherAPI provider implementation
pub struct WeatherApi {
    apikey: String,
}

//
// Error handling structures
//

#[derive(Debug, Deserialize)]
struct ApiError {
    error: ApiErrorInner,
}

impl FromStr for ApiError {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "API error {}: {}",
            self.error.code, self.error.message
        ))
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Deserialize)]
struct ApiErrorInner {
    code: i32,
    message: String,
}

//
// Weather response structures
//

/// Weather response root
#[derive(Deserialize)]
struct WeatherData {
    forecast: Forecast,
}

impl FromStr for WeatherData {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Deserialize)]
struct Forecast {
    forecastday: Vec<ForecastDay>,
}

#[derive(Deserialize)]
struct ForecastDay {
    day: ForecastDayAvg,
}

#[derive(Deserialize)]
struct ForecastDayAvg {
    avghumidity: f32,
    avgtemp_c: f32,
    maxwind_kph: f32,
    condition: Condition,
}

#[derive(Deserialize)]
struct Condition {
    code: u32,
}

impl super::Provider for WeatherApi {
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
            description: "WeatherAPI (https://www.weatherapi.com/)",
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
        let date = date.unwrap_or_else(Date::today);
        let url = format!(
            "https://api.weatherapi.com/v1/history.json?key={apikey}&q={location}&dt={}-{}-{}",
            date.year, date.month, date.day
        );
        let fut = async {
            let resp = restful_get::<WeatherData, ApiError>(url)
                .await
                .with_context(|| anyhow!("Request to historical weather data failed"))?;

            let day = &resp
                .forecast
                .forecastday
                .first()
                .ok_or_else(|| anyhow!("Could not parse response: missing forecast day data"))?
                .day;
            // Use codes from https://www.weatherapi.com/docs/weather_conditions.json
            let weather = match day.condition.code {
                1000 => WeatherKind::Clear,
                1003 | 1006 | 1009 | 1087 => WeatherKind::Clouds,
                1030 | 1135 | 1147 => WeatherKind::Fog,
                1063 | 1072 | 1150 | 1153 | 1168 | 1171 | 1180 | 1183 | 1186 | 1189 | 1192
                | 1195 | 1198 | 1201 | 1240 | 1243 | 1246 | 1273 | 1276 => WeatherKind::Rain,
                1066 | 1069 | 1114 | 1117 | 1204 | 1207 | 1210 | 1213 | 1216 | 1219 | 1222
                | 1225 | 1237 | 1249 | 1252 | 1255 | 1258 | 1261 | 1264 | 1279 | 1282 => {
                    WeatherKind::Snow
                }
                _ => WeatherKind::Unknown,
            };

            Ok(WeatherInfo {
                weather,
                temperature: day.avgtemp_c,
                wind_speed: day.maxwind_kph,
                humidity: day.avghumidity,
            })
        };
        Box::pin(fut)
    }
}
