use super::{AsFdExtra, ReadExt};
use crate::error::AppError;
use std::{
    io::{PipeReader, PipeWriter, Read},
    os::fd::AsFd,
    time::Duration,
};

#[derive(Debug)]
pub enum PipePart<T> {
    Owned(Option<T>),
    Shared(T),
}

impl<T: AsFd> PipePart<T> {
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

    fn borrow_part_mut(&mut self) -> &mut T {
        if let Self::Owned(Some(v)) = self {
            return v;
        }
        panic!("Using fd after it was shared not allowed")
    }
}

impl<T: AsFd + Read> ReadExt for PipePart<T> {
    fn read_buf_ext<const B: usize>(&mut self) -> Result<(usize, [u8; B]), std::io::Error> {
        self.borrow_part_mut().read_buf_ext()
    }

    fn try_read_buf_ext<const B: usize>(
        &mut self,
        timeout: Duration,
    ) -> Result<(usize, [u8; B]), std::io::Error> {
        self.borrow_part_mut().try_read_buf_ext(timeout)
    }
}

impl<T: AsFd> AsFd for PipePart<T> {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        match self {
            Self::Owned(Some(v)) | Self::Shared(v) => v.as_fd(),
            Self::Owned(None) => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct SharedPipe {
    rx: PipePart<PipeReader>,
    tx: PipePart<PipeWriter>,
}

impl SharedPipe {
    pub fn new() -> Result<Self, AppError> {
        let (rx, tx) = std::io::pipe().map_err(AppError::PipeAlloc)?;
        let rx = PipePart::new(rx);
        let tx = PipePart::new(tx);
        Ok(Self { rx, tx })
    }

    pub fn share_rx(&mut self) -> Result<&PipePart<PipeReader>, AppError> {
        self.rx.share()
    }

    pub fn share_tx(&mut self) -> Result<&PipePart<PipeWriter>, AppError> {
        self.tx.share()
    }

    pub fn into_rx(self) -> PipeReader {
        self.rx.take_part()
    }

    pub fn into_tx(self) -> PipeWriter {
        self.tx.take_part()
    }
}
