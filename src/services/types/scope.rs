use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::AppError;

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
        self.remove.extend(other.remove);
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
    scopes: Arc<Mutex<Vec<Scope>>>,
}

impl ScopeCleanup {
    pub fn new(scopes: Vec<Scope>) -> Result<Self, AppError> {
        let scopes = Arc::new(Mutex::new(scopes));

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
fn destroy_scopes(scopes: &Arc<Mutex<Vec<Scope>>>) {
    let scopes = scopes.lock().map(|mut v| std::mem::take(&mut *v));
    let scopes = match scopes {
        Ok(v) if v.is_empty() => {
            tracing::warn!("Scopes already cleaned?");
            std::process::exit(-1);
        }
        Err(e) => {
            tracing::error!("Scopes mutex poisoned: {e:?}");
            std::process::exit(-1);
        }
        Ok(v) => v,
    };

    scopes.into_iter().for_each(Scope::cleanup);
}
