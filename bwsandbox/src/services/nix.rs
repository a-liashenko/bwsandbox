use crate::print_command::print_command;
use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use crate::{error::AppError, utils};
use serde::Deserialize;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    local_read_only: bool,
}

#[derive(Debug)]
pub struct NixMapper {
    command: Command,
    nix_bin: PathBuf,
}

impl NixMapper {
    // Keep interface consistent
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let _ = utils::which_bin(utils::NIX_STORE)?;
        let nix_bin = utils::temp_dir().join("nix-service-extra-bin-overlay");
        std::fs::create_dir(&nix_bin).map_err(AppError::io("failed to create nix overlay"))?;
        let mut command = Command::new(utils::NIX_STORE);

        // Allow db read without writing to nix .lock
        if config.local_read_only {
            command.arg("--extra-experimental-features");
            command.arg("read-only-local-store");
            command.arg("--store");
            command.arg("local?read-only=true");
        }

        command.arg("--query");
        command.arg("--requisites");

        Ok(Self { command, nix_bin })
    }

    fn exec(&mut self, bin: impl AsRef<OsStr>) -> Result<Vec<u8>, AppError> {
        let out = Command::new(utils::NIX_STORE)
            .args(self.command.get_args())
            .arg(bin)
            .output()
            .map_err(AppError::spawn(utils::NIX_STORE))?;

        if !out.status.success() {
            let stderr = std::str::from_utf8(&out.stderr);
            log::error!("Nix store stderr: {stderr:?}");
            return AppError::spawn(utils::NIX_STORE)(ErrorKind::Other.into()).into_err();
        }

        Ok(out.stdout)
    }
}

impl<C: Context> Service<C> for NixMapper {
    fn name(&self) -> &'static str {
        "nix-store automapper"
    }

    fn apply_before(&mut self, _: &mut C) -> Result<Scope, AppError> {
        Ok(Scope::new())
    }

    fn apply_after(&mut self, ctx: &mut C) -> Result<Scope, AppError> {
        let bin = ctx.bin().to_string_lossy();
        let bin = utils::which_bin(bin.as_ref())?;
        let bin_real = bin.canonicalize().map_err(AppError::file(&bin))?;
        if bin_real.starts_with("/nix") {
            let bin_parent = bin.parent().expect("Missing parent for nix exe");
            let bin_name = bin.file_name().expect("Missing nix app name");

            // Inject /nix/store/... path in place of binary via overlayfs to avoid readonly error fs if /usr was bound as read-only earlier
            std::os::unix::fs::symlink(bin_real, self.nix_bin.join(bin_name))
                .map_err(AppError::io("Failed to create nix app symlink"))?;
            ctx.command_mut().arg("--overlay-src").arg(bin_parent);
            ctx.command_mut().arg("--overlay-src").arg(&self.nix_bin);
            ctx.command_mut().arg("--ro-overlay").arg(bin_parent);

            let out = self.exec(bin)?;
            let out = std::str::from_utf8(&out).map_err(AppError::utf8("nix-store output"))?;
            for line in out.lines() {
                if !line.starts_with("/nix") {
                    log::warn!("Unexpected /nix/store path {line:?}");
                    return AppError::io("Unexpected nix-store output")(ErrorKind::Other.into())
                        .into_err();
                }
                ctx.command_mut().arg("--ro-bind").arg(line).arg(line);
            }
        }
        Ok(Scope::new())
    }

    fn start(self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        print_command(&self.command);
        Ok(HandleType::None)
    }
}
