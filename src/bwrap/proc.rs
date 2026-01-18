use crate::services::{Context, Scope, ScopeCleanup, Service};
use crate::{error::AppError, fd::AsFdExtra, utils};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ExitStatus, Stdio};
use std::{
    ffi::OsString,
    fs::File,
    os::fd::{FromRawFd, IntoRawFd},
    process::Command,
};

type BoxedService = Box<dyn Service<BwrapProcBuilder>>;

#[derive(Debug)]
pub struct BwrapProcBuilder {
    args: Vec<OsString>,
    ready_tx: File,
    info_rx: File,
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
        command.arg("--bind");
        command.arg(utils::temp_dir());
        command.arg(utils::temp_dir());

        // Create pipe to signal bwrap that parent initialized all services
        let (ready_rx, ready_tx) = rustix::pipe::pipe().map_err(AppError::PipeAlloc)?;
        ready_rx.share_with_children()?;
        command.arg("--block-fd");
        command.arg(ready_rx.into_raw_fd().to_string());

        // Create pipe to read spawned child pid
        let (info_rx, info_tx) = rustix::pipe::pipe().map_err(AppError::PipeAlloc)?;
        info_tx.share_with_children()?;
        command.arg("--info-fd");
        command.arg(info_tx.into_raw_fd().to_string());

        let ready_tx = unsafe { File::from_raw_fd(ready_tx.into_raw_fd()) };
        let info_rx = unsafe { File::from_raw_fd(info_rx.into_raw_fd()) };

        Ok(Self {
            args,
            ready_tx,
            info_rx,
            command,
        })
    }

    pub fn apply_services(
        &mut self,
        services: &mut [BoxedService],
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
        tracing::info!("Spawning bwrap: {:?}", self.command);
        let child = self
            .command
            .arg("--bind")
            .arg(utils::temp_dir())
            .arg(utils::temp_dir())
            .arg(app)
            .args(args)
            .stdout(Stdio::inherit())
            .spawn()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?;

        let info = Info::from_pipe(&mut self.info_rx)?;
        Ok(BwrapProc::new(child, self.ready_tx, info.child_pid))
    }
}

#[derive(Debug)]
pub struct BwrapProc {
    proc: Child,
    nested_proc_pid: u32,
    ready_tx: File,
}

impl BwrapProc {
    fn new(proc: Child, ready_tx: File, nested_proc_pid: u32) -> Self {
        Self {
            proc,
            ready_tx,
            nested_proc_pid,
        }
    }

    pub fn pid(&self) -> u32 {
        self.nested_proc_pid
    }

    pub fn wait(mut self) -> Result<ExitStatus, AppError> {
        // Notify bwrap that it can start sandboxed app
        self.ready_tx.write(&[1]).map_err(AppError::PipeIO)?;

        let status = self
            .proc
            .wait()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?;

        Ok(status)
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
        tracing::error!("Child killed with {status:?}");
    }
}

#[derive(Debug, serde::Deserialize)]
struct Info {
    #[serde(rename = "child-pid")]
    child_pid: u32,
}

impl Info {
    pub fn from_pipe(rx: &mut File) -> Result<Self, AppError> {
        let mut reader = BufReader::new(rx);
        let mut buf = Vec::new();
        reader
            .read_until(b'}', &mut buf)
            .map_err(AppError::PipeIO)?;

        let info = serde_json::from_slice(&buf).map_err(AppError::BwrapInfo)?;
        Ok(info)
    }
}
