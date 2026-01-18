use crate::error::AppError;
use lexopt::ValueExt;
use rustix::fd::FromRawFd;
use std::{ffi::OsString, os::fd::RawFd};

#[derive(Debug)]
pub struct Args<I> {
    pub ready_fd: RawFd,
    pub args: I,
}

impl<I: Iterator<Item = OsString>> Args<I> {
    pub fn from_iter(mut args: I) -> Result<Self, AppError> {
        let internal = args.next().ok_or(AppError::BadArgs)?;
        if internal != crate::utils::SELF_INTERNAL_ARG {
            tracing::error!("Unexpected first argument for internal launch: {internal:?}");
            return Err(AppError::BadArgs);
        }

        let ready_fd = args.next().ok_or(AppError::BadArgs)?;
        let ready_fd = ready_fd.parse::<i32>()?;
        let ready_fd = unsafe { RawFd::from_raw_fd(ready_fd) };

        Ok(Self { ready_fd, args })
    }
}
