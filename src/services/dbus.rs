use crate::config::{Cmd, EnvVal, TempFileVal};
use crate::services::{Context, HandleOwned, Scope, Service};
use crate::{error::AppError, utils};
use serde::Deserialize;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
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
        let inline_args = cfg.cmd.iter_inline();
        let template_args = cfg.cmd.iter_template()?;

        let mut command = Command::new(utils::DBUS_CMD);
        command
            .arg(cfg.user_bus.as_inner())
            .arg(cfg.proxy_bus.as_inner())
            .args(inline_args)
            .args(template_args.iter());
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
    fn start(mut self: Box<Self>, _pid: u32) -> Result<HandleOwned, AppError> {
        const POLL: Duration = Duration::from_millis(100);
        const TOTAL_POLL: Duration = Duration::from_secs(3);

        let child = self
            .command
            .stdin(Stdio::null())
            .spawn()
            .map_err(AppError::spawn(utils::DBUS_CMD))?;

        let exists = crate::utils::poll_file(&self.proxy_bus, POLL, TOTAL_POLL)?;
        if !exists {
            let err = std::io::ErrorKind::NotFound;
            return Err(AppError::file(&self.proxy_bus)(err.into()));
        }

        Ok(HandleOwned::new(child))
    }
}
