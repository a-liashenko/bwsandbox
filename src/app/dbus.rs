use crate::{error::Error, models::dbus::DbusConfig};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
};

#[derive(Debug)]
pub struct Dbus {
    socket: PathBuf,
    command: Command,
}

impl Dbus {
    pub fn new(cfg: DbusConfig, socket: PathBuf) -> Result<Self, Error> {
        let talk = cfg.talk.iter().map(|v| format!("--talk={v}"));
        let own = cfg.own.iter().map(|v| format!("--own={v}"));

        let mut command = Command::new(cfg.bin);
        command
            .arg(cfg.bus_address.into_inner())
            .arg(&socket)
            .arg("--filter")
            .args(talk)
            .args(own);

        Ok(Self { command, socket })
    }

    pub fn spawn(mut self) -> Result<DbusHandle, Error> {
        let child = self
            .command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .map_err(Error::spawn("DBus proxy"))?;

        self.wait_for_socket()?;

        Ok(DbusHandle {
            socket: self.socket,
            child,
        })
    }

    fn wait_for_socket(&self) -> Result<(), Error> {
        use std::thread::sleep;
        use std::time::Duration;

        const SLEEP: Duration = Duration::from_millis(100);
        const ATTEMPTS: u32 = 30; // 100ms * 30 = 3 seconds

        for _ in 0..ATTEMPTS {
            let exists = std::fs::exists(&self.socket).map_err(Error::file(&self.socket))?;
            if exists {
                return Ok(());
            }
            sleep(SLEEP);
        }

        Err(anyhow::anyhow!("DBus proxy not found {}", self.socket.display()).into())
    }
}

#[derive(Debug)]
pub struct DbusHandle {
    pub child: Child,
    pub socket: PathBuf,
}

impl Drop for DbusHandle {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
        let _ = self.child.kill();
    }
}
