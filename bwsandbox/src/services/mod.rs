mod types;
pub use types::*;

mod appimage;
mod dbus;
mod env_mapper;
mod net;
mod seccomp;

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
    slirp4netns: EntryConfig<net::slirp4netns::Config>,
    appimage: EntryConfig<appimage::AppImageExtract>,
    pasta: EntryConfig<net::pasta::Config>,
}

impl ServicesConfig {
    pub fn load<C: Context>(self) -> Result<Vec<BoxedService<C>>, AppError> {
        log::info!("---- initializing services ----");
        let services = vec![
            Self::load_single(self.dbus, dbus::DbusService::from_config)?,
            Self::load_single(self.env_mapper, env_mapper::EnvMapper::from_config)?,
            Self::load_single(self.seccomp, seccomp::SeccompService::from_config)?,
            Self::load_single(self.slirp4netns, net::slirp4netns::Slirp4netns::from_config)?,
            Self::load_single(self.appimage, appimage::AppImageExtract::from_config)?,
            Self::load_single(self.pasta, net::pasta::Pasta::from_config)?,
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
            log::info!("'{}' initialized", service.name());
            let service = Box::new(service) as BoxedService<Ctx>;
            return Ok(Some(service));
        }
        Ok(None)
    }
}
