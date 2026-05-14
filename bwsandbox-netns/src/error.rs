use rustix::io::Errno;
use rustix::thread::{CapabilitySet, CapabilitySets};
use std::borrow::Cow;
use std::ffi::OsString;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error {0}: {1:?}")]
    Io(Cow<'static, str>, std::io::Error),
    #[error("Unexpected args")]
    BadArgs,
    #[error("Failed to get capabilities: {0}")]
    CapsGet(Errno),
    #[error("Missing required capabilities: {0:?}")]
    CapsMissing(CapabilitySet),
    #[error("Failed to set capabilities: {0:?}")]
    CapsSet(CapabilitySets, Errno),
    #[error("Config file too big: {0} bytes")]
    ConfigTooBig(u64),
    #[error("Failed to get config meta: {0:?}")]
    ConfigMeta(std::io::Error),
    #[error("Config file have wrong permissions. Expected: root:root 700")]
    ConfigBadPermissions,
    #[error("Failed to parse config")]
    ConfigParse(toml::de::Error),
    #[error("User {0} not allowed to use it")]
    UserNotAllowed(rustix::process::Uid),
    #[error("Using {0:?} netns not allowed")]
    NetnsNotAllowed(OsString),
    #[error("Failed to enter netns: {0:?}")]
    NetnsEnter(rustix::io::Errno),
    #[error("Failed to unshare namespace: {0:?}")]
    UnshareNs(rustix::io::Errno),
    #[error("Mount error {0}: {1:?}")]
    Mount(&'static str, rustix::io::Errno),
}

impl AppError {
    pub fn caps_set(caps: CapabilitySets) -> impl Fn(Errno) -> AppError {
        move |err| AppError::CapsSet(caps, err)
    }

    pub fn io(src: &'static str) -> impl Fn(std::io::Error) -> Self {
        move |e| Self::Io(Cow::Borrowed(src), e)
    }

    pub fn io_owned(src: impl Into<String>) -> impl FnOnce(std::io::Error) -> Self {
        move |e| Self::Io(Cow::Owned(src.into()), e)
    }

    pub fn mount(src: &'static str) -> impl FnOnce(rustix::io::Errno) -> Self {
        move |e| Self::Mount(src, e)
    }
}
