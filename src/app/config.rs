use crate::config::{Cmd, Entry};
use crate::error::AppError;
use crate::service::{Context, Handle, Scope, Service};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub bwrap: Cmd,
    #[serde(flatten)]
    pub services: ServiceConfig,
}

super::manager::define_services! {
    dbus => crate::services::DbusService,
    seccomp => crate::services::SeccompService,
    env_mapper => crate::services::EnvMapper
}

impl Service for ServiceType {
    type Config = ServiceConfig;
    type Handle = Box<dyn Handle>;

    fn from_config(_cfg: Self::Config) -> Result<Self, AppError> {
        unreachable!();
    }

    fn apply<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        self.apply(ctx)
    }

    fn start(self) -> Result<Self::Handle, AppError> {
        self.start()
    }
}
