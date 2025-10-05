use crate::{
    error::AppError,
    service::{Context, Scope, Service},
};
use std::process::Command;

#[derive(Debug)]
pub struct Sandbox {
    command: Command,
    scopes: Vec<Scope>,
}

impl Context for Sandbox {
    fn sandbox_mut(&mut self) -> &mut Command {
        &mut self.command
    }
}

impl Sandbox {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            scopes: Default::default(),
        }
    }

    pub fn apply<S: Service>(&mut self, service: &mut S) -> Result<(), AppError> {
        let scope = service.apply(self)?;
        self.scopes.push(scope);
        Ok(())
    }

    pub fn apply_opt<S: Service>(&mut self, service: Option<&mut S>) -> Result<(), AppError> {
        if let Some(v) = service {
            return self.apply(v);
        }
        Ok(())
    }

    pub fn into_parts(self) -> (Command, Vec<Scope>) {
        (self.command, self.scopes)
    }
}
