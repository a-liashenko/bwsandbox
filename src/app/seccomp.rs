use crate::{error::Error, models::seccomp::SeccompConfig, seccomp_ffi::FilterCtx};
use nix::fcntl::{FcntlArg::F_SETFD, FdFlag, fcntl};
use std::{
    fs::File,
    io::Seek,
    os::fd::{AsRawFd, RawFd},
    path::PathBuf,
};

#[derive(Debug)]
pub struct Seccomp {
    path: PathBuf,
    bpf: File,
}

impl Seccomp {
    pub fn new(cfg: SeccompConfig, path: PathBuf) -> Result<Self, Error> {
        let mut ctx = FilterCtx::new(cfg.default_action)?;

        for arch in &cfg.extra_arch {
            ctx.arch_add(*arch)?;
        }

        for rule in &cfg.rules {
            for syscall in &rule.syscalls {
                ctx.rule_add(rule.action, *syscall)?;
            }
        }

        let mut bpf = File::create_new(&path).map_err(Error::file(&path))?;
        ctx.export_bpf(&mut bpf)?;

        // Seek file to 0, so child can read from it
        bpf.rewind().map_err(Error::file(&path))?;
        // Allow child process to inherit seccomp FD
        fcntl(&bpf, F_SETFD(FdFlag::empty())).map_err(Error::Fcntl)?;

        Ok(Self { path, bpf })
    }

    pub fn fd(&self) -> RawFd {
        self.bpf.as_raw_fd()
    }
}

impl Drop for Seccomp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
