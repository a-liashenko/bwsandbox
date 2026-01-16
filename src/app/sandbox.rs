use crate::{
    app::scope_destroyer::ScopeDestroyer,
    error::AppError,
    fd::AsFdExtra,
    service::{Context, Scope, Service},
    utils,
};
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::Write,
    os::fd::{FromRawFd, IntoRawFd},
    process::{Child, Command},
};

#[derive(Debug)]
pub struct SandboxBuilder {
    bin: Command,
    command_args: Vec<OsString>,
    scope: Scope,
    ready_fd: File,
}

impl Context for SandboxBuilder {
    fn command_mut(&mut self) -> &mut Command {
        &mut self.bin
    }
}

impl SandboxBuilder {
    pub fn new(bin: impl Into<OsString>, command_args: Vec<OsString>) -> Result<Self, AppError> {
        let (ready_rx, ready_tx) = rustix::pipe::pipe().map_err(AppError::PipeAlloc)?;
        ready_rx.share_with_children()?;

        let mut bin = Command::new(bin.into());
        bin.arg(utils::SELF_INTERNAL_ARG);
        bin.arg(ready_rx.into_raw_fd().to_string());

        let ready_fd = unsafe { File::from_raw_fd(ready_tx.into_raw_fd()) };

        Ok(Self {
            bin,
            command_args,
            ready_fd,
            scope: Scope::new(),
        })
    }

    pub fn apply_before<S: Service>(&mut self, service: &mut S) -> Result<(), AppError> {
        let scope = service.apply_before(self)?;
        self.scope.merge(scope);
        Ok(())
    }

    pub fn prebuild(&mut self) {
        let args = std::mem::take(&mut self.command_args);
        self.bin.args(args);
    }

    pub fn apply_after<S: Service>(&mut self, service: &mut S) -> Result<(), AppError> {
        assert!(
            self.command_args.is_empty(),
            "prebuild() must be called before apply_after()"
        );

        let scope = service.apply_after(self)?;
        self.scope.merge(scope);
        Ok(())
    }

    pub fn build<A, I>(mut self, app: A, args: I) -> Result<Sandbox, AppError>
    where
        A: AsRef<OsStr>,
        I: Iterator<Item = OsString>,
    {
        let cleanup = ScopeDestroyer::new(vec![self.scope])?;

        self.bin.arg(app).args(args);
        Ok(Sandbox::new(self.bin, self.ready_fd, cleanup))
    }
}

#[derive(Debug)]
pub struct Sandbox {
    cmd: Option<Command>,
    ready_fd: File,

    // Will clean everything on drop
    _cleanup: ScopeDestroyer,
}

impl Sandbox {
    fn new(cmd: Command, ready_fd: File, cleanup: ScopeDestroyer) -> Self {
        Self {
            cmd: Some(cmd),
            ready_fd: ready_fd,
            _cleanup: cleanup,
        }
    }

    pub fn get_command(&self) -> Option<&Command> {
        self.cmd.as_ref()
    }

    pub fn start(&mut self) -> Result<Child, AppError> {
        let mut cmd = self.cmd.take().expect("Sandbox can be started only once");
        let child = cmd.spawn().map_err(AppError::spawn(utils::SELF_CMD))?;
        Ok(child)
    }

    pub fn notify_ready(&mut self) -> Result<(), AppError> {
        self.ready_fd
            .write(&[1])
            .map_err(AppError::file("__ready_fd__"))?;
        Ok(())
    }
}
