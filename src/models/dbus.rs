use serde::Deserialize;

use crate::models::env_value::EnvValue;

#[derive(Debug, Deserialize)]
pub struct DbusConfig {
    #[serde(default = "dbus_proxy_bin")]
    pub bin: String,
    #[serde(default = "dbus_address")]
    pub bus_address: EnvValue<String>,
    #[serde(default)]
    pub own: Vec<String>,
    #[serde(default)]
    pub talk: Vec<String>,
}

fn dbus_proxy_bin() -> String {
    "xdg-dbus-proxy".into()
}

fn dbus_address() -> EnvValue<String> {
    EnvValue::resolve("$DBUS_SESSION_BUS_ADDRESS".into()).unwrap_or_default()
}

#[test]
fn parse_dbus_generic() {
    let content = include_str!("../../config/dbus-generic.toml");
    let _config: DbusConfig = toml::from_str(&content).expect("Failed to parse example config");
    // dbg!(&_config);
}
