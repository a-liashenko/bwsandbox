use crate::error::AppError;
use std::{io::ErrorKind, path::PathBuf, sync::LazyLock};

static PATHS: LazyLock<Vec<PathBuf>> = LazyLock::new(fetch_paths);

fn fetch_paths() -> Vec<PathBuf> {
    let path = match std::env::var("PATH") {
        Ok(v) => v,
        Err(e) => {
            log::warn!("Bad $PATH: {e:?}");
            return Vec::new();
        }
    };
    path.split(':').map(PathBuf::from).collect()
}

fn find_path(paths: &[PathBuf], name: &str) -> Result<PathBuf, AppError> {
    for it in paths {
        let file = it.join(name);
        if file.exists() {
            return Ok(file);
        }
    }

    AppError::file(name)(ErrorKind::NotFound.into()).into_err()
}

pub fn which_bin(name: &str) -> Result<PathBuf, AppError> {
    let path = find_path(&PATHS, name)?;
    if !path.is_file() {
        return AppError::file(path)(ErrorKind::NotFound.into()).into_err();
    }
    Ok(path)
}

#[test]
fn test_which_bin() {
    let v = which_bin("NO_SUCH_FILE_FOR_SURE");
    assert!(v.is_err());

    let expected = PathBuf::from("/usr/bin/true").canonicalize().unwrap();
    let bin = which_bin("true").expect("Missing 'true'");
    assert_eq!(bin, expected);
}
