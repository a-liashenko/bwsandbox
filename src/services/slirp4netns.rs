use crate::services::{Context, HandleOwned, Scope, Service};
use crate::{config::Cmd, error::AppError, utils};
use serde::Deserialize;
use std::process::Stdio;
use std::{ffi::OsString, process::Command};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_if_name")]
    pub if_name: String,
    pub resolv_conf: Option<String>,
    #[serde(flatten)]
    pub cmd: Cmd,
}

fn default_if_name() -> String {
    "tap0".into()
}

pub struct Slirp4netns {
    args: Vec<OsString>,
    if_name: String,
    resolv_conf: Option<String>,
}

impl Slirp4netns {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let args = config.cmd.collect_args()?;
        Ok(Self {
            args,
            if_name: config.if_name,
            resolv_conf: config.resolv_conf,
        })
    }
}

impl<C: Context> Service<C> for Slirp4netns {
    fn apply_before(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        // Probably net should be unshared in bwrap if user want to use slirp4netns
        ctx.command_mut().arg("--unshare-net");

        let mut scope = Scope::new();
        if let Some(resolv_conf) = &self.resolv_conf {
            // If user provide cusom resolv_conf (f.e. using systemd-resolved and can't access host loopback)
            // Create own resolv.conf and override bwrap commands
            let resolve_path = utils::temp_dir().join("resolv.conf");
            std::fs::write(&resolve_path, resolv_conf).map_err(AppError::file(&resolve_path))?;
            ctx.command_mut()
                .arg("--ro-bind")
                .arg(&resolve_path)
                .arg("/etc/resolv.conf");
            scope = scope.remove_file(resolve_path);
        }

        Ok(scope)
    }

    fn start(self: Box<Self>, pid: u32) -> Result<HandleOwned, AppError> {
        // TODO: Use slirp4netns --ready_fd and wait until network configured
        let mut command = Command::new(utils::SLIRP4NETNS_CMD);
        command.args(self.args).arg(pid.to_string());
        command.arg(self.if_name);

        tracing::trace!("Slirp4netns command: {:?}", command);

        let child = command
            .stdout(Stdio::null())
            .spawn()
            .map_err(AppError::spawn(utils::SLIRP4NETNS_CMD))?;
        Ok(HandleOwned::new(child))
    }
}
