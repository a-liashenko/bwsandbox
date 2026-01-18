use std::{ffi::OsString, process::Command};

use crate::config::ArgVal;
use crate::services::{Context, HandleOwned, Scope, Service};
use crate::{config::Cmd, error::AppError, utils};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub cmd: Cmd,
    #[serde(default = "default_if_name")]
    pub if_name: ArgVal,
}

fn default_if_name() -> ArgVal {
    ArgVal::Str {
        value: "tap0".into(),
    }
}

pub struct Slirp4netns {
    args: Vec<OsString>,
    if_name: String,
}

impl Slirp4netns {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let args = config.cmd.collect_args()?;
        let if_name = config.if_name.to_str().to_string();
        Ok(Self { args, if_name })
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

    fn start(self: Box<Self>, pid: u32) -> Result<HandleOwned, AppError> {
        // TODO: Use slirp4netns --ready_fd and wait until network configured
        let mut command = Command::new(utils::SLIRP4NETNS_CMD);
        command.args(self.args).arg(pid.to_string());
        command.arg(self.if_name);

        tracing::trace!("Slirp4netns command: {:?}", command);

        let child = command
            .spawn()
            .map_err(AppError::spawn(utils::SLIRP4NETNS_CMD))?;
        Ok(HandleOwned::new(child))
    }
}
