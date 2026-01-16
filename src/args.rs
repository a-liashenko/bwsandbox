use crate::{error::AppError, utils};
use lexopt::{Parser, ValueExt};
use std::{
    ffi::OsString,
    os::fd::{FromRawFd, RawFd},
    path::PathBuf,
};

const CONFIG_DIR: &str = "XDG_CONFIG_HOME";

#[derive(Debug)]
pub struct InternalArgs<I> {
    pub ready_fd: RawFd,
    pub args: I,
}

impl<I: Iterator<Item = OsString>> InternalArgs<I> {
    pub fn from_iter(mut args: I) -> Result<Self, AppError> {
        let internal = args.next().ok_or(AppError::BadArgs)?;
        if internal != utils::SELF_INTERNAL_ARG {
            tracing::error!("Unexpected first argument for internal launch: {internal:?}");
            return Err(AppError::BadArgs);
        }

        let ready_fd = args.next().ok_or(AppError::BadArgs)?;
        let ready_fd = ready_fd.parse::<i32>()?;
        let ready_fd = unsafe { RawFd::from_raw_fd(ready_fd) };

        Ok(Self { ready_fd, args })
    }
}

#[derive(Debug)]
pub struct Args {
    pub app: OsString,
    #[allow(clippy::struct_field_names)]
    pub app_args: Vec<OsString>,
    pub is_app_image: bool,
    pub config: String,
}

impl Args {
    #[tracing::instrument(skip(iter))]
    pub fn from_iter(iter: impl Iterator<Item = OsString>) -> Result<Self, AppError> {
        use lexopt::prelude::{Long, Short, Value};

        let mut config: Option<String> = None;
        let mut config_auto = false;
        let mut rest = Vec::new();
        let mut is_app_image = false;

        let mut parser = Parser::from_args(iter);
        while let Some(arg) = parser.next()? {
            match arg {
                Long("appimage") => is_app_image = true,
                Short('f') | Long("config-file") => config = Some(parse_file(&mut parser)?),
                Short('n') | Long("config-name") => config = Some(parse_name(&mut parser)?),
                Short('a') | Long("config-auto") => config_auto = true,
                Value(v) => rest.push(v),
                _ => return Err(arg.unexpected().into()),
            }
        }

        if config.is_none() && !config_auto {
            return Err(AppError::BadArgs);
        }

        if rest.is_empty() {
            return Err(AppError::BadArgs);
        }

        let app_name = rest.remove(0);
        if config_auto {
            config = Some(from_auto(app_name.clone())?);
        }

        Ok(Self {
            app: app_name,
            app_args: rest,
            is_app_image,
            config: config.expect("Config must be ready"),
        })
    }
}

#[tracing::instrument]
fn parse_file(parser: &mut Parser) -> Result<String, AppError> {
    let path = parser.value()?;
    let content = std::fs::read_to_string(&path).map_err(AppError::file(&path))?;
    Ok(content)
}

#[tracing::instrument]
fn parse_name(parser: &mut Parser) -> Result<String, AppError> {
    let mut name = parser.value()?;
    name.push(".toml");

    from_name(name)
}

#[tracing::instrument]
fn from_name(name: OsString) -> Result<String, AppError> {
    let config_dir = std::env::var(CONFIG_DIR).map_err(AppError::env(CONFIG_DIR))?;
    let config_path = PathBuf::from(&config_dir).join(utils::APP_NAME).join(name);
    let content = std::fs::read_to_string(&config_path).map_err(AppError::file(&config_path))?;
    Ok(content)
}

#[tracing::instrument]
fn from_auto(mut app_name: OsString) -> Result<String, AppError> {
    app_name.push(".toml");
    from_name(app_name)
}
