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
mod internal;
mod manager;
mod sandbox;
mod scope_destroyer;

use config::Config;
use sandbox::SandboxBuilder;

pub use internal::InternalApp;

#[derive(Debug)]
pub struct App {
    sandbox: SandboxBuilder,
    services: Vec<ServiceType>,
}

impl App {
    pub fn try_parse(content: &str) -> Result<Self, AppError> {
        let config: Config = utils::deserialize(content)?;

        let args = config.bwrap.collect_args()?;
        let sandbox = SandboxBuilder::new(utils::SELF_CMD, args)?;

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
        let mut sandbox = self.sandbox.build(app, args)?;
        tracing::info!("internal command: {:?}", sandbox.get_command());

        let mut internal = sandbox.start()?;

        // Start all background services. Service::start should block until its ready
        let mut handles = services_start(self.services.into_iter(), internal.id())?;
        sandbox.notify_ready()?;

        let exit_status = internal.wait();
        if let Err(e) = handles.iter_mut().try_for_each(Handle::stop) {
            tracing::error!("Failed to stop service with {e:?}");
        }

        exit_status.map_err(AppError::spawn(utils::SELF_CMD))
    }
}

fn services_start<S, I>(iter: I, pid: u32) -> Result<Vec<S::Handle>, AppError>
where
    S: Service,
    I: Iterator<Item = S>,
{
    iter.map(|v| v.start(pid)).collect::<Result<Vec<_>, _>>()
}
