mod app_id;
mod config;

pub use config::Config;

use crate::error::AppError;
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use std::{ffi::CString, io::PipeWriter, os::unix::net::UnixListener};
use wayrs_client::Connection;
use wayrs_protocols::security_context_v1::{WpSecurityContextManagerV1, WpSecurityContextV1};

pub struct SecurityContextV1 {
    config: Config,
    conn: Connection<()>,
    context: WpSecurityContextV1,
    close_tx: PipeWriter,
}

impl SecurityContextV1 {
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        // Connect and fetch supported protocols
        let mut conn = wayrs_client::Connection::<()>::connect()?;
        conn.blocking_roundtrip()
            .map_err(AppError::io("wl roundtrip"))?;

        // Initialize security context protocol
        let manager: WpSecurityContextManagerV1 = conn.bind_singleton(1)?;

        // Pass "fake" wayland socket and rx pipe end for control
        let socket = UnixListener::bind(config.socket.as_inner())
            .map_err(AppError::io("wl bind security socket"))?;
        let (close_rx, close_tx) = std::io::pipe().map_err(AppError::io("wl control pipe"))?;

        let context: WpSecurityContextV1 =
            manager.create_listener(&mut conn, socket.into(), close_rx.into());

        Ok(Self {
            config,
            conn,
            context,
            close_tx,
        })
    }
}

impl<C: Context> Service<C> for SecurityContextV1 {
    fn name(&self) -> &'static str {
        "wl_security_context_v1"
    }

    fn apply_before(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        let engine = CString::new(self.config.sandbox_engine.clone())?;
        self.context.set_sandbox_engine(&mut self.conn, engine);

        let app_id = self.config.app_id.resolve(ctx.bin())?;
        self.context.set_app_id(&mut self.conn, app_id);

        Ok(Scope::new().remove_file(self.config.socket.as_inner()))
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        if let Some(mount) = &self.config.mount {
            ctx.command_mut()
                .arg("--bind")
                .arg(self.config.socket.as_inner())
                .arg(mount.as_inner());
        }
        Ok(Scope::new())
    }

    fn start(mut self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        self.context.commit(&mut self.conn);
        self.context.destroy(&mut self.conn);
        self.conn
            .flush(wayrs_client::IoMode::Blocking)
            .map_err(AppError::io("wl flush"))?;

        self.conn
            .blocking_roundtrip()
            .map_err(AppError::io("wl roundtrip after commit"))?;
        Ok(HandleType::new(self.close_tx))
    }
}
