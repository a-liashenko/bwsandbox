use super::config::Config;
use crate::fd::{AsFdArg, SharedPipe};
use crate::services::slirp4netns::namespace::Namespace;
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use crate::{error::AppError, utils};
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Slirp4netns {
    command: Command,
    ready: SharedPipe,

    resolv_conf: Option<PathBuf>,
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

        let mut ready = SharedPipe::new()?;
        command.arg("--ready-fd").arg_fd(ready.share_tx()?)?;

        let resolv_conf = match config.resolv_conf {
            Some(v) => {
                let file = utils::temp_dir().join("slirp4netns_resolv.conf");
                std::fs::write(&file, v).map_err(AppError::file(&file))?;
                Some(file)
            }
            None => None,
        };

        Ok(Self {
            command,
            ready,
            resolv_conf,
            if_name: config.if_name,
        })
    }

    fn fix_ns(&mut self, pid: u32) -> Result<(), AppError> {
        use std::io::ErrorKind;

        // If --dev used in bwrap it will create intermediate ns
        // And to avoid setns(CLONE_NEWNET): Operation not permitted we need enter intermediate ns
        // If --dev not used, parent ns will be our process ns
        let own_ns = Namespace::open_pid(std::process::id())?;
        let bw_pns = Namespace::open_pid(pid)?.parent()?;

        let dev_used = own_ns.fd_inode()? != bw_pns.fd_inode()?;
        if dev_used {
            self.command.arg("--userns-path=/proc/self/ns/user");
            unsafe {
                self.command.pre_exec(move || {
                    let result = bw_pns.enter();
                    tracing::trace!("pre_exec status {result:?}");
                    result.map_err(|_| ErrorKind::Other.into())
                })
            };
        }
        Ok(())
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
        if let Some(path) = &self.resolv_conf {
            ctx.command_mut()
                .arg("--ro-bind")
                .arg(path)
                .arg("/etc/resolv.conf");
            scope = scope.remove_file(path);
        }

        Ok(scope)
    }

    fn start(mut self: Box<Self>, status: &BwrapInfo) -> Result<HandleType, AppError> {
        use std::io::ErrorKind;

        self.command
            .arg(status.sandbox.child_pid.to_string())
            .arg(&self.if_name);
        tracing::info!("Slirp4netns command: {:?}", self.command);

        self.fix_ns(status.sandbox.child_pid)?;
        let child = self
            .command
            .spawn()
            .map_err(AppError::spawn(utils::SLIRP4NETNS_CMD))?;

        // Wait until ready
        let mut buf = [0u8; 1];
        let mut ready_rx = self.ready.into_rx();
        let bytes = ready_rx.read(&mut buf).map_err(AppError::io(file!()))?;
        if bytes == 0 {
            AppError::io("slirp4netns ready read")(ErrorKind::UnexpectedEof.into()).into_err()
        } else {
            Ok(HandleType::new(child))
        }
    }
}
