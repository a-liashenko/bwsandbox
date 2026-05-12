use crate::{error::AppError, utils};
use lexopt::Parser;
use std::{ffi::OsString, path::PathBuf};

const CONFIG_DIR: &str = "XDG_CONFIG_HOME";

#[derive(Debug)]
pub struct Args {
    pub app: OsString,
    #[allow(clippy::struct_field_names)]
    pub app_args: Vec<OsString>,
    pub config_dir: PathBuf,
    pub config: String,
}

impl Args {
    pub fn from_iter(iter: impl Iterator<Item = OsString>) -> Result<Self, AppError> {
        use lexopt::prelude::{Long, Short, Value};

        let mut config: Option<(PathBuf, String)> = None;
        let mut config_auto = false;
        let mut rest = Vec::new();

        let mut parser = Parser::from_iter(iter);
        while let Some(arg) = parser.next()? {
            match arg {
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

        let (config_file, config) = config.expect("Config must be ready");
        Ok(Self {
            app: app_name,
            app_args: rest,
            config_dir: config_file.parent().expect("Missing config home?").into(),
            config,
        })
    }
}

#[tracing::instrument(skip(parser))]
fn parse_file(parser: &mut Parser) -> Result<(PathBuf, String), AppError> {
    let path = parser.value()?;
    let content = std::fs::read_to_string(&path).map_err(AppError::file(&path))?;
    let path = PathBuf::from(&path)
        .canonicalize()
        .map_err(AppError::file(path))?;
    Ok((path, content))
}

#[tracing::instrument(skip(parser))]
fn parse_name(parser: &mut Parser) -> Result<(PathBuf, String), AppError> {
    let mut name = parser.value()?;
    name.push(".toml");

    from_name(name)
}

#[tracing::instrument]
fn from_name(name: OsString) -> Result<(PathBuf, String), AppError> {
    let config_dir = std::env::var(CONFIG_DIR).map_err(AppError::env(CONFIG_DIR))?;
    let config_path = PathBuf::from(&config_dir).join(utils::APP_NAME).join(name);
    let content = std::fs::read_to_string(&config_path).map_err(AppError::file(&config_path))?;
    Ok((config_path, content))
}

#[tracing::instrument]
fn from_auto(mut app_name: OsString) -> Result<(PathBuf, String), AppError> {
    app_name.push(".toml");
    from_name(app_name)
}
