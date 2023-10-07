use std::error::Error as StdError;
use std::str::FromStr;

use anyhow::{anyhow, Context};
use reqwest::IntoUrl;

/// Execute provided closure and return its result
///
/// Small helper function simply for the purpose of gathering multiple `?` errors
/// and applying some transformation to them once.
/// One such example is adding context via `anyhow::Context::with_context`
#[allow(unused)]
pub fn try_block<R, E>(block: impl FnOnce() -> Result<R, E>) -> Result<R, E> {
    block()
}
/// Execute provided closure and return its result
///
/// Small helper function simply for the purpose of gathering multiple `?` errors
/// and applying some transformation to them once.
/// One such example is adding context via `anyhow::Context::with_context`
///
/// Differs from `try_block` only in fixing error type to `anyhow::Error`
#[allow(unused)]
pub fn try_block_anyhow<R>(block: impl FnOnce() -> anyhow::Result<R>) -> anyhow::Result<R> {
    block()
}
/// Perform HTTP GET request to REST API endpoint, handle its success or failure
/// and parse result, either successful or failing, from text
///
/// Please note that despite error type is specified, failure is returned as `anyhow::Error`.
/// This is because there are many types of errors besides API error itself which may arise.
///
/// # Generics
/// * `R` - successful result type, should be parseable from response text
/// * `E` - failure type, should be parseable from response text
///
/// # Parameters
/// * `url` - request URL
///
/// # Returns
/// Successful result or failure
pub async fn restful_get<R, E>(url: impl IntoUrl) -> anyhow::Result<R>
where
    R: FromStr,
    R::Err: StdError + Send + Sync + 'static,
    E: FromStr + StdError + Send + Sync + 'static,
    E::Err: StdError + Send + Sync + 'static,
{
    let response = reqwest::get(url)
        .await
        .with_context(|| anyhow!("HTTP GET request failed"))?;

    let is_ok = response.status().is_success();
    let code = response.status().as_u16();

    let text = response
        .text()
        .await
        .with_context(|| anyhow!("Could not obtain response text"))?;

    if is_ok {
        Ok(R::from_str(&text)
            .with_context(|| anyhow!("Could not parse response as successful result"))?)
    } else {
        Err(E::from_str(&text)
            .with_context(|| anyhow!("Could not parse response as failure (HTTP {code})"))?
            .into())
    }
}
