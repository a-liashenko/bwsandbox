use super::ffi::{Action, Arch, Syscall};
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub default_action: Action,
    pub extra_arch: Vec<Arch>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
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
fn test_parse_seccomp() {
    let seccomp = toml::toml! {
        default_action = "SCMP_ACT_KILL"
        extra_arch = ["x86"]
        rules = [
            { action = "SCMP_ACT_ERRNO", syscalls = ["open", "close"] },
            { action = "SCMP_ACT_ALLOW", syscalls = ["open"] }
        ]
    };
    let v = toml::to_string_pretty(&seccomp).unwrap();
    let _v: Config = toml::from_str(&v).unwrap();
}
