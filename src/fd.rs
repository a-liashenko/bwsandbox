use crate::error::AppError;
use rustix::io::fcntl_setfd;
use std::{
    io::{PipeReader, PipeWriter},
    os::fd::{AsFd, AsRawFd},
    process::Command,
};

pub trait AsFdExtra {
    fn share_with_children(&self) -> Result<(), AppError>;
}

impl<T: AsFd> AsFdExtra for T {
    fn share_with_children(&self) -> Result<(), AppError> {
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

#[derive(Debug)]
pub enum FdStatus<T> {
    Owned(Option<T>),
    Shared(T),
}

impl<T: AsFd> FdStatus<T> {
    fn new(val: T) -> Self {
        Self::Owned(Some(val))
    }

    fn share(&mut self) -> Result<&Self, AppError> {
        if let Self::Owned(v) = self {
            let fd = std::mem::take(v).expect("not possible");
            fd.share_with_children()?;
            *self = Self::Shared(fd);
        }

        Ok(self)
    }

    fn take_part(self) -> T {
        // Shared fd should be closed on parent part after child process spawned
        // It will help with dangling fd if child process ignored fd and not closed it
        if let Self::Owned(Some(v)) = self {
            return v;
        }
        panic!("Using fd after it was shared not allowed")
    }
}

impl<T: AsFd> AsFd for FdStatus<T> {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        match self {
            Self::Owned(Some(v)) | Self::Shared(v) => v.as_fd(),
            Self::Owned(None) => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct SharedPipe {
    rx: FdStatus<PipeReader>,
    tx: FdStatus<PipeWriter>,
}

impl SharedPipe {
    pub fn new() -> Result<Self, AppError> {
        let (rx, tx) = std::io::pipe().map_err(AppError::PipeAlloc)?;
        let rx = FdStatus::new(rx);
        let tx = FdStatus::new(tx);
        Ok(Self { rx, tx })
    }

    pub fn share_rx(&mut self) -> Result<&FdStatus<PipeReader>, AppError> {
        self.rx.share()
    }

    pub fn share_tx(&mut self) -> Result<&FdStatus<PipeWriter>, AppError> {
        self.tx.share()
    }

    pub fn into_rx(self) -> PipeReader {
        self.rx.take_part()
    }

    pub fn into_tx(self) -> PipeWriter {
        self.tx.take_part()
    }
}
