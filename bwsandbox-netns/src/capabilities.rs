use crate::error::AppError;
use rustix::thread::{CapabilitySet, CapabilitySets};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Capabilities {
    original: CapabilitySets,
    required: CapabilitySet,
}

impl Capabilities {
    pub fn new(required: CapabilitySet) -> Result<Self, AppError> {
        let original = rustix::thread::capabilities(None).map_err(AppError::CapsGet)?;
        crate::trace::trace!("Original caps: {original:#?}");

        if !original.permitted.contains(required) {
            return Err(AppError::CapsMissing(required));
        }

        Ok(Self { original, required })
    }

    pub fn elevate(self) -> Result<CapabilitiesGuard, AppError> {
        if self.original.effective.contains(self.required) {
            // Already have required capabilities
            return Ok(CapabilitiesGuard::new());
        }

        let mut caps = self.original;
        caps.effective |= self.required;
        rustix::thread::set_capabilities(None, caps).map_err(AppError::caps_set(caps))?;

        Ok(CapabilitiesGuard::new())
    }
}

#[derive(Debug)]
pub struct CapabilitiesGuard {
    _data: PhantomData<()>,
}

impl CapabilitiesGuard {
    fn new() -> Self {
        Self { _data: PhantomData }
    }
}

impl Drop for CapabilitiesGuard {
    fn drop(&mut self) {
        let empty = CapabilitySets {
            effective: CapabilitySet::empty(),
            permitted: CapabilitySet::empty(),
            inheritable: CapabilitySet::empty(),
        };

        if let Err(e) = rustix::thread::set_capabilities(None, empty) {
            crate::trace::error!("Failed to drop all capabilities: {e:?}");
            std::process::exit(-1);
        }
    }
}
