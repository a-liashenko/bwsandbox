use super::resolv_conf::{ResolvConf, ResolvConfVal};
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
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
        })
    }
}

impl<C: Context> Service<C> for Pasta {
    fn apply_before(&mut self, _: &mut C) -> Result<Scope, AppError> {
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
        super::nsfix::fix(&mut self.command, info, "--userns=/proc/self/ns/user")?;
        self.command.arg(info.sandbox.child_pid.to_string());

        let child = self
            .command
            .spawn()
            .map_err(AppError::spawn(utils::PASTA_CMD))?;
        Ok(HandleType::new(child))
    }
}
