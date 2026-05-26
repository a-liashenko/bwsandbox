use crate::{error::AppError, services::Scope, utils};
use serde::Deserialize;
use std::{path::PathBuf, process::Command};

#[derive(Debug, Deserialize, Default)]
pub struct ResolvConfVal(Option<String>);
impl ResolvConfVal {
    pub fn generate(self) -> Result<ResolvConf, AppError> {
        let file = match self.0 {
            Some(v) => {
                let file = utils::temp_dir().join("resolv.conf");
                log::trace!("Create {}", file.display());
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
    pub fn mount(&self, command: &mut Command, mut scope: Scope) -> Scope {
        if let Some(path) = &self.0 {
            command.arg("--ro-bind").arg(path).arg("/etc/resolv.conf");
            scope = scope.remove_file(path);
        }
        scope
    }
}
