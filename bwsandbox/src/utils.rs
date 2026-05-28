use std::{path::PathBuf, time::Duration};

pub const RAND_ALPHABET: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

pub const APP_NAME: &str = env!("CARGO_CRATE_NAME");
pub const BWRAP_CMD: &str = "bwrap";
pub const DBUS_CMD: &str = "xdg-dbus-proxy";
pub const SLIRP4NETNS_CMD: &str = "slirp4netns";
pub const PASTA_CMD: &str = "pasta";

pub const READY_TIMEOUT: Duration = Duration::from_secs(3);

pub fn sandbox_id() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| rand_id(16))
}

pub fn rand_id(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).expect("Random not avail?");
    bytes
        .iter()
        .map(|el| RAND_ALPHABET[*el as usize % RAND_ALPHABET.len()] as char)
        .collect()
}

pub fn temp_dir() -> PathBuf {
    // RUNTIME_DIRECTORY - systemd headless
    // XDG_RUNTIME_DIR - user session
    let base = std::env::var("RUNTIME_DIRECTORY")
        .or_else(|_| std::env::var("XDG_RUNTIME_DIR"))
        .unwrap_or_else(|_| {
            log::warn!("Neither RUNTIME_DIRECTORY nor XDG_RUNTIME_DIR set, falling back to /tmp");
            std::env::temp_dir().to_string_lossy().into_owned()
        });

    PathBuf::from(base).join(format!("{APP_NAME}-workdir-{}", sandbox_id()))
}

pub fn deserialize<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, toml::de::Error> {
    toml::from_str(s)
}

#[test]
fn test_rand_id() {
    let size = 32;
    let id = rand_id(size);
    assert_eq!(id.len(), size);

    for ch in id.chars() {
        assert!(RAND_ALPHABET.contains(&(ch as u8)));
    }
}
