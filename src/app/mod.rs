use crate::{
    app::config::ServiceType,
    error::AppError,
    service::{Handle, Service},
    utils,
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
    pub fn try_parse(content: &str) -> Result<Self, AppError> {
        let config: Config = utils::deserialize(content)?;

        let sandbox = {
            let bin = config.bwrap.bin().unwrap_or(utils::BWRAP_CMD.as_ref());
            let args = config.bwrap.collect_args()?;
            Sandbox::new(bin, args)
        };
        let services = config.services.load_services()?;

        Ok(Self { sandbox, services })
    }

    pub fn apply_services(&mut self) -> Result<(), AppError> {
        for it in &mut self.services {
            self.sandbox.apply_before(it)?;
        }

        self.sandbox.prebuild();

        for it in &mut self.services {
            self.sandbox.apply_after(it)?;
        }

        Ok(())
    }

    pub fn run<A, I>(self, app: A, args: I) -> Result<ExitStatus, AppError>
    where
        A: AsRef<OsStr>,
        I: Iterator<Item = OsString>,
    {
        let (mut command, _scopes) = self.sandbox.build()?;
        let command = command.arg(app).args(args);
        tracing::info!("bwrap command: {command:?}");

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
