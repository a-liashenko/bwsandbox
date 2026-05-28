mod fd;
mod file_poll;
mod namespaces;

pub use fd::{AsFdArg, AsFdExtra, ReadExt, SharedPipe};
pub use file_poll::PollFile;
pub use namespaces::{Namespace, NamespaceType};

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
