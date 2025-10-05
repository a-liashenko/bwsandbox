use crate::utils::{APP_NAME, sandbox_id, temp_dir};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct TempFileVal(PathBuf);

impl TempFileVal {
    #[tracing::instrument]
    pub fn new(context: &str) -> Self {
        let name = format!("{APP_NAME}-{context}-{}", sandbox_id());
        Self(temp_dir().join(name))
    }

    pub fn as_inner(&self) -> &Path {
        &self.0
    }

    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl<'de> Deserialize<'de> for TempFileVal {
    #[tracing::instrument(skip_all)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(TempFileVal::new(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_tmp_file() {
        let first = TempFileVal::new("first");
        let second = TempFileVal::new("second");
        assert_ne!(first, second);

        let first2 = TempFileVal::new("first");
        assert_eq!(first, first2);
    }
}
