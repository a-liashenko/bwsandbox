use crate::error::Error;
use std::path::PathBuf;

// TODO: Use OnceLock or ThreadLocal

macro_rules! define_var_fn {
    ($func: ident, $var_name: literal, $var: ty) => {
        pub fn $func() -> Result<$var, Error> {
            use std::env::var;
            const ERROR_TEXT: &str = concat!("Missing env variable: ", $var_name);
            let v = var($var_name).map_err(|_| anyhow::anyhow!(ERROR_TEXT))?;
            Ok(<$var>::from(v))
        }
    };
}

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_PREFIX: &str = APP_NAME;

define_var_fn!(xdg_config_home, "XDG_CONFIG_HOME", PathBuf);
define_var_fn!(xdg_runtime_dir, "XDG_RUNTIME_DIR", PathBuf);

pub fn temp_file(app: &str, ctx: &str) -> PathBuf {
    let rand = std::iter::repeat_with(fastrand::alphanumeric).take(6);
    let filename = format!("{APP_PREFIX}-{app}-{ctx}-{}", rand.collect::<String>());

    let mut dir = xdg_runtime_dir().unwrap_or(std::env::temp_dir());
    dir.push(filename);
    dir
}
