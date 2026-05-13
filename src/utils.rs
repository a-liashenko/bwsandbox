use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::error::AppError;

pub const APP_NAME: &str = env!("CARGO_CRATE_NAME");
pub const BWRAP_CMD: &str = "bwrap";
pub const DBUS_CMD: &str = "xdg-dbus-proxy";
pub const SLIRP4NETNS_CMD: &str = "slirp4netns";
pub const PASTA_CMD: &str = "pasta";

pub const READY_POLL: Duration = Duration::from_millis(100);
pub const READY_TIMEOUT: Duration = Duration::from_secs(3);

pub fn sandbox_id() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| rand_id(16))
}

pub fn rand_id(len: usize) -> String {
    nanoid::nanoid!(len, &nanoid::alphabet::HEX_UPPERCASE)
}

pub fn temp_dir() -> PathBuf {
    // RUNTIME_DIRECTORY - systemd headless
    // XDG_RUNTIME_DIR - user session
    let base = std::env::var("RUNTIME_DIRECTORY")
        .or_else(|_| std::env::var("XDG_RUNTIME_DIR"))
        .unwrap_or_else(|_| {
            tracing::warn!(
                "Neither RUNTIME_DIRECTORY nor XDG_RUNTIME_DIR set, falling back to /tmp"
            );
            std::env::temp_dir().to_string_lossy().into_owned()
        });

    PathBuf::from(base).join(format!("{APP_NAME}-workdir-{}", sandbox_id()))
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
