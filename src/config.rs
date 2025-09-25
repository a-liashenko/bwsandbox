use crate::models::{
    dbus::DbusConfig,
    profile::{Entry, ProfileConfig},
    seccomp::SeccompConfig,
    template::TemplateConfig,
};
use serde::{Deserialize, de::DeserializeOwned};
use std::path::Path;

use crate::error::Error;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub dbus: Option<DbusConfig>,
    pub seccomp: Option<SeccompConfig>,
    pub template: TemplateConfig,
}

impl AppConfig {
    pub fn load(config: impl AsRef<Path>) -> Result<Self, Error> {
        let profile: ProfileConfig = load_toml(&config)?;

        if !profile.template.include.is_dir() {
            let ec = std::io::Error::new(std::io::ErrorKind::NotADirectory, "Must be directory");
            return Err(Error::File(profile.template.include, ec));
        }

        let seccomp = profile.seccomp.map(load_entry).transpose()?;
        let dbus = profile.dbus.map(load_entry).transpose()?;

        Ok(Self {
            dbus,
            seccomp,
            template: profile.template,
        })
    }
}

// Utils
fn load_toml<O: DeserializeOwned>(source: &impl AsRef<Path>) -> Result<O, Error> {
    let content = std::fs::read_to_string(source.as_ref()).map_err(Error::file(source))?;
    let toml: O = toml::from_str(&content).map_err(Error::parse(source))?;
    Ok(toml)
}

fn load_entry<O: DeserializeOwned>(entry: Entry<O>) -> Result<O, Error> {
    match entry {
        Entry::Inline(v) => Ok(v),
        Entry::File { include } => load_toml(&include.as_ref()),
    }
}

#[test]
fn test_load_app_config() {
    let config = AppConfig::load("./config/profile-games-rt.toml").unwrap();
    assert!(config.seccomp.is_some());
    assert!(config.dbus.is_some());
    // dbg!(config);
}
