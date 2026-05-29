use crate::{error::AppError, system::poll::Poll};
use std::{
    io::Read,
    os::fd::{AsFd, AsRawFd},
    process::Command,
    time::Duration,
};

pub trait AsFdExtra {
    fn share_with_children(&self) -> Result<(), AppError>;
}

impl<T: AsFd> AsFdExtra for T {
    fn share_with_children(&self) -> Result<(), AppError> {
        use rustix::io::fcntl_setfd;

        match fcntl_setfd(self, rustix::io::FdFlags::empty()) {
            Ok(()) => Ok(()),
            Err(e) => Err(AppError::FileFdShare(self.as_fd().as_raw_fd(), e)),
        }
    }
}

pub trait AsFdArg<T: AsFd> {
    fn arg_fd(&mut self, fd: &T) -> Result<&mut Command, AppError>;
}

impl<T: AsFd> AsFdArg<T> for std::process::Command {
    fn arg_fd(&mut self, fd: &T) -> Result<&mut Command, AppError> {
        let command = self.arg(fd.as_fd().as_raw_fd().to_string());
        Ok(command)
    }
}

pub trait ReadExt {
    fn read_buf_ext<const B: usize>(&mut self) -> Result<(usize, [u8; B]), std::io::Error>;
    fn try_read_buf_ext<const B: usize>(
        &mut self,
        timeout: Duration,
    ) -> Result<(usize, [u8; B]), std::io::Error>;
}

impl<T: AsFd + Read> ReadExt for T {
    fn read_buf_ext<const B: usize>(&mut self) -> Result<(usize, [u8; B]), std::io::Error> {
        let mut buf = [0u8; B];
        let bytes = self.read(&mut buf)?;
        if bytes == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        Ok((bytes, buf))
    }

    fn try_read_buf_ext<const B: usize>(
        &mut self,
        timeout: Duration,
    ) -> Result<(usize, [u8; B]), std::io::Error> {
        Poll::new(&self).poll_in(timeout)?;
        self.read_buf_ext()
    }
}
