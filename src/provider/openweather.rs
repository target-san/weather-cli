use crate::{BoxFuture, CowString};
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

impl super::Provider for OpenWeather {
    fn new(config: toml::Value) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let Params { apikey } = Params::deserialize(config)?;
        Ok(Self { apikey })
    }

    fn help(_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }

    fn get_weather(&self, _location: CowString, date: Date) -> BoxFuture<anyhow::Result<String>> {
        let apikey = &self.apikey;
        // TODO: use geolocation service
        let lat = 42.0;
        let lon = 42.0;
        let url = format!(
            "https://api.openweathermap.org/data/3.0/onecall/timemachine?lat={lat}&lon={lon}&dt={}-{}-{}&appid={apikey}",
            date.year, date.month, date.day
        );
        let fut = async {
            let text = reqwest::get(url)
                .await?
                .text()
                .await
                .map_err::<anyhow::Error, _>(Into::into)?;

            if let Ok(ApiError { cod, message }) = serde_json::from_str(&text) {
                Err(anyhow::anyhow!("API call error {cod}: {message}"))
            } else {
                Ok(text)
            }
        };
        Box::pin(fut)
    }
}
