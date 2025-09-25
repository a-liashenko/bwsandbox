use std::path::PathBuf;

use super::{
    dbus::DbusConfig, env_value::EnvValue, seccomp::SeccompConfig, template::TemplateConfig,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProfileConfig {
    pub dbus: Option<Entry<DbusConfig>>,
    pub seccomp: Option<Entry<SeccompConfig>>,
    pub template: TemplateConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Entry<T> {
    Inline(T),
    File { include: EnvValue<PathBuf> },
}

#[test]
fn test_parse_games_rt() {
    let content = include_str!("../../config/profile-games-rt.toml");
    let _config: ProfileConfig = toml::from_str(&content).expect("Failed to parse example config");
    // dbg!(&_config);
}
