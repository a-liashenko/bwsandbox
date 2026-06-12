use crate::error::AppError;
use rustix::{
    mount::MountPropagationFlags,
    thread::{LinkNameSpaceType, UnshareFlags},
};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    os::fd::AsFd,
    path::PathBuf,
};

const RESOLV_CONF: &str = "/etc/resolv.conf";

#[derive(Debug)]
pub struct Netns {
    name: OsString,
}

impl Netns {
    pub fn new(name: OsString) -> Result<Self, AppError> {
        let netns = name.to_str().ok_or(AppError::BadArgs)?;
        let validate_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if netns.chars().all(validate_char) {
            Ok(Self { name })
        } else {
            Err(AppError::BadArgs)
        }
    }

    pub fn get_name(&self) -> &OsStr {
        &self.name
    }

    pub fn enter(&self) -> Result<(), AppError> {
        let path = PathBuf::from("/var/run/netns").join(&self.name);
        let fd = File::open(path).map_err(AppError::io("failed to open netns"))?;
        rustix::thread::move_into_link_name_space(fd.as_fd(), Some(LinkNameSpaceType::Network))
            .map_err(AppError::NetnsEnter)?;

        Ok(())
    }

    // Part of ip netns exec functionality to mount custom resolv.conf
    pub fn mount_resolv_conf(&self) -> Result<(), AppError> {
        let path = PathBuf::from("/etc/netns")
            .join(&self.name)
            .join("resolv.conf");

        // Skip if not exists
        if !path.exists() || path.is_symlink() {
            return Ok(());
        }

        // Safety: https://docs.rs/rustix/latest/rustix/thread/fn.unshare_unsafe.html
        // Not mentioned any safety requirements for NEWNS
        // We don't have any shared resources like fds and etc too
        unsafe {
            rustix::thread::unshare_unsafe(UnshareFlags::NEWNS).map_err(AppError::UnshareNs)?;
        }

        // Isolate host / mount table
        rustix::mount::mount_change(
            "/",
            MountPropagationFlags::PRIVATE | MountPropagationFlags::REC,
        )
        .map_err(AppError::mount("Failed to isolate host /etc/resolv.conf"))?;

        // Now bind new resolv.conf
        rustix::mount::mount_bind(path, RESOLV_CONF)
            .map_err(AppError::mount("Failed to bind netns resolv.conf"))?;

        Ok(())
    }
}
