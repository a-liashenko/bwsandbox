use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use crate::error::AppError;

#[derive(Debug)]
pub struct SigTerm {
    terminated: Arc<AtomicBool>,
}

impl SigTerm {
    pub fn register() -> Result<Self, AppError> {
        use signal_hook::consts::{SIGINT, SIGTERM};

        let terminated = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGTERM, terminated.clone()).map_err(AppError::CtrlC)?;
        signal_hook::flag::register(SIGINT, terminated.clone()).map_err(AppError::CtrlC)?;
        Ok(Self { terminated })
    }

    pub fn is_terminated(&self) -> bool {
        // This load will be called once in app lifetime, so we care about correctness more than performance
        self.terminated.load(SeqCst)
    }
}
