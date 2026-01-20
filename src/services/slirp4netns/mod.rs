mod config;
mod ioctl;
mod namespace;
mod service;

pub use config::Config;
pub use service::Slirp4netns;

// Allow to supress Ns prefix warnings to keep it easier to add errors in the future
#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    NsOpen(rustix::io::Errno),
    #[error(transparent)]
    NsGetParent(rustix::io::Errno),
    #[error(transparent)]
    NsEnter(rustix::io::Errno),
    #[error(transparent)]
    NsIno(rustix::io::Errno),
}
