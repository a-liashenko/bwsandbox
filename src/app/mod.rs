use crate::{bwrap::BwrapProcBuilder, error::AppError, services::Handle, utils};
pub use args::Args;
use std::process::ExitStatus;

mod args;
mod config;

// App responsibility:
// Parse, load and validate bwrap and services configuration
// Start and forward bwrap and sandboxed app arguments into child instance
// Configure and run all services
// Signal child instance that everyting ready and wait until child process finished
// Cleanup resources registered in Scope by services
// Graceful shutdown of services (if implemented) and scoped resources cleanup

pub struct App;
impl App {
    pub fn start(args: Args) -> Result<ExitStatus, AppError> {
        let config: config::Config = utils::deserialize(&args.config)?;

        let bwrap_args = config.bwrap.collect_args()?;
        let mut bwrap_builder = BwrapProcBuilder::new(bwrap_args)?;

        let mut services = config.services.load()?;
        let _cleanup = bwrap_builder.apply_services(&mut services)?;

        let proc = bwrap_builder.spawn(args.app, args.app_args)?;
        let _handles = services
            .into_iter()
            .map(|v| v.start(proc.pid()).map(ServiceHandle::new))
            .collect::<Result<Vec<ServiceHandle>, _>>()?;

        let status = proc.wait()?;
        Ok(status)
    }
}

#[derive(Debug)]
pub struct ServiceHandle {
    handle: Box<dyn Handle>,
}

impl ServiceHandle {
    pub fn new(handle: Box<dyn Handle>) -> Self {
        Self { handle }
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if let Err(e) = self.handle.stop() {
            tracing::error!("Failed to stop service: {e:?}")
        }
    }
}
