use crate::{
    bwrap::{ctl::BwrapCtl, proc::BwrapProc},
    error::AppError,
    services::{Context, Scope, ScopeCleanup, Service},
};
use std::{ffi::OsString, process::Command};

#[derive(Debug)]
pub struct ServiceCtx {
    command: Command,
    args: Vec<OsString>,
}

impl ServiceCtx {
    fn new(args: Vec<OsString>) -> Self {
        let command = Command::new(crate::utils::BWRAP_CMD);
        Self { command, args }
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
}

#[derive(Debug)]
pub struct ProcBuilder {
    ctx: ServiceCtx,
}

impl ProcBuilder {
    pub fn new(args: Vec<OsString>) -> Self {
        let mut ctx = ServiceCtx::new(args);
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
        let mut scope = Scope::new();
        for it in services.iter_mut() {
            scope += it.apply_before(&mut self.ctx)?;
        }
        self.ctx.apply_args();
        for it in services.iter_mut() {
            scope += it.apply_after(&mut self.ctx)?;
        }

        Ok(ScopeCleanup::from(scope))
    }

    pub fn spawn(mut self, app: OsString, app_args: Vec<OsString>) -> Result<BwrapProc, AppError> {
        use crate::system::{AsFdArg, SharedPipe};

        // Setup ready block
        let mut block = SharedPipe::new()?;
        self.ctx.command_mut().arg("--block-fd");
        self.ctx.command_mut().arg_fd(&block.share_rx()?)?;

        // Setup status callback
        let mut status = SharedPipe::new()?;
        self.ctx.command_mut().arg("--json-status-fd");
        self.ctx.command_mut().arg_fd(&status.share_tx()?)?;

        // Configure sandboxed app
        self.ctx.command_mut().arg(app);
        self.ctx.command_mut().args(app_args);
        crate::print_command::print_command(self.ctx.command_mut());

        let child = self
            .ctx
            .command
            .spawn()
            .map_err(AppError::spawn(crate::utils::BWRAP_CMD))?;
        let ctl = BwrapCtl::new(status.into_rx(), block.into_tx());
        BwrapProc::new(child, ctl)
    }
}
