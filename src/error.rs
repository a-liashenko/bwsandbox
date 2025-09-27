use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File {0}: {1:#?}")]
    File(PathBuf, std::io::Error),
    #[error("Toml deser {0}: {1:#?}")]
    TomlDeser(PathBuf, toml::de::Error),
    #[error("Spawn {0}: {1:#?}")]
    Spawn(&'static str, std::io::Error),
    #[error(transparent)]
    Template(#[from] handlebars::TemplateError),
    #[error(transparent)]
    TemplateRender(#[from] handlebars::RenderError),
    #[error("fcntl faile {0:?}")]
    Fcntl(#[from] nix::Error),
    #[error(transparent)]
    Args(#[from] lexopt::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    fn path_err<S, B, E>(source: &S, builder: B) -> impl Fn(E) -> Self
    where
        S: AsRef<Path>,
        B: Fn(PathBuf, E) -> Self,
        B: 'static,
    {
        move |e| {
            let source = source.as_ref().to_owned();
            builder(source, e)
        }
    }

    pub fn file(source: &impl AsRef<Path>) -> impl Fn(std::io::Error) -> Self {
        Self::path_err(source, Error::File)
    }

    pub fn parse(source: &impl AsRef<Path>) -> impl Fn(toml::de::Error) -> Self {
        Self::path_err(source, Error::TomlDeser)
    }

    pub fn spawn(source: &'static str) -> impl Fn(std::io::Error) -> Self {
        move |e| Self::Spawn(source, e)
    }
}
