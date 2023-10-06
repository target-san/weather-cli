use crate::{date_now, BoxFuture, CowString};
use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use toml::value::Date;

use super::{WeatherInfo, WeatherKind};

pub struct WeatherApi {
    apikey: String,
}

#[derive(Deserialize)]
struct Params {
    apikey: String,
}

#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorInner,
}

#[derive(Deserialize)]
struct ApiErrorInner {
    code: i32,
    message: String,
}

#[derive(Deserialize)]
struct RawResponse {
    forecast: Forecast,
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
        let date = date.unwrap_or_else(date_now);
        let url = format!(
            "https://api.weatherapi.com/v1/history.json?key={apikey}&q={location}&dt={}-{}-{}",
            date.year, date.month, date.day
        );
        let fut = async {
            let response = reqwest::get(url).await?;

            let is_ok = response.status().is_success();
            let code = response.status().as_u16();
            let text = response
                .text()
                .await
                .with_context(|| anyhow!("Could not obtain response text"))?;

            if !is_ok {
                let ApiError {
                    error: ApiErrorInner { code, message },
                } = serde_json::from_str(&text)
                    .with_context(|| anyhow!("Could not parse API error, HTTP code {code}"))?;
                bail!("API call error {code}: {message}");
            }

            let resp: RawResponse =
                serde_json::from_str(&text).with_context(|| anyhow!("Could not parse response"))?;

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
