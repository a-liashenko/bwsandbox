use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Scope {
    pub remove: BTreeSet<PathBuf>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.remove.insert(file.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.remove.is_empty()
    }

    fn cleanup(self) {
        for it in self.remove {
            if let Err(e) = std::fs::remove_file(&it) {
                log::error!("Failed to remove file {}: {e}", it.display());
            }
        }
    }
}

#[derive(Debug)]
pub struct ScopeCleanup {
    scopes: Vec<Scope>,
}

impl ScopeCleanup {
    pub fn new(size: usize) -> Self {
        Self {
            scopes: Vec::with_capacity(size),
        }
    }

    pub fn push(&mut self, scope: Scope) {
        if !scope.is_empty() {
            self.scopes.push(scope);
        }
    }
}

impl Drop for ScopeCleanup {
    fn drop(&mut self) {
        log::trace!("Drop scopes: {:#?}", self.scopes);
        let scopes = std::mem::take(&mut self.scopes);
        scopes.into_iter().for_each(Scope::cleanup);
    }
}
