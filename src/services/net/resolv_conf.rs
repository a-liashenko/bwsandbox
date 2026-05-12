use crate::{error::AppError, services::Scope, utils};
use serde::Deserialize;
use std::{path::PathBuf, process::Command};

#[derive(Debug, Deserialize, Default)]
pub struct ResolvConfVal(Option<String>);
impl ResolvConfVal {
    pub fn generate(self) -> Result<ResolvConf, AppError> {
        let file = match self.0 {
            Some(v) => {
                let file = utils::temp_dir().join("pasta_resolv.conf");
                std::fs::write(&file, v).map_err(AppError::file(&file))?;
                Some(file)
            }
            None => None,
        };
        Ok(ResolvConf(file))
    }
}

#[derive(Debug)]
pub struct ResolvConf(Option<PathBuf>);
impl ResolvConf {
    pub fn mount(&self, command: &mut Command, scope: Scope) -> Scope {
        let path = match &self.0 {
            Some(v) => v,
            None => return scope,
        };
        command.arg("--ro-bind").arg(path).arg("/etc/resolv.conf");
        scope.remove_file(path)
    }
}
