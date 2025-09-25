use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[repr(u32)]
pub enum Action {
    #[serde(rename = "SCMP_ACT_KILL")]
    Kill = super::SCMP_ACT_KILL,
    #[serde(rename = "SCMP_ACT_ERRNO")]
    Errno = super::SCMP_ACT_ERRNO,
    #[serde(rename = "SCMP_ACT_ALLOW")]
    Allow = super::SCMP_ACT_ALLOW,
}

impl Action {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

#[test]
fn test_deser() {
    #[derive(Debug, Deserialize)]
    struct Test {
        action: Action,
    }

    let value = "action = 'SCMP_ACT_KILL'";
    let parsed: Test = toml::from_str(value).unwrap();
    assert_eq!(parsed.action, Action::Kill);
    assert_eq!(parsed.action.as_u32(), crate::seccomp_ffi::SCMP_ACT_KILL);
}
