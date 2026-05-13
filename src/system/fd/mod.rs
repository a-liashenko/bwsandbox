mod shared_pipe;

use crate::error::AppError;
use std::{
    io::Read,
    os::fd::{AsFd, AsRawFd},
    process::Command,
    time::Duration,
};

pub use shared_pipe::SharedPipe;

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
    fn arg_fd_path(&mut self, fd: &T) -> &mut Command;
}

impl<T: AsFd> AsFdArg<T> for std::process::Command {
    fn arg_fd(&mut self, fd: &T) -> Result<&mut Command, AppError> {
        let command = self.arg(fd.as_fd().as_raw_fd().to_string());
        Ok(command)
    }

    fn arg_fd_path(&mut self, fd: &T) -> &mut Command {
        let fd = fd.as_fd().as_raw_fd();
        let arg = format!("/proc/{}/fd/{}", std::process::id(), fd);
        self.arg(arg)
    }
}

pub trait ReadExt {
    fn read_ext<const B: usize>(&mut self) -> Result<(usize, [u8; B]), std::io::Error>;
    fn try_read_ext<const B: usize>(
        &mut self,
        timeout: Duration,
    ) -> Result<(usize, [u8; B]), std::io::Error>;
}

impl<T: AsFd + Read> ReadExt for T {
    fn read_ext<const B: usize>(&mut self) -> Result<(usize, [u8; B]), std::io::Error> {
        let mut buf = [0u8; B];
        let bytes = self.read(&mut buf)?;
        if bytes == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        Ok((bytes, buf))
    }

    fn try_read_ext<const B: usize>(
        &mut self,
        timeout: Duration,
    ) -> Result<(usize, [u8; B]), std::io::Error> {
        use rustix::event::{PollFd, PollFlags, Timespec};

        let timeout = Timespec {
            tv_sec: timeout.as_secs().try_into().expect("Bad timeout value"),
            tv_nsec: timeout.subsec_nanos().into(),
        };
        let mut poll_fds = [PollFd::new(self, PollFlags::IN | PollFlags::HUP)];
        let ready = rustix::event::poll(&mut poll_fds, Some(&timeout))?;
        if ready == 0 {
            return Err(std::io::ErrorKind::TimedOut.into());
        }

        let evts = poll_fds[0].revents();
        if evts.contains(PollFlags::HUP) && !evts.contains(PollFlags::IN) {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        self.read_ext()
    }
}
