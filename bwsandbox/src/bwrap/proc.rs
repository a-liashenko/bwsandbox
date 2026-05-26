use crate::bwrap::events::{Events, EventsReader, SandboxStatus};
use crate::bwrap::sigterm::SigTerm;
use crate::services::{BwrapInfo, Context, Scope, ScopeCleanup, Service};
use crate::system::{AsFdArg, SharedPipe};
use crate::{error::AppError, utils};
use std::io::{PipeReader, PipeWriter, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ExitStatus, Stdio};
use std::{ffi::OsString, process::Command};

#[derive(Debug)]
pub struct BwrapProcBuilder {
    args: Vec<OsString>,
    ready: SharedPipe,
    info: SharedPipe,
    command: Command,
}

impl Context for BwrapProcBuilder {
    fn command_mut(&mut self) -> &mut std::process::Command {
        &mut self.command
    }

    fn arg_exist_before(&self, arg: &str) -> bool {
        assert!(!self.args.is_empty(), "Valid only for apply_before");
        self.args.iter().any(|v| v == arg)
    }
}

impl BwrapProcBuilder {
    pub fn new(args: Vec<OsString>) -> Result<Self, AppError> {
        // Unshare bwrap so app can have full permissions to all bwrap created namespaces
        let mut command = Command::new(utils::BWRAP_CMD);

        // Bind working dir into sandbox, used if service need to create content in sandbox and mount it AFTER sandbox started
        command.arg("--bind");
        command.arg(utils::temp_dir());
        command.arg(utils::temp_dir());

        let ready = SharedPipe::new()?;
        let info = SharedPipe::new()?;

        Ok(Self {
            args,
            ready,
            info,
            command,
        })
    }

    pub fn apply_services(
        &mut self,
        services: &mut [Box<dyn Service<BwrapProcBuilder>>],
    ) -> Result<ScopeCleanup, AppError> {
        let mut services_scope = Scope::new();
        for it in services.iter_mut() {
            let scope = it.apply_before(self)?;
            services_scope.merge(scope);
        }
        self.command.args(std::mem::take(&mut self.args));
        for it in services.iter_mut() {
            let scope = it.apply_after(self)?;
            services_scope.merge(scope);
        }

        let cleanup = ScopeCleanup::new(vec![services_scope]);
        Ok(cleanup)
    }

    pub fn spawn(mut self, app: OsString, args: Vec<OsString>) -> Result<BwrapProc, AppError> {
        // Configure lifecycle tracking fds
        self.command
            .arg("--block-fd")
            .arg_fd(self.ready.share_rx()?)?;

        self.command
            .arg("--json-status-fd")
            .arg_fd(self.info.share_tx()?)?;

        self.command.arg(app).args(args);
        crate::print_command::print_command(&self.command);

        let proc = self
            .command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?;

        let proc = BwrapProc::new(proc, self.ready.into_tx(), self.info.into_rx())?;
        Ok(proc)
    }
}

#[derive(Debug)]
pub struct BwrapProc {
    proc: Child,
    status: SandboxStatus,
    reader: EventsReader<PipeReader>,
    ready: PipeWriter,
}

impl BwrapProc {
    fn new(proc: Child, ready: PipeWriter, info: PipeReader) -> Result<Self, AppError> {
        // Wait until bwrap fully initialized
        let mut reader = EventsReader::new(info);
        let status = reader.try_next::<SandboxStatus>()?;

        Ok(Self {
            proc,
            status,
            reader,
            ready,
        })
    }

    pub fn bwrap_info(&self) -> BwrapInfo {
        BwrapInfo {
            pid: self.proc.id(),
            sandbox: self.status,
        }
    }

    pub fn wait(mut self) -> Result<ExitStatus, AppError> {
        let sig = SigTerm::register()?;

        // Notify bwrap to start sandboxed app
        self.ready
            .write(&[1])
            .map_err(AppError::io("bwrap ready write"))?;
        let status = self.wait_exit_event(&sig)?;

        self.proc
            .wait()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?;
        Ok(status)
    }

    fn wait_exit_event(&mut self, sig: &SigTerm) -> Result<ExitStatus, AppError> {
        use std::io::ErrorKind;

        loop {
            let status = match self.reader.try_next::<Events>() {
                Ok(Events::Exit(status)) => status.exit_code,
                Err(AppError::Io(ctx, e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    if !sig.is_terminated() {
                        log::warn!("Bwrap crashed? Context: {ctx}");
                        return Err(AppError::io("Bwrap unexpected exit")(e));
                    }

                    log::info!("App was terminated");
                    linux_raw_sys::general::SIGINT
                        .try_into()
                        .expect("SIGINT u32 -> i32")
                }
                Ok(e) => {
                    log::warn!("Unhandled bwrap event {e:?}");
                    continue;
                }
                Err(e) => return Err(e),
            };

            return Ok(ExitStatus::from_raw(status));
        }
    }
}

impl Drop for BwrapProc {
    fn drop(&mut self) {
        let status = match self.proc.try_wait() {
            Ok(None) => {
                log::error!("Early BwrapProc drop? Killing child");
                self.proc.kill()
            }
            Err(e) => {
                log::error!("Unknown BwrapProc status: {e:?}");
                self.proc.kill()
            }
            _ => return,
        };
        log::info!("bwrap killed with {status:?}");

        let wait = self.proc.wait();
        log::info!("bwrap wait after kill {wait:?}");
    }
}
