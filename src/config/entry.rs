use crate::{config::EnvVal, error::AppError};
use serde::{Deserialize, de::DeserializeOwned};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Entry<T> {
    Inline(T),
    Include { include: EnvVal<PathBuf> },
}

impl<T: DeserializeOwned> Entry<T> {
    pub fn load<F, E>(self, loader: F) -> Result<T, AppError>
    where
        E: Into<AppError> + std::error::Error,
        F: Fn(&str) -> Result<T, E>,
    {
        let path = match self {
            Entry::Inline(v) => return Ok(v),
            Entry::Include { include } => include.into_inner(),
        };

        let content = std::fs::read_to_string(&path).map_err(AppError::file(&path))?;
        let item = loader(&content).map_err(Into::into)?;
        Ok(item)
    }
}
