use std::fmt::{Formatter, self};
use std::future::Future;

pub trait Provider {
    /// Creates new instance of provider, using provided TOML config to configure it
    /// 
    /// # Parameters
    /// * `config` - TOML data tree, should be parseable into internal config
    fn new(config: toml::Value) -> Box<dyn Future<Output = anyhow::Result<Self>>> where Self: Sized;
    fn help(f: &mut Formatter<'_>) -> fmt::Result;
    
    fn read_weather(&self) -> Box<dyn Future<Output = anyhow::Result<toml::Value>>>;
}
