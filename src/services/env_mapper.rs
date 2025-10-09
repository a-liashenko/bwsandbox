use crate::{
    error::AppError,
    service::{Context, Scope, Service},
};
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

fn unset_all_default() -> bool {
    true
}

impl Service for EnvMapper {
    type Config = Self;
    type Handle = ();

    fn from_config(config: Self::Config) -> Result<Self, AppError> {
        Ok(config)
    }

    fn apply_before<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        if self.unset_all {
            ctx.sandbox_mut().arg("--clearenv");
        }

        for it in &self.keep {
            if let Ok(v) = std::env::var(it) {
                ctx.sandbox_mut().arg("--setenv").arg(it).arg(v);
            }
        }

        for it in &self.unset {
            ctx.sandbox_mut().arg("--unsetenv").arg(it);
        }

        Ok(Scope::new())
    }

    fn apply_after<C: Context>(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    fn start(self) -> Result<Self::Handle, AppError> {
        Ok(())
    }
}
