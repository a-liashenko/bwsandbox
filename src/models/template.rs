use super::env_value::EnvValue;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct TemplateConfig {
    #[serde(default = "bwrap_bin")]
    pub bin: String,
    pub name: String,
    pub include: PathBuf,
    pub context: BTreeMap<String, ContextValue>,
}

fn bwrap_bin() -> String {
    "bwrap".into()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ContextValue {
    Env { src: EnvValue<String> },
}
impl Serialize for ContextValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Env { src } => src.as_ref().serialize(serializer),
        }
    }
}

#[test]
fn test_parse_template_generic() {
    let content = include_str!("../../config/profile-games-rt.toml");
    let config: toml::Value = toml::from_str(&content).expect("Failed to parse example config");

    let template = config.get("template").unwrap().clone();
    let _config: TemplateConfig = template.try_into().unwrap();
    // println!("{:#?}", _config);
}
