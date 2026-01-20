use rustix::ioctl::Opcode;
use std::os::fd::{FromRawFd, OwnedFd};

pub struct NsGetParent;

unsafe impl rustix::ioctl::Ioctl for NsGetParent {
    type Output = OwnedFd;
    const IS_MUTATING: bool = false;

    fn opcode(&self) -> Opcode {
        linux_raw_sys::ioctl::NS_GET_PARENT
    }

    fn as_ptr(&mut self) -> *mut rustix::ffi::c_void {
        std::ptr::null_mut()
    }

    unsafe fn output_from_ptr(
        out: rustix::ioctl::IoctlOutput,
        _: *mut rustix::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        let fd = unsafe { OwnedFd::from_raw_fd(out) };
        Ok(fd)
    }
}
