use crate::error::AppError;
use crate::fd::FileExtraFd;
use crate::service::{Context, Scope, Service};
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

impl Service for SeccompService {
    type Config = Config;
    type Handle = Handle;

    #[tracing::instrument]
    fn from_config(cfg: Self::Config) -> Result<Self, AppError> {
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
            .context("Failed to export compilled seccomp filter")
            .map_err(AppError::SeccompLib)?;

        fd.share_with_children()?;
        fd.rewind().map_err(AppError::file("__in-memory__"))?;

        Ok(Self { fd })
    }

    fn apply_before<C: Context>(&mut self, _ctx: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    #[tracing::instrument]
    fn apply_after<C: Context>(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        ctx.command_mut()
            .arg("--seccomp")
            .arg(self.fd.as_raw_fd().to_string());
        Ok(Scope::new())
    }

    #[tracing::instrument]
    fn start(self, _pid: u32) -> Result<Self::Handle, AppError> {
        Ok(Handle { _fd: self.fd })
    }
}

#[derive(Debug)]
pub struct Handle {
    _fd: File,
}

impl crate::service::Handle for Handle {
    fn stop(&mut self) -> Result<(), AppError> {
        // Do nothing, tempfile will be closed by OS
        Ok(())
    }
}
