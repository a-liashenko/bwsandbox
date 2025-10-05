use crate::config::{Cmd, Entry};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config<D, S> {
    pub bwrap: Cmd,
    pub dbus: Option<Entry<D>>,
    pub seccomp: Option<Entry<S>>,
}
