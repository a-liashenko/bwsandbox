mod fd;
mod namespaces;
mod pidfd;
mod poll;
mod poll_file;
mod shared_pipe;

pub use fd::{AsFdArg, AsFdExtra, ReadExt};
pub use namespaces::{Namespace, NamespaceType};
pub use pidfd::PidFd;
pub use poll_file::PollFile;
pub use shared_pipe::SharedPipe;

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
    #[error(transparent)]
    PidFdOpen(rustix::io::Errno),
    #[error(transparent)]
    PidFdSig(rustix::io::Errno),
    #[error(transparent)]
    InotInit(rustix::io::Errno),
    #[error(transparent)]
    InotWatch(rustix::io::Errno),
    #[error(transparent)]
    InotRead(rustix::io::Errno),
}
