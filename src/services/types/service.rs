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
    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn start(self: Box<Self>, status: &BwrapInfo) -> Result<HandleType, AppError>;
}

pub trait Context: std::fmt::Debug {
    fn command_mut(&mut self) -> &mut std::process::Command;
}

pub trait Handle: std::fmt::Debug {
    fn stop(&mut self) -> Result<(), AppError>;
}

pub trait HandleExt<T, E> {
    fn transpose(self) -> Option<Result<T, E>>;
}

impl<T, E> HandleExt<T, E> for Result<T, E> {
    fn transpose(self) -> Option<Result<T, E>> {
        match self {
            Ok(v) => Some(Ok(v)),
            Err(e) => Some(Err(e)),
        }
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

#[derive(Debug)]
pub enum HandleType {
    None,
    Owned { _drop: HandleOwned },
}

impl HandleType {
    pub fn new<T: Handle + 'static>(handle: T) -> Self {
        Self::Owned {
            _drop: HandleOwned::new(handle),
        }
    }
}

#[derive(Debug)]
pub struct HandleOwned {
    handle: Box<dyn Handle>,
}

impl HandleOwned {
    pub fn new<H: Handle + 'static>(handle: H) -> Self {
        let handle = Box::new(handle);
        Self { handle }
    }
}

impl Drop for HandleOwned {
    fn drop(&mut self) {
        if let Err(e) = self.handle.stop() {
            tracing::error!("Failed to stop service with {e:?}");
        }
    }
}
