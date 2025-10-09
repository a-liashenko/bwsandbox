use crate::{
    app::scope_destroyer::ScopeDestroyer,
    error::AppError,
    service::{Context, Scope, Service},
};
use std::{ffi::OsString, process::Command};

#[derive(Debug)]
pub struct Sandbox {
    bin: Command,
    command_args: Vec<OsString>,
    scope: Scope,
}

impl Context for Sandbox {
    fn sandbox_mut(&mut self) -> &mut Command {
        &mut self.bin
    }
}

impl Sandbox {
    pub fn new(bin: impl Into<OsString>, command_args: Vec<OsString>) -> Self {
        let bin = Command::new(bin.into());
        Self {
            bin,
            command_args,
            scope: Scope::new(),
        }
    }

    pub fn apply_before<S: Service>(&mut self, service: &mut S) -> Result<(), AppError> {
        let scope = service.apply_before(self)?;
        self.scope.merge(scope);
        Ok(())
    }

    pub fn prebuild(&mut self) {
        let args = std::mem::take(&mut self.command_args);
        self.bin.args(args);
    }

    pub fn apply_after<S: Service>(&mut self, service: &mut S) -> Result<(), AppError> {
        assert!(
            self.command_args.is_empty(),
            "prebuild() must be called before apply_after()"
        );

        let scope = service.apply_after(self)?;
        self.scope.merge(scope);
        Ok(())
    }

    pub fn build(self) -> Result<(Command, ScopeDestroyer), AppError> {
        let scope_destroyer = ScopeDestroyer::new(vec![self.scope])?;
        Ok((self.bin, scope_destroyer))
    }
}
