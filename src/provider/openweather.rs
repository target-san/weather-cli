use crate::{BoxFuture, CowString};
use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use toml::value::Date;

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
    ) -> BoxFuture<anyhow::Result<String>> {
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

        let data_url = format!("https://api.openweathermap.org/data/2.5/weather?appid={apikey}");
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

            let value = serde_json::from_str::<serde_json::Value>(&text)
                .with_context(|| anyhow!("Could not parse response as JSON"))?;

            serde_json::to_string_pretty(&value)
                .with_context(|| anyhow!("Could not write JSON to string"))
        };
        Box::pin(fut)
    }
}
