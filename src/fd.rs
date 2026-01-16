use crate::error::AppError;
use rustix::io::fcntl_setfd;
use std::os::fd::AsRawFd;

pub trait FileExtraFd {
    fn share_with_children(&self) -> Result<(), AppError>;
}

impl FileExtraFd for std::fs::File {
    fn share_with_children(&self) -> Result<(), AppError> {
        match fcntl_setfd(self, rustix::io::FdFlags::empty()) {
            Ok(_) => Ok(()),
            Err(e) => Err(AppError::FileFdShare(self.as_raw_fd(), e)),
        }
    }
}
