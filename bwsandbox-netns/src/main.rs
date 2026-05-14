mod capabilities;
mod config;
mod error;
mod netns;
mod trace;

use crate::error::AppError;
use rustix::thread::CapabilitySet;
use std::{ffi::OsStr, os::unix::process::CommandExt, process::Command};

const TRACE_ENABLED: &str = "BWSANDBOX_TRACE";
const CAPS_REQUIRED: CapabilitySet = CapabilitySet::SYS_ADMIN;
const CONFIG_PATH: &str = "/etc/bwsandbox/netns.toml";
const CONFIG_MAX_SIZE: u64 = 1024 * 1024; // 1 mb
const BWSANDBOX_BIN: &str = "/usr/bin/bwsandbox";

fn run() -> Result<(), AppError> {
    // Enable trace if needed
    if std::env::var(TRACE_ENABLED).is_ok() {
        trace::set_trace_enabled(true);
    }

    // Check args
    let mut args = std::env::args_os();
    if args.len() < 4 {
        trace::error!("Usage: bwsandbox-netns <netns> <bwsandbox args ...>");
        return Err(AppError::BadArgs);
    }
    let _app_name = args.next().unwrap();

    // Create and validate netns
    let netns = args.next().ok_or(AppError::BadArgs)?;
    let netns = netns::Netns::new(netns)?;

    // Parse global config
    let config = config::Config::from_file(CONFIG_PATH, CONFIG_MAX_SIZE)?;

    // Check if current user and netns allowed
    let user = rustix::process::getuid();
    config.check_user(user)?;
    config.check_netns(netns.get_name())?;

    // Setup capabilities
    let capabilites = capabilities::Capabilities::new(CAPS_REQUIRED)?;
    {
        let _guard = capabilites.elevate()?;
        netns.enter()?;
        netns.mount_resolv_conf()?;
    }

    let cmd = config.get_bwsandbox_bin(OsStr::new(BWSANDBOX_BIN))?;
    let err = Command::new(cmd).args(args).exec();
    Err(AppError::io("Failed to launch bwsandbox")(err))
}

fn main() -> Result<(), AppError> {
    if let Err(e) = run() {
        trace::error!("{e:?}: {e}");
        return Err(e);
    }

    Ok(())
}
