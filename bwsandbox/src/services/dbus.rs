use crate::config::{Cmd, EnvVal, TempFileVal};
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service, ServiceCommand};
use crate::system::PollFile;
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

        Ok(Self {
            command,
            sandboxed_bus: cfg.sandboxed_bus.into_inner(),
            proxy_bus: cfg.proxy_bus.into_inner(),
        })
    }
}

impl<C: Context> Service<C> for DbusService {
    fn name(&self) -> &'static str {
        "xdg-dbus-proxy"
    }

    fn apply_before(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        ctx.command_mut()
            .arg("--symlink")
            .arg(&self.proxy_bus)
            .arg(&self.sandboxed_bus);
        Ok(Scope::new().remove_file(&self.proxy_bus))
    }

    fn start(mut self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        crate::print_command::print_command(&self.command);
        let child = self
            .command
            .stdin(Stdio::null())
            .spawn_service()
            .map_err(AppError::spawn(utils::DBUS_CMD))?;
        PollFile::watch(&self.proxy_bus)?.wait_exists(utils::READY_TIMEOUT)?;
        Ok(HandleType::new(child))
    }
}
