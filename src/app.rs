use crate::{
    config::AppConfig,
    error::Error,
    models::{dbus::DbusConfig, seccomp::SeccompConfig, template::TemplateConfig},
    run_scope::RunScope,
    seccomp_ffi::FilterCtx,
};
use std::{ffi::OsStr, os::fd::AsRawFd, path::Path, process::Command};

#[derive(Debug)]
pub struct App {
    pub bwrap: Command,
    pub dbus: Option<Command>,
    pub _scope: RunScope,
}

impl App {
    pub fn new<App, Args, C>(app: App, app_args: Args, config: C) -> Result<Self, Error>
    where
        App: AsRef<Path>,
        Args: Iterator<Item = String>,
        C: AsRef<Path>,
    {
        let app_name = match app.as_ref().file_name().and_then(OsStr::to_str) {
            Some(v) => v,
            None => return Err(anyhow::anyhow!("Missing filename").into()),
        };
        let mut scope = RunScope::new(app_name);

        let config = AppConfig::load(config)?;
        let mut bwrap = new_bwrap(config.template)?;

        let seccomp = config.seccomp.map(new_seccomp).transpose()?;
        if let Some(seccomp) = seccomp {
            use nix::fcntl::{F_SETFD, FdFlag, fcntl};
            let (path, mut file) = scope.new_file("seccomp")?;
            seccomp.export_bpf(&mut file)?;

            let file = std::fs::File::open(&path).map_err(Error::file(&path))?;
            fcntl(&file, F_SETFD(FdFlag::empty()))
                .map_err(|e| anyhow::anyhow!("fcntl failed: {e:?}"))?;
            bwrap.arg("--seccomp").arg(file.as_raw_fd().to_string());
            scope.add_file(path, file);
        }

        let dbus = config
            .dbus
            .map(|cfg| -> Result<_, Error> {
                let (proxy_socket, _) = scope.new_file_scoped("dbus-proxy")?;
                let dbus = new_dbus_proxy(cfg, &proxy_socket)?;

                let mounted = scope.scope_dir().join("bus");
                bwrap.arg("--bind").arg(&proxy_socket).arg(mounted);
                Ok(dbus)
            })
            .transpose()?;

        bwrap.arg(app.as_ref()).args(app_args);

        Ok(Self {
            bwrap,
            dbus,
            _scope: scope,
        })
    }
}

fn new_seccomp(cfg: SeccompConfig) -> Result<FilterCtx, Error> {
    let mut ctx = FilterCtx::new(cfg.default_action)?;

    for arch in &cfg.extra_arch {
        ctx.arch_add(*arch)?;
    }

    for rule in &cfg.rules {
        for syscall in &rule.syscalls {
            ctx.rule_add(rule.action, *syscall)?;
        }
    }

    Ok(ctx)
}

fn new_dbus_proxy(cfg: DbusConfig, socket: &Path) -> Result<Command, Error> {
    let talk = cfg.talk.iter().map(|v| format!("--talk={v}"));
    let own = cfg.own.iter().map(|v| format!("--own={v}"));

    let mut command = Command::new(cfg.bin);
    command
        .arg(cfg.bus_address.into_inner())
        .arg(socket)
        .arg("--filter")
        .args(talk)
        .args(own);
    Ok(command)
}

fn new_bwrap(cfg: TemplateConfig) -> Result<Command, Error> {
    let mut handlebars = handlebars::Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_templates_directory(cfg.include, Default::default())?;
    let args = handlebars.render(&cfg.name, &cfg.context)?;
    let args = shlex::Shlex::new(&args).into_iter();

    let mut command = Command::new(cfg.bin);
    command.args(args);
    Ok(command)
}
