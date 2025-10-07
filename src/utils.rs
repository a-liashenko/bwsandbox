use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::error::AppError;

pub const APP_NAME: &str = env!("CARGO_CRATE_NAME");
pub const BWRAP_CMD: &str = "bwrap";
pub const DBUS_CMD: &str = "xdg-dbus-proxy";

pub fn sandbox_id() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| rand_id(16))
}

pub fn rand_id(len: usize) -> String {
    nanoid::nanoid!(len)
}

pub fn temp_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or(std::env::temp_dir())
}

pub fn poll_file(path: &Path, poll: Duration, total: Duration) -> Result<bool, AppError> {
    let start = Instant::now();
    while start.elapsed() < total {
        if std::fs::exists(path).map_err(AppError::file(path))? {
            return Ok(true);
        }

        std::thread::sleep(poll);
    }

    Ok(false)
}

pub fn deserialize<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, toml::de::Error> {
    toml::from_str(s)
}
