use crate::error::AppError;
use crate::fd::AsFdExtra;
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use anyhow::Context as _;
use std::io::Seek;
use std::{fs::File, os::fd::AsRawFd};

mod config;
mod ffi;

pub use config::Config;

#[derive(Debug)]
#[repr(transparent)]
pub struct SeccompService {
    fd: File,
}

impl SeccompService {
    pub fn from_config(cfg: Config) -> Result<Self, AppError> {
        let mut filter = ffi::FilterCtx::new(cfg.default_action).map_err(AppError::SeccompLib)?;

        for arch in cfg.extra_arch {
            filter
                .arch_add(arch)
                .with_context(|| format!("Failed to add {arch:?}"))
                .map_err(AppError::SeccompLib)?;
        }

        for rule in cfg.rules {
            for syscall in rule.syscalls {
                filter
                    .rule_add(rule.action, syscall)
                    .with_context(|| format!("Failed to add {syscall:?} rule"))
                    .map_err(AppError::SeccompLib)?;
            }
        }

        // Use in-memory(?) temp file, it will be cleaned by OS
        let mut fd = tempfile::tempfile().map_err(AppError::FileTempAlloc)?;
        filter
            .export_bpf(&mut fd)
            .context("Failed to export compiled seccomp filter")
            .map_err(AppError::SeccompLib)?;

        fd.share_with_children()?;
        fd.rewind().map_err(AppError::file("__seccomp-bpf__"))?;

        Ok(Self { fd })
    }
}

impl<C: Context> Service<C> for SeccompService {
    fn apply_before(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    #[tracing::instrument]
    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        ctx.command_mut()
            .arg("--seccomp")
            .arg(self.fd.as_raw_fd().to_string());
        Ok(Scope::new())
    }

    #[tracing::instrument]
    fn start(self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        Ok(HandleType::None)
    }
}
