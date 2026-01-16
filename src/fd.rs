use crate::error::AppError;
use rustix::io::fcntl_setfd;
use std::os::fd::{AsFd, AsRawFd};

pub trait FileExtraFd {
    fn share_with_children(&self) -> Result<(), AppError>;
}

impl<T: AsFd> FileExtraFd for T {
    fn share_with_children(&self) -> Result<(), AppError> {
        match fcntl_setfd(self, rustix::io::FdFlags::empty()) {
            Ok(_) => Ok(()),
            Err(e) => Err(AppError::FileFdShare(self.as_fd().as_raw_fd(), e)),
        }
    }
}
