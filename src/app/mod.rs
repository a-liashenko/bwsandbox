use crate::{config::AppConfig, error::Error, models::template::TemplateConfig, paths};
use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, ExitStatus},
};

pub use dbus::Dbus;
pub use seccomp::Seccomp;

mod dbus;
mod seccomp;

#[derive(Debug)]
pub struct App {
    pub seccomp: Option<Seccomp>,
    pub dbus: Option<Dbus>,
    pub bwrap: Command,
}

impl App {
    pub fn new<App, Args, C>(app: App, app_args: Args, config: C) -> Result<Self, Error>
    where
        App: AsRef<Path>,
        Args: Iterator<Item = String>,
        C: AsRef<Path>,
    {
        let app_name = app.as_ref().file_name().and_then(OsStr::to_str);
        let app_name = app_name.ok_or_else(|| anyhow::anyhow!("Missing app"))?;

        let config = AppConfig::load(config)?;
        let mut bwrap = new_bwrap(config.template)?;

        let seccomp = config.seccomp.map(|cfg| -> Result<_, Error> {
            let path = paths::temp_file(app_name, "seccomp");
            let seccomp = Seccomp::new(cfg, path)?;
            bwrap.arg("--seccomp").arg(seccomp.fd().to_string());
            Ok(seccomp)
        });

        let dbus = config.dbus.map(|cfg| -> Result<_, Error> {
            let socket = paths::temp_file(app_name, "dbus");
            let mounted = paths::xdg_runtime_dir()?.join("bus");

            let dbus = Dbus::new(cfg, socket.clone())?;
            bwrap.arg("--bind").arg(socket).arg(mounted);
            Ok(dbus)
        });

        bwrap.arg(app.as_ref()).args(app_args);

        Ok(Self {
            bwrap,
            dbus: dbus.transpose()?,
            seccomp: seccomp.transpose()?,
        })
    }

    pub fn run_app(mut self) -> Result<ExitStatus, Error> {
        // Cleanup will be called on drop
        let _seccomp = self.seccomp.take();
        let _dbus = self.dbus.map(|v| v.spawn()).transpose()?;

        let mut bwrap = self.bwrap.spawn().map_err(Error::spawn("app"))?;
        let status = bwrap.wait().map_err(Error::spawn("app"))?;
        Ok(status)
    }
}

fn new_bwrap(cfg: TemplateConfig) -> Result<Command, Error> {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_templates_directory(cfg.include.as_inner(), Default::default())?;
    let args = handlebars.render(&cfg.name, &cfg.context)?;
    let args = shlex::Shlex::new(&args);

    let mut command = Command::new(cfg.bin);
    command.args(args);
    Ok(command)
}
