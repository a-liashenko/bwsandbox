use crate::{bwrap::BwrapProcBuilder, error::AppError, utils};
pub use args::Args;
use std::env::{current_dir, set_current_dir};
use std::process::ExitStatus;

mod args;
mod config;

pub struct App;
impl App {
    pub fn start(args: Args) -> Result<ExitStatus, AppError> {
        let current_dir = current_dir().map_err(AppError::io("Failed to get current dir"))?;

        set_current_dir(args.config_dir).map_err(AppError::io("Failed to set current dir"))?;

        let config: config::Config = utils::deserialize(&args.config)?;

        let bwrap_args = config.bwrap.collect_args()?;
        let mut bwrap_builder = BwrapProcBuilder::new(bwrap_args)?;

        let mut services = config.services.load()?;
        let _cleanup = bwrap_builder.apply_services(&mut services)?;

        set_current_dir(current_dir).map_err(AppError::io("Failed to restore current dir"))?;

        let proc = bwrap_builder.spawn(args.app, args.app_args)?;
        let proc_status = proc.bwrap_info();
        let _handles = services
            .into_iter()
            .map(|v| {
                log::info!("Starting '{}' service", v.name());
                v.start(&proc_status)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let status = proc.wait()?;
        Ok(status)
    }
}
