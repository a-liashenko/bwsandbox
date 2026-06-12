use super::handle::{ChildHandle, HandleType};
use super::scope::Scope;
use crate::{bwrap::SandboxStatus, error::AppError};
use std::ffi::OsStr;

#[derive(Debug)]
pub struct BwrapInfo {
    pub pid: u32,
    pub sandbox: SandboxStatus,
}

impl BwrapInfo {
    pub fn new(pid: u32, sandbox: SandboxStatus) -> Self {
        Self { pid, sandbox }
    }
}

pub trait Context: std::fmt::Debug {
    fn command_mut(&mut self) -> &mut std::process::Command;
    fn arg_exist_before(&self, arg: &str) -> bool;
    fn bin(&self) -> &OsStr;
}

pub trait Service<C: Context> {
    fn name(&self) -> &'static str;
    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn start(self: Box<Self>, status: &BwrapInfo) -> Result<HandleType, AppError>;
}

impl<C: Context> Service<C> for Box<dyn Service<C>> {
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }

    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        self.as_mut().apply_before(ctx)
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        self.as_mut().apply_after(ctx)
    }

    fn start(self: Box<Self>, status: &BwrapInfo) -> Result<HandleType, AppError> {
        (*self).start(status)
    }
}

// Force spawn_service() instead of spawn() to wrap into Handle with .kill()/.wait() in drop
pub trait ServiceCommand {
    fn spawn_service(&mut self) -> Result<ChildHandle, std::io::Error>;
}

impl ServiceCommand for std::process::Command {
    fn spawn_service(&mut self) -> Result<ChildHandle, std::io::Error> {
        self.spawn().map(ChildHandle::new)
    }
}
