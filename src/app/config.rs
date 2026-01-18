use crate::{config::Cmd, services::ServicesConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bwrap: Cmd,
    #[serde(flatten)]
    pub services: ServicesConfig,
}
