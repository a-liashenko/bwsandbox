use super::{Action, Arch, Syscall, Version};
use anyhow::ensure;
use std::{ffi::c_void, fs::File, os::fd::AsRawFd, ptr};

#[derive(Debug)]
pub struct FilterCtx(*mut c_void);

impl FilterCtx {
    #[tracing::instrument]
    pub fn new(def_action: Action) -> anyhow::Result<Self> {
        use super::seccomp_init;

        Version::new().verify_version(2, 5);

        let ptr = unsafe { seccomp_init(def_action.as_uint()) };
        ensure!(!ptr.is_null(), "seccomp_init: nullptr");
        Ok(Self(ptr))
    }

    #[tracing::instrument]
    pub fn rule_add(&mut self, act: Action, syscall: Syscall) -> anyhow::Result<()> {
        use super::seccomp_rule_add;

        let res = unsafe { seccomp_rule_add(self.0, act.as_uint(), syscall.raw(), 0) };
        ensure!(res == 0, "seccomp_rule_add: {res}");
        Ok(())
    }

    #[tracing::instrument]
    pub fn arch_add(&mut self, arch: Arch) -> anyhow::Result<()> {
        use super::seccomp_arch_add;

        let res = unsafe { seccomp_arch_add(self.0, arch.raw()) };
        ensure!(res == 0, "seccomp_arch_add: {res}");
        Ok(())
    }

    #[allow(unused)]
    pub fn load(&self) -> anyhow::Result<()> {
        use super::seccomp_load;

        let res = unsafe { seccomp_load(self.0) };
        ensure!(res == 0, "seccomp_load: {res}");
        Ok(())
    }

    #[tracing::instrument]
    pub fn export_bpf(&self, file: &mut File) -> anyhow::Result<()> {
        use super::seccomp_export_bpf;

        let fd = file.as_raw_fd();
        let res = unsafe { seccomp_export_bpf(self.0, fd) };
        ensure!(res == 0, "seccomp_export_bpf: {res}");
        Ok(())
    }
}

impl Drop for FilterCtx {
    fn drop(&mut self) {
        use super::seccomp_release;

        unsafe { seccomp_release(self.0) };
        self.0 = ptr::null_mut();
    }
}

#[test]
fn test_workflow() {
    let mut ctx = FilterCtx::new(Action::Allow).unwrap();

    let arch = Arch::from_str("x86").unwrap();
    ctx.arch_add(arch).unwrap();

    let syscall = Syscall::from_str("openat").unwrap();
    ctx.rule_add(Action::Errno, syscall).unwrap();

    assert!(File::open("/etc/hosts").is_ok());
    ctx.load().unwrap();
    assert!(File::open("/etc/hosts").is_err());
}
