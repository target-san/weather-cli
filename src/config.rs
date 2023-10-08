use std::{
    collections::BTreeMap,
    convert::Infallible,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, bail, Context};
use light_ini::{IniHandler, IniParser};
/// Representation of INI file section
/// BTreeMap is used to preserve nice alphabetic order of keys
pub type Section = BTreeMap<String, String>;

/// Application configuration structure
#[derive(Default)]
pub struct Config {
    pub globals: Section,
    pub sections: BTreeMap<String, Section>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FromStr for Config {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut visitor = IniVisitor::new();
        let mut parser = IniParser::with_start_comment(&mut visitor, '#');
        parser.parse(s.as_bytes())?;
        Ok(visitor.build())
    }
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let mut buf = String::new();

        let mut write_section = |name: Option<&str>, section: &Section| {
            if section.is_empty() {
                return;
            }

            if let Some(name) = name {
                buf.push_str(&format!("[{name}]\n"));
            }

            for (name, value) in section {
                buf.push_str(&format!("{name} = {value}\n"));
            }
            buf.push('\n');
        };

        write_section(None, &self.globals);

        for (name, section) in &self.sections {
            write_section(Some(name.as_str()), section);
        }

        buf
    }
}

type HandleSection = Vec<(String, String)>;

#[derive(Default)]
struct IniVisitor {
    globals: HandleSection,
    sections: Vec<(String, HandleSection)>,
    current: (Option<String>, HandleSection),
}

impl IniVisitor {
    fn new() -> Self {
        Self::default()
    }

    fn build(mut self) -> Config {
        self.flush_current();
        Config {
            globals: self.globals.into_iter().collect(),
            sections: self
                .sections
                .into_iter()
                .map(|(name, items)| (name, items.into_iter().collect()))
                .collect(),
        }
    }

    fn flush_current(&mut self) {
        let (current_name, current_items) = std::mem::take(&mut self.current);
        if current_items.is_empty() {
            return;
        }

        if let Some(name) = current_name {
            self.sections.push((name, current_items));
        } else {
            self.globals = current_items;
        }
    }
}

impl IniHandler for IniVisitor {
    type Error = Infallible;

    fn section(&mut self, name: &str) -> Result<(), Self::Error> {
        self.flush_current();
        self.current.0 = Some(name.to_string());

        Ok(())
    }

    fn option(&mut self, key: &str, value: &str) -> Result<(), Self::Error> {
        self.current.1.push((key.to_string(), value.to_string()));
        Ok(())
    }
}

/// Read app's configuration at specified path; if path isn't provided, default config path is used
///
/// # Parameters
/// * `path` - optional config path
///
/// # Returns
/// Parsed configuration as TOML table and path to it
pub fn read_from_file(path: Option<PathBuf>) -> anyhow::Result<(Config, PathBuf)> {
    // Fetch path to config file
    let config_path = if let Some(path) = path {
        path
    } else if let Some(path) = dirs::config_dir() {
        path.join("weather-cli").join("config.ini")
    } else if let Some(path) = dirs::home_dir() {
        path.join(".weather-cli.ini")
    } else {
        bail!(
            "Current OS doesn't seem to have notion of either user's config directory or user's home directory. Please use explicit '--config' argument"
        )
    };

    // Read config file itself - if it exists
    let config = if config_path.is_file() {
        let contents = fs::read_to_string(&config_path)
            .with_context(|| anyhow!("When reading config file '{}'", config_path.display()))?;
        Config::from_str(&contents)
            .with_context(|| anyhow!("When parsing config file '{}'", config_path.display()))?
    } else if config_path.exists() {
        bail!(
            "Path '{}' exists yet points not to file",
            config_path.display()
        )
    } else {
        Config::new()
    };

    Ok((config, config_path))
}
/// Writes app's configuration at specified path
///
/// # Parameters
/// * `config` - configuration object
/// * `path` - path where to write configuration
pub fn write_to_file(config: &Config, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let config_path = path.as_ref();
    // Write config back to file
    if !config_path.is_file() {
        let config_dir_path = config_path.parent().ok_or_else(|| {
            anyhow!(
                "Config file path is somehow incorrect, as its parent directory cannot be obtained"
            )
        })?;
        fs::create_dir_all(config_dir_path).with_context(|| {
            anyhow!(
                "When creating config directory {}",
                config_dir_path.display()
            )
        })?;
    }

    fs::write(&config_path, config.to_string())
        .with_context(|| anyhow!("When writing configuration to {}", config_path.display()))
}
