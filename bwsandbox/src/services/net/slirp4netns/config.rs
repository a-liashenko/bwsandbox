use crate::{config::Cmd, services::net::resolv_conf::ResolvConfVal};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_if_name")]
    pub if_name: String,
    #[serde(default)]
    pub resolv_conf: ResolvConfVal,
    #[serde(default = "default_quiet")]
    pub quiet: bool,
    #[serde(flatten)]
    pub cmd: Cmd,
}

fn default_if_name() -> String {
    "tap0".into()
}

fn default_quiet() -> bool {
    true
}
