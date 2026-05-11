mod namespaces;
pub use namespaces::Namespace;

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
