use std::sync::atomic::{AtomicBool, Ordering};

pub static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);
pub fn set_trace_enabled(enabled: bool) {
    TRACE_ENABLED.store(enabled, Ordering::Relaxed);
}

macro_rules! trace {
    ($($arg:tt)*) => {
        if crate::trace::TRACE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[TRACE] {}:{} - {}", file!(), line!(), format_args!($($arg)*));
        }
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        if crate::trace::TRACE_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[ERROR] {}:{} - {}", file!(), line!(), format_args!($($arg)*));
        }
    };
}

pub(crate) use error;
pub(crate) use trace;
