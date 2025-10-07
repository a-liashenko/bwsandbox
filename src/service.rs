use crate::error::AppError;
use serde::de::DeserializeOwned;
use std::{path::PathBuf, process::Command};

pub trait Service: Sized {
    type Config: DeserializeOwned + std::fmt::Debug;
    type Handle: Handle;

    fn from_config(cfg: Self::Config) -> Result<Self, AppError>;
    fn apply<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError>;
    fn start(self) -> Result<Self::Handle, AppError>;
}

pub trait Context: std::fmt::Debug {
    fn sandbox_mut(&mut self) -> &mut Command;
}

pub trait Handle: std::fmt::Debug {
    fn stop(&mut self) -> Result<(), AppError>;
}

impl Handle for () {
    fn stop(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}

impl Handle for Box<dyn Handle> {
    fn stop(&mut self) -> Result<(), AppError> {
        self.as_mut().stop()
    }
}

#[derive(Debug, Default)]
pub struct Scope {
    pub remove: Vec<PathBuf>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.remove.push(file.into());
        self
    }
}
