use crate::services::{BwrapInfo, Context, HandleType, Scope, Service};
use crate::{error::AppError, utils};
use bin::NixBin;
use serde::Deserialize;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

mod bin;
mod cmd;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    local_read_only: bool,
}

#[derive(Debug)]
pub struct NixMapper {
    config: Config,
    tmp_bin: PathBuf,
}

impl NixMapper {
    // Keep interface consistent
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_config(config: Config) -> Result<Self, AppError> {
        let _ = utils::which_bin(utils::NIX_STORE)?;
        let tmp_bin = utils::temp_dir().join("nix-service-extra-bin-overlay");
        std::fs::create_dir(&tmp_bin).map_err(AppError::io("failed to create nix overlay"))?;

        Ok(Self { config, tmp_bin })
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
        let bin = utils::which_bin(&bin)?;
        let bin_nix = NixBin::new(&bin)?;
        if !bin_nix.is_nix() {
            return Ok(Scope::new());
        }

        // Link /nix/store/...<bin> into working dir for future overlay
        // Direct --ro-bind not possible, if binary comes from /usr/bin and it was mounted as ro before
        let bin_name = bin.file_name().expect("Missing sandbox app name?");
        symlink(bin_nix.readlink(), self.tmp_bin.join(bin_name)).map_err(AppError::io(
            "Failed to symlink nix store binary into working dir",
        ))?;

        let parent = bin.parent().expect("Missing sandboxed app parent dir?");

        // Mount symlink to real store location of binary to avoid errors of multiple nested symlinks
        // Use overlay to keep all other binaiers in parent dir intact (case for /usr/bin symlink to nix)
        ctx.command_mut().arg("--overlay-src").arg(parent);
        ctx.command_mut().arg("--overlay-src").arg(&self.tmp_bin);
        ctx.command_mut().arg("--ro-overlay").arg(parent);

        let deps = bin_nix.list_deps(self.config.local_read_only)?;
        for it in deps.iter() {
            let it = it?;
            ctx.command_mut().arg("--ro-bind").arg(it).arg(it);
        }

        Ok(Scope::new())
    }

    fn start(self: Box<Self>, _: &BwrapInfo) -> Result<HandleType, AppError> {
        Ok(HandleType::None)
    }
}
