use crate::services::{Context, Scope, ScopeCleanup, Service};
use crate::{error::AppError, fd::AsFdExtra, utils};
use std::io::Write;
use std::process::{Child, ExitStatus};
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
    command: Command,
}

impl Context for BwrapProcBuilder {
    fn command_mut(&mut self) -> &mut std::process::Command {
        &mut self.command
    }
}

impl BwrapProcBuilder {
    pub fn new(args: Vec<OsString>) -> Result<Self, AppError> {
        // Create pipe to signal BwrapRunner that parent initialized all services
        let (ready_rx, ready_tx) = rustix::pipe::pipe().map_err(AppError::PipeAlloc)?;
        ready_rx.share_with_children()?;

        let mut command = Command::new(utils::SELF_CMD);
        command.arg(crate::utils::SELF_INTERNAL_ARG);
        command.arg(ready_rx.into_raw_fd().to_string());

        let ready_tx = unsafe { File::from_raw_fd(ready_tx.into_raw_fd()) };

        Ok(Self {
            args,
            ready_tx,
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
        tracing::info!("Spawning nested sandbox {:?}", self.command);
        let child = self
            .command
            .arg(app)
            .args(args)
            .spawn()
            .map_err(AppError::spawn(utils::SELF_CMD))?;

        Ok(BwrapProc {
            child,
            ready_tx: self.ready_tx,
        })
    }
}

#[derive(Debug)]
pub struct BwrapProc {
    child: Child,
    ready_tx: File,
}

impl BwrapProc {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn wait(mut self) -> Result<ExitStatus, AppError> {
        self.ready_tx
            .write(&[1])
            .map_err(AppError::file("__ready_fd__"))?;

        let status = self
            .child
            .wait()
            .map_err(AppError::spawn(utils::SELF_CMD))?;

        Ok(status)
    }
}

impl Drop for BwrapProc {
    fn drop(&mut self) {
        let status = match self.child.try_wait() {
            Ok(None) => {
                tracing::error!("Early BwrapProc drop? Killing child");
                self.child.kill()
            }
            Err(e) => {
                tracing::error!("Unknown BwrapProc status: {e:?}");
                self.child.kill()
            }
            _ => return,
        };
        tracing::error!("Child killed with {status:?}");
    }
}
