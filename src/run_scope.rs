use crate::error::Error;
use std::{
    collections::BTreeMap,
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct RunScope {
    prefix: String,
    dir: PathBuf,
    files: BTreeMap<PathBuf, std::fs::File>,
}

impl RunScope {
    pub fn new(prefix: impl Into<String>) -> Self {
        let temp_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or(std::env::temp_dir());

        Self {
            dir: temp_dir,
            prefix: prefix.into(),
            files: Default::default(),
        }
    }

    pub fn scope_dir(&self) -> &Path {
        &self.dir
    }

    pub fn new_file_scoped(&mut self, ctx: &str) -> Result<(PathBuf, &mut File), Error> {
        let (path, file) = self.new_file(ctx)?;
        self.add_file(path.clone(), file);

        let file = self.files.get_mut(&path).unwrap();
        Ok((path, file))
    }

    pub fn new_file(&mut self, ctx: &str) -> Result<(PathBuf, File), Error> {
        let name = std::iter::repeat_with(fastrand::alphanumeric)
            .take(8)
            .collect::<String>();
        let name = format!("sandbox-{}-{}-{}", self.prefix, ctx, name);

        let path = self.dir.join(name);
        let file = File::create_new(&path).map_err(Error::file(&path))?;
        Ok((path, file))
    }

    pub fn add_file(&mut self, path: PathBuf, file: File) {
        self.files.insert(path, file);
    }
}

impl Drop for RunScope {
    fn drop(&mut self) {
        let files = std::mem::take(&mut self.files);
        for (path, file) in files.into_iter() {
            drop(file);
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("Failed to cleanup files at exit {:?}: {}", path, e);
            }
        }
    }
}
