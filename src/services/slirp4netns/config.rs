use crate::config::Cmd;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_if_name")]
    pub if_name: String,
    pub resolv_conf: Option<String>,
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
