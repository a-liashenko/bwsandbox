use crate::error::AppError;

pub trait Handle: std::fmt::Debug {
    fn stop(&mut self) -> Result<(), AppError>;
}

// New type to force all services to use extra .spawn_service() which wraps Child into HandleType
// Constructor is private to avoid zombie process if error occured in between spawn() and return HandleType::new(child)
#[derive(Debug)]
#[repr(transparent)]
pub struct ChildHandle(Option<std::process::Child>);

impl ChildHandle {
    pub(super) fn new(child: std::process::Child) -> Self {
        Self(Some(child))
    }
}

impl Handle for ChildHandle {
    fn stop(&mut self) -> Result<(), AppError> {
        let Some(mut child) = std::mem::take(&mut self.0) else {
            return Ok(());
        };

        if let Err(e) = child.kill() {
            log::error!("Failed to kill service child: {e:?}");
        }

        if let Err(e) = child.wait() {
            log::error!("Failed to wait for service child exit: {e:?}");
        }

        Ok(())
    }
}

impl Drop for ChildHandle {
    fn drop(&mut self) {
        let status = self.stop();
        if let Err(e) = status {
            log::error!("Failed to stop service: {e:?}");
        }
    }
}

// Do nothing, file will be closed on exit
impl Handle for std::fs::File {
    fn stop(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}

impl Handle for Box<dyn Handle> {
    fn stop(&mut self) -> Result<(), AppError> {
        self.as_mut().stop()
    }
}

#[derive(Debug)]
pub struct HandleOwned {
    handle: Box<dyn Handle>,
}

impl HandleOwned {
    fn new<H: Handle + 'static>(handle: H) -> Self {
        let handle = Box::new(handle);
        Self { handle }
    }
}

impl Drop for HandleOwned {
    fn drop(&mut self) {
        if let Err(e) = self.handle.stop() {
            log::error!("Failed to stop service with {e:?}");
        }
    }
}

#[derive(Debug)]
pub enum HandleType {
    None,
    Owned { _drop: HandleOwned },
}

impl HandleType {
    pub fn new<T: Handle + 'static>(handle: T) -> Self {
        Self::Owned {
            _drop: HandleOwned::new(handle),
        }
    }
}
