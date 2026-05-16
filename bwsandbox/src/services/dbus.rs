use crate::config::{Cmd, EnvVal, TempFileVal};
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use crate::{error::AppError, utils};
use serde::Deserialize;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub user_bus: EnvVal<PathBuf>,
    pub sandboxed_bus: EnvVal<PathBuf>,
    pub proxy_bus: TempFileVal,
    #[serde(flatten)]
    pub cmd: Cmd,
}

#[derive(Debug)]
pub struct DbusService {
    sandboxed_bus: PathBuf,
    proxy_bus: PathBuf,
    command: Command,
}

impl DbusService {
    pub fn from_config(cfg: Config) -> Result<Self, AppError> {
        let args = cfg.cmd.collect_args()?;

        let mut command = Command::new(utils::DBUS_CMD);
        command
            .arg(cfg.user_bus.as_inner())
            .arg(cfg.proxy_bus.as_inner())
            .args(args);
        tracing::info!("dbus proxy command {command:?}");

        Ok(Self {
            command,
            sandboxed_bus: cfg.sandboxed_bus.into_inner(),
            proxy_bus: cfg.proxy_bus.into_inner(),
        })
    }
}

impl<C: Context> Service<C> for DbusService {
    fn apply_before(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    #[tracing::instrument]
    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        ctx.command_mut()
            .arg("--symlink")
            .arg(&self.proxy_bus)
            .arg(&self.sandboxed_bus);
        Ok(Scope::new().remove_file(&self.proxy_bus))
    }

    #[tracing::instrument]
    fn start(mut self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        let child = self
            .command
            .stdin(Stdio::null())
            .spawn()
            .map_err(AppError::spawn(utils::DBUS_CMD))?;

        let exists = utils::poll_file(&self.proxy_bus, utils::READY_POLL, utils::READY_TIMEOUT)?;
        if !exists {
            let err = std::io::ErrorKind::NotFound;
            return Err(AppError::file(&self.proxy_bus)(err.into()));
        }

        Ok(HandleType::new(child))
    }
}
