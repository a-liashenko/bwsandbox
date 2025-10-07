use crate::{error::AppError, service::Scope};
use std::sync::{Arc, Mutex};

type ScVec = Arc<Mutex<Option<Vec<Scope>>>>;

#[derive(Debug)]
pub struct ScopeDestroyer {
    scopes: ScVec,
}

impl ScopeDestroyer {
    pub fn new(scopes: Vec<Scope>) -> Result<Self, AppError> {
        let scopes = Arc::new(Mutex::new(Some(scopes)));

        let scopes_lnk = scopes.clone();
        let sig_handle = move || {
            tracing::info!("----- Called Ctrl + C");
            if let Some(scopes) = take_scopes(&scopes_lnk) {
                destroy(scopes.into_iter());
            }
        };
        ctrlc::set_handler(sig_handle)?;

        Ok(Self { scopes })
    }
}

impl Drop for ScopeDestroyer {
    fn drop(&mut self) {
        if let Some(scopes) = take_scopes(&self.scopes) {
            destroy(scopes.into_iter());
        }
    }
}

#[tracing::instrument]
fn take_scopes(scopes: &ScVec) -> Option<Vec<Scope>> {
    let scopes = scopes.lock().map(|mut v| v.take());
    match scopes {
        Ok(Some(v)) => return Some(v),
        Ok(None) => tracing::trace!("Scopes already killed?"),
        Err(e) => tracing::error!("Scopes mutex poisoned {e:?}"),
    }
    None
}

#[tracing::instrument(skip_all)]
fn destroy(iter: impl Iterator<Item = Scope>) {
    let files = iter.flat_map(|v| v.remove.into_iter());
    for file in files {
        if let Err(e) = std::fs::remove_file(&file) {
            tracing::warn!("Failed to remove {file:?}, err {e}");
        }
    }
}
