use super::handle::{ChildHandle, HandleType};
use super::scope::Scope;
use crate::{bwrap::SandboxStatus, error::AppError};

#[derive(Debug)]
pub struct BwrapInfo {
    // Allow to keep root bwrap pid for traces and debug
    #[allow(unused)]
    pub pid: u32,
    pub sandbox: SandboxStatus,
}

pub trait Service<C: Context> {
    fn name(&self) -> &'static str;
    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn start(self: Box<Self>, status: &BwrapInfo) -> Result<HandleType, AppError>;
}

pub trait Context: std::fmt::Debug {
    fn command_mut(&mut self) -> &mut std::process::Command;
    fn arg_exist_before(&self, arg: &str) -> bool;
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
