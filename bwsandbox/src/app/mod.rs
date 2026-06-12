use crate::{bwrap::ProcBuilder, error::AppError, utils};
pub use args::Args;
use std::process::ExitStatus;

mod args;
mod config;
mod current_dir;

pub struct App;
impl App {
    pub fn start(args: Args) -> Result<ExitStatus, AppError> {
        let (mut services, bwrap_args) = current_dir::run_in_dir(&args.config_dir, move || {
            let config: config::Config = utils::deserialize(&args.config)?;
            let bwrap_args = config.bwrap.collect_args()?;
            let services = config.services.load()?;
            Ok((services, bwrap_args))
        })?;

        let mut bwrap_builder = ProcBuilder::new(args.app, bwrap_args);
        let _cleanup = bwrap_builder.apply_services(&mut services)?;

        let proc = bwrap_builder.spawn(args.app_args)?;
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
