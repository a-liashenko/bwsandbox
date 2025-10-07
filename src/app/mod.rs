use crate::{
    app::{config::ServiceType, scope_destroyer::ScopeDestroyer},
    error::AppError,
    service::{Handle, Service},
};
use std::{
    ffi::{OsStr, OsString},
    process::ExitStatus,
};

mod config;
mod manager;
mod sandbox;
mod scope_destroyer;

use config::Config;
use sandbox::Sandbox;

#[derive(Debug)]
pub struct App {
    sandbox: Sandbox,
    services: Vec<ServiceType>,
}

impl App {
    pub fn from_str(content: &str) -> Result<Self, AppError> {
        let config: Config = crate::utils::deserialize(content)?;

        let services = config.services.load_services()?;
        let sandbox = config.bwrap.into_command(crate::utils::BWRAP_CMD)?;
        Ok(Self {
            services,
            sandbox: Sandbox::new(sandbox),
        })
    }

    pub fn apply_services(&mut self) -> Result<(), AppError> {
        for it in &mut self.services {
            self.sandbox.apply(it)?;
        }
        Ok(())
    }

    pub fn run<A, I>(self, app: A, args: I) -> Result<ExitStatus, AppError>
    where
        A: AsRef<OsStr>,
        I: Iterator<Item = OsString>,
    {
        let (mut command, scope) = self.sandbox.into_parts();
        let command = command.arg(app).args(args);
        tracing::info!("bwrap command: {command:?}");

        let _scopes = ScopeDestroyer::new(scope)?;

        let mut handles = services_start(self.services.into_iter())?;
        let exit_status = command.spawn().map_err(AppError::spawn("bwrap"))?.wait();
        if let Err(e) = handles.iter_mut().try_for_each(Handle::stop) {
            tracing::error!("Failed to stop service with {e:?}");
        }

        exit_status.map_err(AppError::spawn("bwrap"))
    }
}

fn services_start<S, I>(iter: I) -> Result<Vec<S::Handle>, AppError>
where
    S: Service,
    I: Iterator<Item = S>,
{
    iter.map(Service::start).collect::<Result<Vec<_>, _>>()
}
