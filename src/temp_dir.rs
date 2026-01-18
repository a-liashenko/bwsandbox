use crate::error::AppError;
use std::path::PathBuf;

#[derive(Debug)]
pub struct TempDirGuard {
    dir: PathBuf,
}

impl TempDirGuard {
    pub fn new(dir: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&dir).map_err(|e| AppError::TempDir(dir.clone(), e))?;
        Ok(Self { dir })
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.dir) {
            tracing::error!("Failed to remove sandbox temp dir: {e:?}");
        }
    }
}
