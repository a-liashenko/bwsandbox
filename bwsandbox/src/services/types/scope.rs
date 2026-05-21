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

    pub fn merge(&mut self, other: Scope) {
        if !other.is_empty() {
            self.remove.extend(other.remove);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.remove.is_empty()
    }

    fn cleanup(self) {
        let _guard = tracing::error_span!("[scope cleanup]").entered();
        for it in self.remove {
            if let Err(e) = std::fs::remove_file(&it) {
                tracing::error!("Failed to remove file {it:?}: {e}");
            }
        }
    }
}

#[derive(Debug)]
pub struct ScopeCleanup {
    scopes: Vec<Scope>,
}

impl ScopeCleanup {
    pub fn new(scopes: Vec<Scope>) -> Self {
        Self { scopes }
    }
}

impl Drop for ScopeCleanup {
    fn drop(&mut self) {
        tracing::trace!("Removing all scoped resources: {:?}", self.scopes);
        let scopes = std::mem::take(&mut self.scopes);
        scopes.into_iter().for_each(Scope::cleanup);
    }
}
