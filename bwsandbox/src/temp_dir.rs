use crate::error::AppError;
use std::{os::unix::fs::PermissionsExt, path::Path};

#[derive(Debug)]
pub struct TempDirGuard {
    dir: &'static Path,
}

impl TempDirGuard {
    pub fn new(dir: &'static Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(dir).map_err(|e| AppError::TempDir(dir.into(), e))?;
        std::fs::set_permissions(dir, PermissionsExt::from_mode(0o700))
            .map_err(|e| AppError::TempDir(dir.into(), e))?;
        Ok(Self { dir })
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(self.dir) {
            log::error!("Failed to remove sandbox temp dir: {e:?}");
        }
    }
}
