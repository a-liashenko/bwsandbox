use super::scope::Scope;
use crate::error::AppError;

pub trait Service<C: Context> {
    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn start(self: Box<Self>, pid: u32) -> Result<Box<dyn Handle>, AppError>;
}

pub trait Context: std::fmt::Debug {
    fn command_mut(&mut self) -> &mut std::process::Command;
}

pub trait Handle: std::fmt::Debug {
    fn stop(&mut self) -> Result<(), AppError>;
}

// Simple placeholder for services without any stop logic
impl Handle for () {
    fn stop(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}

// Automatically kill child on stop
impl Handle for std::process::Child {
    fn stop(&mut self) -> Result<(), AppError> {
        if let Err(e) = self.kill() {
            tracing::error!("Failed to kill service Child: {e:?}");
        }
        Ok(())
    }
}

// Do nothing, file will be closed on exit
impl Handle for std::fs::File {
    fn stop(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}

impl Handle for Box<dyn Handle> {
    fn stop(&mut self) -> Result<(), AppError> {
        self.as_mut().stop()
    }
}
