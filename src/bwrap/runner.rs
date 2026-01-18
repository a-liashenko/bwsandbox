use super::Args;
use crate::{error::AppError, utils};
use std::{
    ffi::OsString,
    fs::File,
    io::Read,
    os::fd::FromRawFd,
    process::{Command, ExitStatus},
};

#[derive(Debug)]
pub struct BwrapRunner<I> {
    ready_fd: File,
    args: I,
}

impl<I: IntoIterator<Item = OsString>> BwrapRunner<I> {
    pub fn new(args: Args<I>) -> Self {
        let ready_fd = unsafe { File::from_raw_fd(args.ready_fd) };
        Self {
            ready_fd,
            args: args.args,
        }
    }

    pub fn run(mut self) -> Result<ExitStatus, AppError> {
        let mut command = Command::new(utils::BWRAP_CMD);
        command.args(self.args);

        // Services startup shouldn't take too much time, raed interrupt ignored because chance is very low
        Self::wait_parent(&mut self.ready_fd)?;

        let status = command
            .spawn()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?
            .wait()
            .map_err(AppError::spawn(utils::BWRAP_CMD))?;
        Ok(status)
    }

    fn wait_parent(ready_fd: &mut File) -> Result<(), AppError> {
        let mut out = [0u8; 1];
        ready_fd.read(&mut out).map_err(AppError::PipeRead)?;
        tracing::trace!("Parent bytes {out:?}");

        let ready_flag = out[0];
        if ready_flag == 1 {
            return Ok(());
        }

        tracing::error!("Unexpected ready flag {}", ready_flag);
        Err(AppError::PipeUnexpectedStatus(ready_flag))
    }
}
