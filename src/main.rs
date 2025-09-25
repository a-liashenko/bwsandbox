use app::App;
use error::Error;
use std::process::{ExitCode, Stdio};

mod app;
mod config;
mod error;
mod models;
mod run_scope;
mod seccomp_ffi;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    if args.len() < 4 {
        return on_help();
    }

    let config_opt = args.next().unwrap(); // 1
    let config_val = args.next().unwrap(); // 2
    if config_opt != "--config" {
        return on_help();
    }

    let seaparator = args.next().unwrap(); // 3
    if seaparator != "--" {
        return on_help();
    }

    let app_name = args.next().unwrap(); // 4
    let app = match App::new(app_name, args, config_val) {
        Ok(v) => v,
        Err(e) => return on_error(e),
    };

    let code = match start_sandbox(app) {
        Ok(v) => v,
        Err(e) => return on_error(e),
    };

    code
}

fn on_help() -> ExitCode {
    let exe = std::env::current_exe()
        .map(|v| {
            v.file_name()
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or("app".into())
        })
        .unwrap_or("app".into());
    let help = format!("Usage example: {} --config config.toml -- app --arg 1", exe);
    println!("{help}");

    ExitCode::FAILURE
}

fn on_error(e: Error) -> ExitCode {
    println!("{e:#?}");
    ExitCode::FAILURE
}

fn start_sandbox(mut app: App) -> Result<ExitCode, Error> {
    let dbus = app
        .dbus
        .map(|mut v| v.stdin(Stdio::null()).stdout(Stdio::null()).spawn())
        .transpose()
        .map_err(Error::spawn("DBus proxy"))?;

    let mut app_runner = move || -> Result<ExitCode, Error> {
        let mut bwrap = app.bwrap.spawn().map_err(Error::spawn("App"))?;
        let status = bwrap.wait().map_err(Error::spawn("App"))?;

        let code = if status.success() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
        Ok(code)
    };

    let app_code = app_runner();
    let _dbus_err = dbus.and_then(|mut v| v.kill().err());
    Ok(app_code?)
}
