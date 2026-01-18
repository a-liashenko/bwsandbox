mod scope;
mod service;

pub use scope::{Scope, ScopeCleanup};
pub use service::{Context, HandleOwned, Service};

// Allow unused to allow new services define own Handles if needed in the future
#[allow(unused)]
pub use service::Handle;
