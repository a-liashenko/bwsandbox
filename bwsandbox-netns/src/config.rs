use crate::error::AppError;
use rustix::thread::{RawUid, Uid};
use serde::Deserialize;
use std::{ffi::OsStr, fs::Metadata, io::Read, os::unix::fs::MetadataExt, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    bwsandbox: Option<String>,
    allowed_uids: Vec<RawUid>,
    allowed_netns: Vec<String>,
}

impl Config {
    pub fn from_file(path: &str, max_size: u64) -> Result<Self, AppError> {
        let file = std::fs::File::open(path).map_err(AppError::io_owned(path))?;

        let meta = file.metadata().map_err(AppError::ConfigMeta)?;
        check_permissions(&meta, max_size)?;

        let max_size_usize = usize::try_from(max_size).expect("max_size will be truncated");
        let mut buff = String::with_capacity(max_size_usize);
        file.take(max_size)
            .read_to_string(&mut buff)
            .map_err(AppError::io_owned(path))?;

        let config = toml::from_str(&buff).map_err(AppError::ConfigParse)?;
        Ok(config)
    }

    pub fn check_user(&self, user: Uid) -> Result<(), AppError> {
        if !self.allowed_uids.contains(&user.as_raw()) {
            return Err(AppError::UserNotAllowed(user));
        }

        Ok(())
    }

    pub fn check_netns(&self, ns: &OsStr) -> Result<(), AppError> {
        // Should be zero-cost, because OsStr and String should be same for Linux
        if !self.allowed_netns.iter().any(|s| OsStr::new(s) == ns) {
            return Err(AppError::NetnsNotAllowed(ns.to_os_string()));
        }

        Ok(())
    }

    pub fn get_bwsandbox_bin(&self, default: &OsStr) -> Result<PathBuf, AppError> {
        let path = self.bwsandbox.as_deref().map_or(default, OsStr::new);
        let path = PathBuf::from(path)
            .canonicalize()
            .map_err(AppError::io("Failed to get full path for bwsandbox"))?;
        Ok(path)
    }
}

fn check_permissions(meta: &Metadata, max_size: u64) -> Result<(), AppError> {
    // Should be owned by root
    crate::trace::trace!("Config file owner {}:{}", meta.uid(), meta.gid());
    if meta.uid() != 0 || meta.gid() != 0 {
        return Err(AppError::ConfigBadPermissions);
    }

    // Should have acceptable size
    crate::trace::trace!("Config file size {}, max_size {}", meta.size(), max_size);
    if meta.size() >= max_size {
        return Err(AppError::ConfigTooBig(meta.size()));
    }

    // Only owner (root) should be able to edit it
    crate::trace::trace!("Config file permissions {:#o}", meta.mode());
    if meta.mode() & 0o022 != 0 {
        return Err(AppError::ConfigBadPermissions);
    }

    Ok(())
}
