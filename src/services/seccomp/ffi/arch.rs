use anyhow::{Context, ensure};
use std::ffi::CString;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Arch(u32);

impl Arch {
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> u32 {
        self.0
    }

    #[tracing::instrument]
    pub fn from_str(name: &str) -> anyhow::Result<Self> {
        use super::seccomp_arch_resolve_name;

        let cstr = CString::new(name).with_context(|| format!("CString::new({name})"))?;
        let ec = unsafe { seccomp_arch_resolve_name(cstr.as_ptr()) };
        ensure!(ec != 0, "seccomp_arch_resolve_name({name}): {ec}");

        Ok(Self::from_raw(ec))
    }
}

#[test]
fn test_invalid() {
    let res = Arch::from_str("Invalid arch");
    assert!(res.is_err());
}

#[test]
fn test_valid() {
    let res = Arch::from_str("x86_64");
    assert!(res.is_ok());
}
