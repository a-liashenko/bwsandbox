use crate::bwrap::events::{Events, EventsReader, SandboxStatus};
use crate::fd::{AsFdArg, SharedPipe};
use crate::services::{BwrapInfo, Context, Scope, ScopeCleanup, Service};
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
}

impl BwrapProcBuilder {
    pub fn new(args: Vec<OsString>) -> Result<Self, AppError> {
        // Unshare bwrap so app can have full permissions to all bwrap created namespaces
        let mut command = Command::new(utils::BWRAP_CMD);

        // Bind working dir into sandbox, used if service need to create content in sandbox and mount it AFTER sandbox started
        command.arg("--bind");
        command.arg(utils::temp_dir());
        command.arg(utils::temp_dir());

        // Block bwrap until all services ready and operational
        let mut ready = SharedPipe::new()?;
        command.arg("--block-fd").arg_fd(ready.share_rx()?)?;

        // Read bwrap events
        let mut info = SharedPipe::new()?;
        command.arg("--json-status-fd").arg_fd(info.share_tx()?)?;

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

        let cleanup = ScopeCleanup::new(vec![services_scope])?;
        Ok(cleanup)
    }

    pub fn spawn(mut self, app: OsString, args: Vec<OsString>) -> Result<BwrapProc, AppError> {
        self.command.arg(app).args(args);
        tracing::info!("Bwrap command: {:?}", self.command);

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
        // Notify bwrap to start sandboxed app
        self.ready
            .write(&[1])
            .map_err(AppError::io("bwrap ready write"))?;
        let status = self.wait_exit_event()?;

        self.proc
            .wait()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?;
        Ok(status)
    }

    fn wait_exit_event(&mut self) -> Result<ExitStatus, AppError> {
        use std::io::ErrorKind;

        loop {
            let status = match self.reader.try_next::<Events>() {
                Ok(Events::Exit(status)) => status.exit_code,
                Err(AppError::Io(ctx, e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    tracing::warn!("Bwrap crashed? Context: {ctx}");
                    -1
                }
                Ok(e) => {
                    tracing::warn!("Unhandled bwrap event {e:?}");
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
                tracing::error!("Early BwrapProc drop? Killing child");
                self.proc.kill()
            }
            Err(e) => {
                tracing::error!("Unknown BwrapProc status: {e:?}");
                self.proc.kill()
            }
            _ => return,
        };
        tracing::info!("bwrap killed with {status:?}");
    }
}
