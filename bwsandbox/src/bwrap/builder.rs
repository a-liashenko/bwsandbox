use crate::{
    bwrap::{ctl::BwrapCtl, proc::BwrapProc},
    error::AppError,
    services::{Context, ScopeCleanup, Service},
};
use std::{ffi::OsString, process::Command};

#[derive(Debug)]
pub struct ServiceCtx {
    command: Command,
    args: Vec<OsString>,
    app: OsString,
}

impl ServiceCtx {
    fn new(app: OsString, args: Vec<OsString>) -> Self {
        let command = Command::new(crate::utils::BWRAP_CMD);
        Self { command, args, app }
    }

    fn apply_args(&mut self) {
        let args = std::mem::take(&mut self.args);
        self.command.args(args);
    }
}

impl Context for ServiceCtx {
    fn command_mut(&mut self) -> &mut std::process::Command {
        &mut self.command
    }

    fn arg_exist_before(&self, arg: &str) -> bool {
        debug_assert!(!self.args.is_empty(), "Valid only for apply_before");
        self.args.iter().any(|v| v == arg)
    }

    fn bin(&self) -> &std::ffi::OsStr {
        &self.app
    }
}

#[derive(Debug)]
pub struct ProcBuilder {
    ctx: ServiceCtx,
}

impl ProcBuilder {
    pub fn new(app: OsString, args: Vec<OsString>) -> Self {
        let mut ctx = ServiceCtx::new(app, args);
        // Allow access to services resources for sandboxed app (f.e. proxy dbus socket)
        ctx.command_mut().arg("--bind");
        ctx.command_mut().arg(crate::utils::temp_dir());
        ctx.command_mut().arg(crate::utils::temp_dir());

        // Inherit all output from bwrap and sandboxed app
        ctx.command_mut().stdout(std::process::Stdio::inherit());
        ctx.command_mut().stderr(std::process::Stdio::inherit());
        ctx.command_mut().stdin(std::process::Stdio::inherit());

        Self { ctx }
    }

    pub fn apply_services<S>(&mut self, services: &mut [S]) -> Result<ScopeCleanup, AppError>
    where
        S: Service<ServiceCtx>,
    {
        let mut cleanup = ScopeCleanup::new(services.len());
        for it in services.iter_mut() {
            let scope = it.apply_before(&mut self.ctx)?;
            cleanup.push(scope);
        }
        self.ctx.apply_args();
        for it in services.iter_mut() {
            let scope = it.apply_after(&mut self.ctx)?;
            cleanup.push(scope);
        }

        Ok(cleanup)
    }

    pub fn spawn(self, app_args: Vec<OsString>) -> Result<BwrapProc, AppError> {
        use crate::system::{AsFdArg, SharedPipe};

        let app = self.ctx.app;
        let mut command = self.ctx.command;

        // Setup ready block
        let mut block = SharedPipe::new()?;
        command.arg("--block-fd");
        command.arg_fd(&block.share_rx()?)?;

        // Setup status callback
        let mut status = SharedPipe::new()?;
        command.arg("--json-status-fd");
        command.arg_fd(&status.share_tx()?)?;

        // Configure sandboxed app
        command.arg(app).args(app_args);
        crate::print_command::print_command(&command);

        let child = command
            .spawn()
            .map_err(AppError::spawn(crate::utils::BWRAP_CMD))?;
        let ctl = BwrapCtl::new(status.into_rx(), block.into_tx());
        BwrapProc::new(child, ctl)
    }
}
