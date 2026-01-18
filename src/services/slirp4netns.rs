use std::{ffi::OsString, process::Command};

use crate::services::{Context, Handle, Scope, Service};
use crate::{config::Cmd, error::AppError, utils};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub cmd: Cmd,
}

pub struct Slirp4netns {
    args: Vec<OsString>,
}

impl Slirp4netns {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let args = config.cmd.collect_args()?;
        Ok(Self { args })
    }
}

impl<C: Context> Service<C> for Slirp4netns {
    fn apply_before(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        // Only sandbox pid required, no extra args to bwrap
        Ok(Scope::new())
    }

    fn apply_after(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        // Only sandbox pid required, no extra args to bwrap
        Ok(Scope::new())
    }

    fn start(self: Box<Self>, pid: u32) -> Result<Box<dyn Handle>, AppError> {
        // TODO: Use slirp4netns --ready_fd and wait until network configured
        let mut command = Command::new(utils::SLIRP4NETNS_CMD);
        command.args(self.args).arg(pid.to_string());

        let child = command
            .spawn()
            .map_err(AppError::spawn(utils::SLIRP4NETNS_CMD))?;
        Ok(Box::new(child))
    }
}
