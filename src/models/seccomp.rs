use crate::seccomp_ffi::{Action, Arch, Syscall};
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, Deserialize)]
pub struct SeccompConfig {
    pub default_action: Action,
    pub extra_arch: Vec<Arch>,
    pub rules: Vec<SeccompRule>,
}

#[derive(Debug, Deserialize)]
pub struct SeccompRule {
    pub action: Action,
    pub syscalls: Vec<Syscall>,
}

impl<'de> Deserialize<'de> for Syscall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        let syscall = Syscall::from_str(&name).map_err(D::Error::custom)?;
        Ok(syscall)
    }
}

impl<'de> Deserialize<'de> for Arch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        let arch = Arch::from_str(&name).map_err(D::Error::custom)?;
        Ok(arch)
    }
}

#[test]
fn test_parse_seccomp_generic() {
    let content = include_str!("../../config/seccomp-generic.toml");
    let _config: SeccompConfig = toml::from_str(content).expect("Failed to parse example config");
    dbg!(&_config);
}
