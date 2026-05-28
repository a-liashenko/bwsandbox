use super::config::Config;
use crate::services::net::{nsfix, resolv_conf::ResolvConf};
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service, ServiceCommand};
use crate::system::{AsFdArg, ReadExt, SharedPipe};
use crate::{error::AppError, utils};
use std::process::{Command, Stdio};

pub struct Slirp4netns {
    command: Command,
    ready: SharedPipe,
    with_dev: bool,

    resolv_conf: ResolvConf,
    if_name: String,
}

impl Slirp4netns {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let args = config.cmd.collect_args()?;
        let mut command = Command::new(utils::SLIRP4NETNS_CMD);
        command.args(args);

        if config.quiet {
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
        }

        let resolv_conf = config.resolv_conf.generate()?;
        let ready = SharedPipe::new()?;
        Ok(Self {
            command,
            ready,
            with_dev: false,
            resolv_conf,
            if_name: config.if_name,
        })
    }
}

impl<C: Context> Service<C> for Slirp4netns {
    fn name(&self) -> &'static str {
        "slirp4netns network"
    }

    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        self.with_dev = ctx.arg_exist_before("--dev");
        Ok(Scope::new())
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        // Probably net should be unshared in bwrap if user want to use slirp4netns
        ctx.command_mut().arg("--unshare-net");

        let scope = self.resolv_conf.mount(ctx.command_mut(), Scope::new());
        Ok(scope)
    }

    fn start(mut self: Box<Self>, info: &BwrapInfo) -> Result<HandleType, AppError> {
        self.command
            .arg("--ready-fd")
            .arg_fd(self.ready.share_tx()?)?
            .arg(info.sandbox.child_pid.to_string())
            .arg(&self.if_name);

        if self.with_dev {
            nsfix::pre_exec_enter_ns(&mut self.command, info)?;
            self.command.arg("--userns-path=/proc/self/ns/user");
        }

        crate::print_command::print_command(&self.command);
        let child = self
            .command
            .spawn_service()
            .map_err(AppError::spawn(utils::SLIRP4NETNS_CMD))?;

        let mut rx = self.ready.into_rx();
        match rx.try_read_buf_ext::<1>(utils::READY_TIMEOUT) {
            Ok(_) => Ok(HandleType::new(child)),
            Err(e) => Err(AppError::io("Failed to read slirp4netns ready")(e)),
        }
    }
}
