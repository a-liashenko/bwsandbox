mod action;
mod arch;
mod filter_ctx;
mod syscall;

pub use action::Action;
pub use arch::Arch;
pub use filter_ctx::FilterCtx;
pub use syscall::Syscall;

use std::ffi::{c_char, c_int, c_uint, c_void};

const SCMP_ACT_KILL: c_uint = 0x80000000;
const SCMP_ACT_ALLOW: c_uint = 0x7fff0000;
// TODO: Allow custom codes, not only EPERM
const SCMP_ACT_ERRNO: c_uint = 0x00050001;

const __NR_SCMP_ERROR: i32 = -1;

#[link(name = "seccomp", kind = "dylib")]
unsafe extern "C" {
    fn seccomp_version() -> *const Version;

    fn seccomp_syscall_resolve_name(name: *const c_char) -> c_int;
    fn seccomp_arch_resolve_name(name: *const c_char) -> c_uint;

    fn seccomp_init(def_action: c_uint) -> *mut c_void;
    fn seccomp_load(ctx: *const c_void) -> c_int;
    fn seccomp_release(ctx: *mut c_void);

    fn seccomp_export_bpf(ctx: *const c_void, fd: c_int) -> c_int;

    fn seccomp_arch_add(ctx: *mut c_void, arch_token: c_uint) -> c_int;
    fn seccomp_rule_add(
        ctx: *mut c_void,
        action: c_uint,
        syscall: c_int,
        arg_cnt: c_uint,
        ...
    ) -> c_int;

}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Version {
    pub major: c_uint,
    pub minor: c_uint,
    pub micro: c_uint,
}

impl Version {
    pub fn new() -> Self {
        unsafe { *seccomp_version() }
    }

    pub fn verify_version(&self, major: c_uint, minor: c_uint) {
        if self.major < major || self.minor < minor {
            eprintln!("libseccomp >= {major}.{minor} required");
            std::process::exit(1)
        }
    }
}
