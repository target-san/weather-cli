use std::{collections::BTreeMap, convert::Infallible, str::FromStr};

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
