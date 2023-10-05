use serde::Deserialize;
use crate::BoxFuture;

pub struct OpenWeather {
    apikey: String,
}

#[derive(Deserialize)]
struct Params {
    apikey: String
}

#[derive(Deserialize)]
struct ApiError {
    cod: i32,
    message: String,
}

impl super::Provider for OpenWeather {
    fn new(config: toml::Value) -> anyhow::Result<Self> where Self: Sized {
        let Params { apikey } = Params::deserialize(config)?;
        Ok(Self { apikey })
    }

    fn help(_f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }

    fn read_weather(&self, lat: f64, lon: f64, time: u64) -> BoxFuture<anyhow::Result<String>> {
        let apikey = &self.apikey;
        let url = format!(
            "https://api.openweathermap.org/data/3.0/onecall/timemachine?lat={lat}&lon={lon}&dt={time}&appid={apikey}"
        );
        let fut = async {
            let text = reqwest::get(url)
                .await?
                .text()
                .await
                .map_err::<anyhow::Error, _>(Into::into)?;

            if let Ok(ApiError { cod, message }) = serde_json::from_str(&text) {
                Err(anyhow::anyhow!("API call error {cod}: {message}"))
            }
            else {
                Ok(text)
            }
        };
        Box::pin(fut)
    }
}
