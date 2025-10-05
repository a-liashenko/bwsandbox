use crate::{
    config::{Cmd, EnvVal, TempFileVal},
    error::AppError,
    service::{Context, Scope, Service},
    utils,
};
use serde::Deserialize;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
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

impl Service for DbusService {
    type Config = Config;
    type Handle = Handle;

    #[tracing::instrument]
    fn from_config(cfg: Self::Config) -> Result<Self, AppError> {
        let bin = cfg.cmd.bin().unwrap_or(utils::DBUS_CMD.as_ref());
        let inline_args = cfg.cmd.iter_inline();
        let template_args = cfg.cmd.iter_template()?;

        let mut command = Command::new(bin);
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

    #[tracing::instrument]
    fn apply<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        ctx.sandbox_mut()
            .arg("--bind")
            .arg(&self.proxy_bus)
            .arg(&self.sandboxed_bus);
        Ok(Scope::new().remove_file(&self.proxy_bus))
    }

    #[tracing::instrument]
    fn start(mut self) -> Result<Self::Handle, AppError> {
        const POLL: Duration = Duration::from_millis(100);
        const TOTAL_POLL: Duration = Duration::from_secs(3);

        let child = self
            .command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(AppError::spawn("dbus-proxy"))?;

        let exists = crate::utils::poll_file(&self.proxy_bus, POLL, TOTAL_POLL)?;
        if !exists {
            let err = std::io::ErrorKind::NotFound;
            return Err(AppError::file(&self.proxy_bus)(err.into()));
        }

        Ok(Handle { child })
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Handle {
    child: Child,
}

impl crate::service::Handle for Handle {
    #[tracing::instrument]
    fn stop(mut self) -> Result<(), AppError> {
        let _ = self.child.kill();
        Ok(())
    }
}
