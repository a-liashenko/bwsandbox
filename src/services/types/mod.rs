mod scope;
mod service;

pub use scope::{Scope, ScopeCleanup};
pub use service::{BwrapInfo, Context, HandleExt, HandleType, Service};

// Allow unused to allow new services define own Handles if needed in the future
#[allow(unused)]
pub use service::Handle;
