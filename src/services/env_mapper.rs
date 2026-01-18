use crate::error::AppError;
use crate::services::{Context, Handle, Scope, Service};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct EnvMapper {
    #[serde(default = "unset_all_default")]
    unset_all: bool,
    #[serde(default)]
    keep: Vec<String>,
    #[serde(default)]
    unset: Vec<String>,
}

impl EnvMapper {
    pub fn from_config(config: Self) -> Result<Self, AppError> {
        Ok(config)
    }
}

fn unset_all_default() -> bool {
    true
}

impl<C: Context> Service<C> for EnvMapper {
    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        if self.unset_all {
            ctx.command_mut().arg("--clearenv");
        }

        for it in &self.keep {
            if let Ok(v) = std::env::var(it) {
                ctx.command_mut().arg("--setenv").arg(it).arg(v);
            }
        }

        for it in &self.unset {
            ctx.command_mut().arg("--unsetenv").arg(it);
        }

        Ok(Scope::new())
    }

    fn apply_after(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    fn start(self: Box<Self>, _pid: u32) -> Result<Box<dyn Handle>, AppError> {
        Ok(Box::new(()))
    }
}
