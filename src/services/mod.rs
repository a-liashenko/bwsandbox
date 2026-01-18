mod types;
pub use types::{Context, Handle, Scope, ScopeCleanup, Service};

mod dbus;
mod env_mapper;
mod seccomp;
mod slirp4netns;

use crate::error::AppError;
use serde::de::DeserializeOwned;

type ServiceBuilder<C, S> = fn(C) -> Result<S, AppError>;
type EntryConfig<C> = Option<crate::config::Entry<C>>;
type BoxedService<C> = Box<dyn Service<C>>;

#[derive(Debug, serde::Deserialize)]
pub struct ServicesConfig {
    dbus: EntryConfig<dbus::Config>,
    env_mapper: EntryConfig<env_mapper::EnvMapper>,
    seccomp: EntryConfig<seccomp::Config>,
    slirp4netns: EntryConfig<slirp4netns::Config>,
}

impl ServicesConfig {
    pub fn load<C: Context>(self) -> Result<Vec<BoxedService<C>>, AppError> {
        let services = vec![
            Self::load_single(self.dbus, dbus::DbusService::from_config)?,
            Self::load_single(self.env_mapper, env_mapper::EnvMapper::from_config)?,
            Self::load_single(self.seccomp, seccomp::SeccompService::from_config)?,
            Self::load_single(self.slirp4netns, slirp4netns::Slirp4netns::from_config)?,
        ];

        let services = services.into_iter().flatten().collect();
        Ok(services)
    }

    fn load_single<Ctx: Context, C: DeserializeOwned, S: Service<Ctx> + 'static>(
        cfg: EntryConfig<C>,
        build: ServiceBuilder<C, S>,
    ) -> Result<Option<BoxedService<Ctx>>, AppError> {
        if let Some(entry) = cfg {
            let config = entry.load(crate::utils::deserialize)?;
            let service = build(config)?;
            let service = Box::new(service) as BoxedService<Ctx>;
            return Ok(Some(service));
        }
        Ok(None)
    }
}
