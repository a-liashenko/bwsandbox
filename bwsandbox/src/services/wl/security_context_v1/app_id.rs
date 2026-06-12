use crate::error::AppError;
use serde::Deserialize;
use std::ffi::{CString, OsStr};

pub trait AppIdResolver {
    fn resolve(&self, bin: &OsStr) -> Result<CString, AppError>;
}

#[derive(Debug, Deserialize)]
pub struct StaticResolver {
    name: String,
}

impl AppIdResolver for StaticResolver {
    fn resolve(&self, _: &OsStr) -> Result<CString, AppError> {
        let name = CString::new(self.name.clone())?;
        Ok(name)
    }
}

#[derive(Debug, Deserialize)]
pub struct BinResolver;

impl AppIdResolver for BinResolver {
    fn resolve(&self, bin: &OsStr) -> Result<CString, AppError> {
        let bin = bin.as_encoded_bytes().to_vec();
        let name = CString::new(bin)?;
        Ok(name)
    }
}
