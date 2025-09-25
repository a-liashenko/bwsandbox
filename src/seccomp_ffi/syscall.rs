use anyhow::{Context, ensure};
use std::ffi::CString;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Syscall(i32);

impl Syscall {
    pub fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> i32 {
        self.0
    }

    pub fn from_str(name: &str) -> anyhow::Result<Self> {
        use super::{__NR_SCMP_ERROR, seccomp_syscall_resolve_name};

        let cstr = CString::new(name).with_context(|| format!("CString::new({name})"))?;
        let ec = unsafe { seccomp_syscall_resolve_name(cstr.as_ptr()) };
        ensure!(ec != __NR_SCMP_ERROR, "syscall_resolve_name({name}): {ec}");

        Ok(Self::from_raw(ec))
    }
}

#[test]
fn test_invalid() {
    let res = Syscall::from_str("Invalid syscall");
    assert!(res.is_err());
}

#[test]
fn test_valid() {
    let res = Syscall::from_str("open");
    assert!(res.is_ok());
}
