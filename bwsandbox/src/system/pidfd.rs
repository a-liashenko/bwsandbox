use crate::system::poll::Poll;

use super::Error;
use rustix::fd::OwnedFd;
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open};
use std::time::Duration;

#[derive(Debug)]
#[repr(transparent)]
pub struct PidFd {
    fd: OwnedFd,
}

impl PidFd {
    pub fn from_pid(pid: u32) -> Result<Self, Error> {
        let pid = Pid::from_raw(pid.cast_signed()).expect("pid cast u32->i32");
        let fd = pidfd_open(pid, PidfdFlags::empty()).map_err(Error::PidFdOpen)?;
        Ok(Self { fd })
    }

    pub fn send_sig(&self, sig: Signal) -> Result<(), Error> {
        rustix::process::pidfd_send_signal(&self.fd, sig).map_err(Error::PidFdSig)?;
        Ok(())
    }

    pub fn wait(&self, timeout: Duration) -> Result<(), std::io::Error> {
        Poll::new(&self.fd).poll_in(timeout)
    }
}
