use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use crate::error::AppError;

#[derive(Debug)]
pub struct SigTerm {
    terminated: Arc<AtomicBool>,
}

impl SigTerm {
    pub fn register() -> Result<Self, AppError> {
        let terminated = Arc::new(AtomicBool::new(false));

        {
            let terminated = terminated.clone();
            ctrlc::try_set_handler(move || {
                // bwrap is responsible to handle terminate signal and shutdown sandboxed app
                // sandboxed app is responsible to handle terminate signal
                // if one of the party ignore it - be it, we will politely wait until killed
                terminated.store(true, SeqCst);
            })?;
        }

        Ok(Self { terminated })
    }

    pub fn is_terminated(&self) -> bool {
        // This load will be called once in app lifetime, so we care about correctness more than performance
        self.terminated.load(SeqCst)
    }
}
