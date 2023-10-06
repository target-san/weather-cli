use crate::{date_now, BoxFuture, CowString};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use toml::value::Date;

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
    ) -> BoxFuture<anyhow::Result<String>> {
        let apikey = &self.apikey;
        let date = date.unwrap_or_else(date_now);
        let url = format!(
            "http://api.weatherapi.com/v1/history.json?key={apikey}&q={location}&dt={}-{}-{}",
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

            if is_ok {
                Ok(serde_json::to_string_pretty(&serde_json::from_str::<
                    serde_json::Value,
                >(&text)?)?)
            } else {
                let ApiError {
                    error: ApiErrorInner { code, message },
                } = serde_json::from_str(&text)
                    .with_context(|| anyhow!("Could not parse API error, HTTP code {code}"))?;
                Err(anyhow!("API call error {code}: {message}"))
            }
        };
        Box::pin(fut)
    }
}
