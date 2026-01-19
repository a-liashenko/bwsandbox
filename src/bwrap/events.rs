use crate::error::AppError;
use serde::Deserialize;
use std::io::{BufRead, BufReader, ErrorKind::UnexpectedEof};

pub trait EventType: serde::de::DeserializeOwned {}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(unused)] // Keep all events if any needed in future
pub enum Events {
    Status(SandboxStatus),
    Exit(ExitStatus),
}

impl EventType for Events {}

// Example: { "child-pid": 77360, "cgroup-namespace": 4026534046, "ipc-namespace": 4026534044, "mnt-namespace": 4026534042, "net-namespace": 4026534047, "pid-namespace": 4026534045, "uts-namespace": 4026534043 }
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(unused)] // Keep structure in sync with bwrap events
pub struct SandboxStatus {
    pub child_pid: u32,
    pub cgroup_namespace: Option<u32>,
    pub ipc_namespace: Option<u32>,
    pub mnt_namespace: Option<u32>,
    pub net_namespace: Option<u32>,
    pub pid_namespace: Option<u32>,
    pub uts_namespace: Option<u32>,
}

impl EventType for SandboxStatus {}

// Example: {"exit-code": 0}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExitStatus {
    pub exit_code: i32,
}

impl EventType for ExitStatus {}

#[derive(Debug)]
pub struct EventsReader<T> {
    rx: BufReader<T>,
}

impl<T: std::io::Read> EventsReader<T> {
    pub fn new(rx: T) -> Self {
        let rx = BufReader::new(rx);
        Self { rx }
    }

    fn try_raw(&mut self) -> Result<String, AppError> {
        let mut line = String::new();

        let next = self
            .rx
            .read_line(&mut line)
            .map_err(AppError::io("bwrap status"))?;

        tracing::trace!("Bwrap raw event: {line}");

        if next == 0 {
            tracing::warn!("Unexpected EOF from bwrap --json-status-fd");
            return AppError::io("bwrap eof")(UnexpectedEof.into()).into_err();
        }

        Ok(line)
    }

    pub fn try_next<E: EventType>(&mut self) -> Result<E, AppError> {
        let raw = self.try_raw()?;
        let event: E = serde_json::from_str(&raw).map_err(AppError::BwrapEvent)?;
        Ok(event)
    }
}
