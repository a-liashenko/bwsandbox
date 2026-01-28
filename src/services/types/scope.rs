use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::AppError;

type SharedScopes = Arc<Mutex<Option<Vec<Scope>>>>;

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
        let _guard = tracing::info_span!("[scope cleanup]").entered();
        for it in self.remove {
            if let Err(e) = std::fs::remove_file(&it) {
                tracing::error!("Failed to remove file {it:?}: {e}");
            }
        }
    }
}

#[derive(Debug)]
pub struct ScopeCleanup {
    scopes: SharedScopes,
}

impl ScopeCleanup {
    pub fn new(scopes: Vec<Scope>) -> Result<Self, AppError> {
        let scopes = Arc::new(Mutex::new(Some(scopes)));

        let scopes_arc = Arc::clone(&scopes);
        let sig_handle = move || {
            tracing::info!("----- Called Ctrl + C");
            destroy_scopes(&scopes_arc);
        };
        ctrlc::set_handler(sig_handle)?;

        Ok(Self { scopes })
    }
}

impl Drop for ScopeCleanup {
    fn drop(&mut self) {
        let scopes = std::mem::take(&mut self.scopes);
        destroy_scopes(&scopes);
    }
}

#[tracing::instrument]
fn destroy_scopes(scopes: &SharedScopes) {
    let scopes = scopes.lock().map(|mut v| v.take());
    let scopes = match scopes {
        Ok(Some(v)) => v,
        Ok(None) => {
            tracing::warn!("Scopes already cleaned? Signal/exit data race?");
            std::process::exit(-1);
        }
        Err(e) => {
            tracing::error!("Scopes mutex poisoned: {e:?}");
            std::process::exit(-1);
        }
    };

    scopes.into_iter().for_each(Scope::cleanup);
}
