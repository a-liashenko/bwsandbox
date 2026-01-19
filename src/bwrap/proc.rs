use crate::bwrap::events::{Events, EventsReader, SandboxStatus};
use crate::services::{Context, Scope, ScopeCleanup, Service};
use crate::{error::AppError, fd::AsFdExtra, utils};
use std::io::{PipeReader, PipeWriter, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ExitStatus, Stdio};
use std::{ffi::OsString, os::fd::IntoRawFd, process::Command};

#[derive(Debug)]
pub struct BwrapProcBuilder {
    args: Vec<OsString>,
    ready_tx: PipeWriter,
    info_rx: PipeReader,
    command: Command,
}

impl Context for BwrapProcBuilder {
    fn command_mut(&mut self) -> &mut std::process::Command {
        &mut self.command
    }
}

impl BwrapProcBuilder {
    pub fn new(args: Vec<OsString>) -> Result<Self, AppError> {
        let mut command = Command::new(utils::BWRAP_CMD);
        // Bind working dir into sandbox, used if service need to create content in sandbox and mount it AFTER sandbox started
        command.arg("--bind");
        command.arg(utils::temp_dir());
        command.arg(utils::temp_dir());

        // Block bwrap until all services ready and operational
        let (ready_rx, ready_tx) = std::io::pipe().map_err(AppError::PipeAlloc)?;
        ready_rx.share_with_children()?;
        command.arg("--block-fd");
        command.arg(ready_rx.into_raw_fd().to_string());

        // Read bwrap events
        let (info_rx, info_tx) = std::io::pipe().map_err(AppError::PipeAlloc)?;
        info_tx.share_with_children()?;
        command.arg("--json-status-fd");
        command.arg(info_tx.into_raw_fd().to_string());

        Ok(Self {
            args,
            ready_tx,
            info_rx,
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

        let proc = BwrapProc::new(proc, self.ready_tx, self.info_rx)?;
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
    fn new(proc: Child, ready_tx: PipeWriter, info_rx: PipeReader) -> Result<Self, AppError> {
        // Wait until bwrap fully initialized
        let mut reader = EventsReader::new(info_rx);
        let status = reader.try_next::<SandboxStatus>()?;

        Ok(Self {
            proc,
            status,
            reader,
            ready: ready_tx,
        })
    }

    pub fn app_status(&self) -> &SandboxStatus {
        &self.status
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
