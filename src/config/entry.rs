use serde::Deserialize;

use crate::config::EnvVal;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Entry<T> {
    Inline(T),
    Include { include: EnvVal<PathBuf> },
}
