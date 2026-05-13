use super::resolv_conf::{ResolvConf, ResolvConfVal};
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use crate::system::{AsFdArg, ReadExt, SharedPipe};
use crate::{config::Cmd, error::AppError, utils};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub resolv_conf: ResolvConfVal,
    #[serde(flatten)]
    pub cmd: Cmd,
}

#[derive(Debug)]
pub struct Pasta {
    command: Command,
    resolv_conf: ResolvConf,
    with_dev: bool,
}

impl Pasta {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let args = config.cmd.collect_args()?;
        let mut command = Command::new(utils::PASTA_CMD);

        // Make pasta process foreground for easy tracking from parent process
        command.arg("--foreground");
        command.args(args);

        let resolv_conf = config.resolv_conf.generate()?;
        Ok(Self {
            command,
            resolv_conf,
            with_dev: false,
        })
    }
}

impl<C: Context> Service<C> for Pasta {
    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        // TODO: Find better solution to avoid datarace, iterating ALL args can be pretty slow and error prone
        self.with_dev = ctx.arg_exist_before("--dev");
        Ok(Scope::new())
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        // Probably net should be unshared in bwrap if user want to use slirp4netns
        ctx.command_mut().arg("--unshare-net");

        // Mount resolv conf
        let scope = self.resolv_conf.mount(ctx.command_mut(), Scope::new());
        Ok(scope)
    }

    fn start(mut self: Box<Self>, info: &BwrapInfo) -> Result<HandleType, AppError> {
        // Pasta strace:
        // close_range(3, 4294967295, CLOSE_RANGE_UNSHARE) = 0
        // In result, pipe tx end will be closed in pasta and lead to open error
        // This why fd should be alive on parent side

        let mut ready = SharedPipe::new()?;
        self.command.arg("--pid").arg_fd_path(ready.share_tx()?);

        let arg = if self.with_dev {
            super::nsfix::pre_exec_enter_ns(&mut self.command, info)?;
            format!("--netns=/proc/{}/ns/net", info.sandbox.child_pid)
        } else {
            info.sandbox.child_pid.to_string()
        };
        self.command.arg(arg);

        tracing::trace!("pasta cmd: {:?}", self.command);
        let child = self
            .command
            .spawn()
            .map_err(AppError::spawn(utils::PASTA_CMD))?;

        let (mut rx, _tx) = ready.into_parts();
        match rx.try_read_ext::<1>(std::time::Duration::from_secs(1)) {
            Ok(_) => Ok(HandleType::new(child)),
            Err(e) => Err(AppError::io("Failed to read pasta ready")(e)),
        }
    }
}
