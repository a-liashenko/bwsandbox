use crate::bwrap::{SandboxStatus, events::EventsReader};
use crate::error::AppError;
use std::io::{PipeReader, PipeWriter};
use std::process::ExitStatus;

#[derive(Debug)]
pub struct BwrapCtl {
    status_rx: EventsReader<PipeReader>,
    block_tx: Option<PipeWriter>,
}

impl BwrapCtl {
    pub fn new(status_rx: PipeReader, block_tx: PipeWriter) -> Self {
        let status_rx = EventsReader::new(status_rx);
        Self {
            status_rx,
            block_tx: Some(block_tx),
        }
    }

    pub fn unblock(&mut self) {
        // EOF will allow app to start
        // https://github.com/containers/bubblewrap/issues/753
        std::mem::take(&mut self.block_tx);
    }

    pub fn wait_status(&mut self) -> Result<SandboxStatus, AppError> {
        self.status_rx.try_next()
    }

    pub fn wait_exit(&mut self) -> Result<ExitStatus, AppError> {
        use super::{events::Events, sigterm::SigTerm};
        use linux_raw_sys::general::SIGINT;
        use std::os::unix::process::ExitStatusExt;

        let sig = SigTerm::register()?;
        loop {
            match self.status_rx.try_next::<Events>() {
                Ok(Events::Exit(status)) => {
                    return Ok(ExitStatus::from_raw(status.exit_code));
                }
                Ok(evt) => {
                    log::warn!("Unknown bwrap event: {evt:?}");
                }
                Err(e) if sig.is_terminated() => {
                    log::info!("SIGTERM/SIGINT received: {e:?}");
                    let code = i32::try_from(SIGINT).expect("SIGINT u32 -> i32");
                    return Ok(ExitStatus::from_raw(code));
                }
                Err(e) => {
                    log::warn!("bwrap unexpected exit: {e:?}");
                    return Err(e);
                }
            }
        }
    }
}
