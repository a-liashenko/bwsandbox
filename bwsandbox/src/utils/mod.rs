mod path_bin;
mod rand;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub use path_bin::which_bin;
#[cfg(test)]
pub use rand::rand_id;

pub const APP_NAME: &str = env!("CARGO_CRATE_NAME");

pub const BWRAP_CMD: &str = "bwrap";
pub const DBUS_CMD: &str = "xdg-dbus-proxy";
pub const SLIRP4NETNS_CMD: &str = "slirp4netns";
pub const PASTA_CMD: &str = "pasta";

pub const READY_TIMEOUT: Duration = Duration::from_secs(3);
pub const SIGTERM_TIMEOUT: Duration = Duration::from_secs(30);

pub fn sandbox_id() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| rand::rand_id(16))
}

pub fn temp_dir() -> &'static Path {
    const DIRS: &[&str] = &["RUNTIME_DIRECTORY", "XDG_RUNTIME_DIR"];

    static TEMP_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    TEMP_DIR.get_or_init(|| {
        let base = DIRS.iter().find_map(|v| std::env::var(v).ok());
        let base = base.unwrap_or_else(|| {
            log::warn!("Can't find any temp dir in {DIRS:?}, using std::env::temp_dir");
            std::env::temp_dir().to_string_lossy().into()
        });
        assert!(!base.is_empty());
        PathBuf::from(base).join(format!("{APP_NAME}-workdir-{}", sandbox_id()))
    })
}

pub fn deserialize<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, toml::de::Error> {
    toml::from_str(s)
}
