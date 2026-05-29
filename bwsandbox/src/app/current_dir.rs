use crate::error::AppError;
use std::path::Path;

pub fn run_in_dir<O, F>(dir: &Path, f: F) -> Result<O, AppError>
where
    F: FnOnce() -> Result<O, AppError>,
{
    let old = std::env::current_dir().map_err(AppError::io("Failed to get current dir"))?;

    std::env::set_current_dir(dir).map_err(AppError::io("Failed to set current dir"))?;
    let res = f();
    std::env::set_current_dir(&old).map_err(AppError::io("Failed to restore current dir"))?;

    res
}
