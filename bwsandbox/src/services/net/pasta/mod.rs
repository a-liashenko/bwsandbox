use super::resolv_conf::{ResolvConf, ResolvConfVal};
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use crate::{config::Cmd, error::AppError, utils};
use serde::Deserialize;
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub resolv_conf: ResolvConfVal,
    #[serde(default = "default_quiet")]
    pub quiet: bool,
    #[serde(flatten)]
    pub cmd: Cmd,
}

fn default_quiet() -> bool {
    true
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
        if config.quiet {
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
        }

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
        let pasta_pid = utils::temp_dir().join("pasta.pid");
        self.command.arg("--pid").arg(&pasta_pid);

        let arg = if self.with_dev {
            super::nsfix::pre_exec_enter_ns(&mut self.command, info)?;
            format!("--netns=/proc/{}/ns/net", info.sandbox.child_pid)
        } else {
            info.sandbox.child_pid.to_string()
        };
        self.command.arg(arg);

        log::info!("CMD: {:?}", self.command);
        let child = self
            .command
            .spawn()
            .map_err(AppError::spawn(utils::PASTA_CMD))?;

        let exists = utils::poll_file(&pasta_pid, utils::READY_POLL, utils::READY_TIMEOUT)?;
        if !exists {
            let err = std::io::ErrorKind::NotFound;
            return Err(AppError::file(&pasta_pid)(err.into()));
        }

        Ok(HandleType::new(child))
    }
}
