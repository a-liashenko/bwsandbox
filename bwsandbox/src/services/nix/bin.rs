use super::cmd;
use crate::error::AppError;
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

pub struct NixBin {
    bin: PathBuf,
}

impl NixBin {
    pub fn new(bin: &Path) -> Result<Self, AppError> {
        let bin = bin.canonicalize().map_err(AppError::file(&bin))?;
        Ok(Self { bin })
    }

    pub fn readlink(&self) -> &Path {
        &self.bin
    }

    pub fn list_deps(&self, with_ro: bool) -> Result<NixBinDeps, AppError> {
        let stdout = cmd::list_deps(&self.bin, with_ro)?;
        Ok(NixBinDeps { stdout })
    }

    pub fn is_nix(&self) -> bool {
        self.bin.starts_with("/nix")
    }
}

pub struct NixBinDeps {
    stdout: String,
}

impl NixBinDeps {
    pub fn iter(&self) -> NixBinDepsIter<'_> {
        NixBinDepsIter {
            lines: self.stdout.lines(),
        }
    }
}

pub struct NixBinDepsIter<'a> {
    lines: std::str::Lines<'a>,
}

impl<'a> Iterator for NixBinDepsIter<'a> {
    type Item = Result<&'a str, AppError>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.lines.next()?;
        if !next.starts_with("/nix") {
            let error = format!("Non nix path from nix-query {next:?}");
            let error = AppError::Io(error.into(), ErrorKind::Other.into());
            return Some(Err(error));
        }
        Some(Ok(next))
    }
}
