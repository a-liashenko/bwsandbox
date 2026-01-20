use std::{
    borrow::Cow,
    os::fd::RawFd,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error {0}: {1:?}")]
    Io(Cow<'static, str>, std::io::Error),
    #[error("File {0:?}: {1:?}")]
    File(PathBuf, std::io::Error),
    #[error("Failed to use fcntl to share fd {0} with spawned apps, ec {1}")]
    FileFdShare(RawFd, rustix::io::Errno),
    #[error("Failed to alloc new tempfile, ec {0:?}")]
    FileTempAlloc(std::io::Error),
    #[error("TemDir {0:?}: {1:?}")]
    TempDir(PathBuf, std::io::Error),
    #[error("Env {0:?}: {1:?}")]
    Env(String, std::env::VarError),
    #[error("Failed to parse config {msg}", msg = .0.message())]
    Config(#[from] toml::de::Error),
    #[error("Template: {0:?}")]
    Template(#[from] minijinja::Error),
    #[error("Spawn {0:?}: {1:?}")]
    Spawn(String, std::io::Error),
    #[error("Unexpected or missing arguments")]
    BadArgs,
    #[error(transparent)]
    ArgParser(#[from] lexopt::Error),
    #[error("Failed ffi call to libseccomp {0:?}")]
    SeccompLib(anyhow::Error),
    #[error("Failed to register ctrl+c handle")]
    CtrlC(#[from] ctrlc::Error),
    #[error("Failed to allocate new pipe: {0:?}")]
    PipeAlloc(std::io::Error),
    #[error("Failed to parse bwrap event {0:?}")]
    BwrapEvent(serde_json::Error),
    #[error("Slir4netns failed with {0:?}")]
    Slir4netns(#[from] crate::services::slirp4netns::Error),
    //
    // #[error(transparent)]
    // Other(#[from] anyhow::Error),
}

impl AppError {
    pub fn into_err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    pub fn file(src: impl AsRef<Path>) -> impl Fn(std::io::Error) -> Self {
        move |e| Self::File(src.as_ref().into(), e)
    }

    pub fn env(src: impl AsRef<str>) -> impl Fn(std::env::VarError) -> Self {
        move |e| Self::Env(src.as_ref().into(), e)
    }

    pub fn spawn(src: impl AsRef<str>) -> impl Fn(std::io::Error) -> Self {
        move |e| Self::Spawn(src.as_ref().into(), e)
    }

    pub fn io(src: &'static str) -> impl Fn(std::io::Error) -> Self {
        move |e| Self::Io(Cow::Borrowed(src), e)
    }
}
