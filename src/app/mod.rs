use crate::{
    config::Entry,
    error::AppError,
    service::{Handle, Scope, Service},
    utils,
};
use std::{
    ffi::{OsStr, OsString},
    process::ExitStatus,
};

mod config;
mod sandbox;

use config::Config;
use sandbox::Sandbox;

#[derive(Debug)]
pub struct App<D, S> {
    sandbox: Sandbox,
    dbus: Option<D>,
    seccomp: Option<S>,
}

impl<D: Service, S: Service> App<D, S> {
    pub fn from_str(content: &str) -> Result<Self, AppError> {
        let config: Config<D::Config, S::Config> = toml::from_str(content)?;

        let sandbox = config.bwrap.into_command(utils::BWRAP_CMD)?;
        let seccomp = config.seccomp.map(service_load).transpose()?;
        let dbus = config.dbus.map(service_load).transpose()?;

        Ok(Self {
            sandbox: Sandbox::new(sandbox),
            seccomp,
            dbus,
        })
    }

    pub fn apply_services(&mut self) -> Result<(), AppError> {
        self.sandbox.apply_opt(self.seccomp.as_mut())?;
        self.sandbox.apply_opt(self.dbus.as_mut())?;
        Ok(())
    }

    pub fn run<A, I>(self, app: A, args: I) -> Result<ExitStatus, AppError>
    where
        A: AsRef<OsStr>,
        I: Iterator<Item = OsString>,
    {
        let seccomp = self.seccomp.map(Service::start).transpose()?;
        let dbus = self.dbus.map(Service::start).transpose()?;

        let (mut command, scope) = self.sandbox.into_parts();
        let command = command.arg(app).args(args);
        tracing::info!("bwrap command: {command:?}");

        let exit_status = command.spawn().map_err(AppError::spawn("bwrap"))?.wait();

        seccomp.map(S::Handle::stop);
        dbus.map(D::Handle::stop);
        destroy_scope(scope.into_iter());

        exit_status.map_err(AppError::spawn("bwrap"))
    }
}

#[tracing::instrument]
fn service_load<S: Service>(config: Entry<S::Config>) -> Result<S, AppError> {
    let config = match config {
        Entry::Inline(v) => v,
        Entry::Include { include } => {
            let path = include.as_inner();
            let content = std::fs::read_to_string(path).map_err(AppError::file(path))?;
            toml::from_str(&content)?
        }
    };

    let service = S::from_config(config)?;
    Ok(service)
}

#[tracing::instrument(skip_all)]
fn destroy_scope(iter: impl Iterator<Item = Scope>) {
    let files = iter.flat_map(|v| v.remove.into_iter());
    for file in files {
        if let Err(e) = std::fs::remove_file(&file) {
            tracing::warn!("Failed to remove {file:?}, err {e}");
        }
    }
}
