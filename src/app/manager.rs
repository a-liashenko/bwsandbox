use crate::{config::Entry, error::AppError, service::Service};

pub fn load_service<S>(config: Entry<S::Config>) -> Result<S, AppError>
where
    S: Service,
{
    let config = config.load(crate::utils::deserialize)?;
    let service = S::from_config(config)?;
    Ok(service)
}

macro_rules! define_services {
    { $($name: ident => $service: ty),+ } => {
        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub struct ServiceConfig {
            $(
                $name: Option<Entry<<$service as Service>::Config>>,
            )+
        }

        impl ServiceConfig {
            pub fn load_services(self) -> Result<Vec<ServiceType>, AppError> {
                use crate::app::manager::load_service;

                let mut items = Vec::with_capacity(32);
                $(
                    if let Some(config) = self.$name {
                        tracing::trace!("Loading service {}", stringify!($name));
                        let service: $service = load_service(config)?;
                        items.push(ServiceType::$name(Box::new(service)));
                    }
                )+
                Ok(items)
            }
        }

        #[derive(Debug)]
        #[allow(non_camel_case_types)]
        pub enum ServiceType {
            $(
                $name(Box<$service>),
            )+
        }

        impl ServiceType {
            pub fn apply_before<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
                let scope = match self {
                    $(
                        Self::$name(v) => v.apply_before(ctx)?,
                    )+
                };
                Ok(scope)
            }

            pub fn apply_after<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
                let scope = match self {
                    $(
                        Self::$name(v) => v.apply_after(ctx)?,
                    )+
                };
                Ok(scope)
            }

            pub fn start(self) -> Result<Box<dyn Handle>, AppError> {
                let handle = match self {
                    $(
                        Self::$name(v) => v.start().map(|v| Box::new(v) as _ )?,
                    )+
                };
                Ok(handle)
            }
        }
    };
}

pub(super) use define_services;
