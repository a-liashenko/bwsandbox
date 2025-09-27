use app::App;
use error::Error;
use std::{ffi::OsString, path::PathBuf, process::ExitCode};

mod app;
mod config;
mod error;
mod models;
mod paths;
mod seccomp_ffi;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

fn main() -> ExitCode {
    let args = std::env::args_os().skip(1);
    let (config, mut app_args) = match parse_args(args) {
        Ok(v) => v,
        Err(e) => {
            on_help();
            return on_error(e);
        }
    };

    if app_args.is_empty() {
        return on_help();
    }

    let app_name = app_args.remove(0);
    let app = match App::new(app_name, app_args.into_iter(), config) {
        Ok(v) => v,
        Err(e) => return on_error(e),
    };

    match app.run_app() {
        Ok(v) => {
            let code = v.code().unwrap_or(-1);
            std::process::exit(code);
        }
        Err(e) => on_error(e),
    }
}

fn on_help() -> ExitCode {
    let help = format!("Usage example: {APP_NAME} --config config.toml -- app --arg 1",);
    println!("{help}");
    ExitCode::SUCCESS
}

fn on_error(e: Error) -> ExitCode {
    println!("{e:#?}");
    ExitCode::FAILURE
}

fn parse_args<I>(args: I) -> Result<(PathBuf, Vec<String>), Error>
where
    I: Iterator<Item = OsString>,
{
    use lexopt::prelude::*;
    let mut config_file: Option<String> = None;
    let mut config_name: Option<String> = None;
    let mut rest = vec![];

    let mut parser = lexopt::Parser::from_args(args);
    while let Some(arg) = parser.next()? {
        match arg {
            Short('f') | Long("config-file") => {
                config_file = Some(parser.value()?.string()?);
            }
            Short('n') | Long("config-name") => {
                config_name = Some(parser.value()?.string()?);
            }
            Value(v) => {
                rest.push(v.string()?);
            }
            _ => return Err(arg.unexpected().into()),
        }
    }

    let config = match (config_file, config_name) {
        (Some(config), _) => PathBuf::from(config),
        (_, Some(config)) => paths::xdg_config_home()?
            .join(paths::APP_NAME)
            .join(format!("{config}.toml")),
        _ => return Err(anyhow::anyhow!("Missing args").into()),
    };

    Ok((config, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_resolution() {
        let config_name = "profile-test";
        let config_dir = "/test/.config";
        let config = format!("{config_dir}/{APP_NAME}/{config_name}.toml");
        unsafe { std::env::set_var("XDG_CONFIG_HOME", config_dir) };

        let args = vec!["--config-file".into(), config.clone().into(), "sh".into()];
        let (cfg, _) = parse_args(args.into_iter()).unwrap();
        assert_eq!(cfg.display().to_string(), config);

        let args = vec!["--config-name".into(), config_name.into(), "sh".into()];
        let (cfg, _) = parse_args(args.into_iter()).unwrap();
        assert_eq!(cfg.display().to_string(), config);
    }

    #[test]
    fn test_app_args() {
        let cmd = &["sh", "--option", "value", "positional"];

        let args = &["-f", "f", "--"];
        let full_args = args.iter().chain(cmd).map(OsString::from);
        let (_, app_args) = parse_args(full_args).unwrap();
        assert_eq!(app_args, cmd);

        // Should return error because of missing --
        let args = &["-f", "f"];
        let full_args = args.iter().chain(cmd).map(OsString::from);
        let res = parse_args(full_args);
        assert!(res.is_err());
    }
}
