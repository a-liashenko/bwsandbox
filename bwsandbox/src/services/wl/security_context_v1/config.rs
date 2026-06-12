use super::app_id::{AppIdResolver, BinResolver, StaticResolver};
use crate::config::{EnvVal, TempFileVal};
use crate::error::AppError;
use serde::Deserialize;
use std::ffi::{CString, OsStr};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum AppId {
    Static(StaticResolver),
    Bin(BinResolver),
}

impl AppId {
    pub fn resolve(&self, bin: &OsStr) -> Result<CString, AppError> {
        match self {
            AppId::Static(v) => v.resolve(bin),
            AppId::Bin(v) => v.resolve(bin),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mount: Option<EnvVal<PathBuf>>,
    pub sandbox_engine: String,
    #[serde(default = "app_id_default")]
    pub app_id: AppId,
    pub socket: TempFileVal,
}

fn app_id_default() -> AppId {
    AppId::Bin(BinResolver)
}
