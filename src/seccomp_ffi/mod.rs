mod action;
mod arch;
mod filter_ctx;
mod syscall;

pub use action::Action;
pub use arch::Arch;
pub use filter_ctx::FilterCtx;
pub use syscall::Syscall;

use std::ffi::{c_char, c_void};

const SCMP_ACT_KILL: u32 = 0x80000000;
const SCMP_ACT_ALLOW: u32 = 0x7fff0000;
const SCMP_ACT_ERRNO: u32 = 0x00050001;

const __NR_SCMP_ERROR: i32 = -1;

#[link(name = "seccomp", kind = "dylib")]
unsafe extern "C" {
    fn seccomp_init(def_action: u32) -> *mut c_void;
    fn seccomp_release(ctx: *mut c_void);

    fn seccomp_rule_add(ctx: *mut c_void, action: u32, syscall: i32, arg_cnt: u32, ...) -> i32;
    fn seccomp_arch_add(ctx: *mut c_void, arch_token: u32) -> i32;

    fn seccomp_syscall_resolve_name(name: *const c_char) -> i32;
    fn seccomp_arch_resolve_name(name: *const c_char) -> u32;

    fn seccomp_load(ctx: *const c_void) -> i32;
    fn seccomp_export_bpf(ctx: *const c_void, fd: i32) -> i32;

}
