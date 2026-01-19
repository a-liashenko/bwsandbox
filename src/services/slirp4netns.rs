use crate::bwrap::SandboxStatus;
use crate::fd::{AsFdArg, SharedPipe};
use crate::services::{Context, HandleOwned, Scope, Service};
use crate::{config::Cmd, error::AppError, utils};
use serde::Deserialize;
use std::io::Read;
use std::process::Stdio;
use std::{ffi::OsString, process::Command};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_if_name")]
    pub if_name: String,
    pub resolv_conf: Option<String>,
    #[serde(default = "default_quiet")]
    pub quiet: bool,
    #[serde(flatten)]
    pub cmd: Cmd,
}

fn default_if_name() -> String {
    "tap0".into()
}

fn default_quiet() -> bool {
    true
}

pub struct Slirp4netns {
    args: Vec<OsString>,
    if_name: String,
    quiet: bool,
    resolv_conf: Option<String>,
}

impl Slirp4netns {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let args = config.cmd.collect_args()?;
        Ok(Self {
            args,
            if_name: config.if_name,
            resolv_conf: config.resolv_conf,
            quiet: config.quiet,
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

    fn start(self: Box<Self>, status: &SandboxStatus) -> Result<HandleOwned, AppError> {
        use std::io::ErrorKind;

        let mut ready = SharedPipe::new()?;
        let mut command = Command::new(utils::SLIRP4NETNS_CMD);
        command
            .args(self.args)
            .arg("--ready-fd")
            .arg_fd(ready.share_tx()?)?
            .arg(status.child_pid.to_string())
            .arg(self.if_name);
        tracing::info!("Slirp4netns command: {:?}", command);

        if self.quiet {
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
        }

        // Start slirp4netns and wait until network is ready
        let child = command
            .spawn()
            .map_err(AppError::spawn(utils::SLIRP4NETNS_CMD))?;

        let mut buf = [0u8; 1];
        let mut ready_rx = ready.into_rx();
        let bytes = ready_rx.read(&mut buf).map_err(AppError::io(file!()))?;
        if bytes == 0 {
            AppError::io("slirp4netns ready read")(ErrorKind::UnexpectedEof.into()).into_err()
        } else {
            Ok(HandleOwned::new(child))
        }
    }
}
