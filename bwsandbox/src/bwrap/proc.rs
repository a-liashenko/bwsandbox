use crate::{
    bwrap::{SandboxStatus, ctl::BwrapCtl},
    error::AppError,
    services::BwrapInfo,
    system::PidFd,
    utils,
};
use rustix::process::Signal;
use std::process::{Child, ExitStatus};

// Notes:
// - bwrap treats --block-fd EOF as "green" flag to launch sandboxed app
// - bwrap will spawn nested bwrap and report nested pid (child_pid) via --json-status-fd
// - killing top level bwrap will NOT kill nested bwrap, in result after --block-fd drop sandboxed process will be executed
// Existing logic have a small race window:
// 1. bwrap sent status with child_pid
// 2. child_pid was killed for some reason, new process with exact same pid spawned
// 3. killing child_pid may kill something important
// Mitigation:
// - Collision chance is very low for such small amout of time
// - App designed to work from unpriveledged user, so in the worst case scenario unpriveldged process got killed
// - TODO: Use cgroups and kill whole cgroup, but keep in mind compatibility issues

#[derive(Debug)]
pub struct BwrapProc {
    proc: Child,
    ctl: BwrapCtl,
    status: SandboxStatus,
    child_pidfd: PidFd,
}

impl BwrapProc {
    pub fn new(proc: Child, mut ctl: BwrapCtl) -> Result<Self, AppError> {
        // FIXME: Potential race if child killed in between status and pifd_open
        let status = ctl.wait_status()?;
        let child_pidfd = PidFd::from_pid(status.child_pid)?;

        Ok(Self {
            proc,
            ctl,
            status,
            child_pidfd,
        })
    }

    pub fn bwrap_info(&self) -> BwrapInfo {
        BwrapInfo::new(self.proc.id(), self.status)
    }

    pub fn wait(mut self) -> Result<ExitStatus, AppError> {
        self.ctl.unblock();
        let status = self.ctl.wait_exit()?;
        Ok(status)
    }

    fn kill(&mut self) -> Result<(), AppError> {
        // Try to stop child process gracefully
        let _ = self.child_pidfd.send_sig(Signal::TERM);
        let status = self.child_pidfd.wait(utils::SIGTERM_TIMEOUT);
        if status.is_err() {
            log::error!("Failed to stop bwrap gracefuly");
            let status = self.child_pidfd.send_sig(Signal::KILL);
            log::error!("SIGKILL status: {status:?}");
        }

        // Now safe to wait until top-level bwrap finish
        self.proc
            .wait()
            .map_err(AppError::io("bwrap unknown state"))?;

        Ok(())
    }
}

impl Drop for BwrapProc {
    fn drop(&mut self) {
        let status = self.kill();
        log::trace!("bwrap exit status: {status:?}");
    }
}
