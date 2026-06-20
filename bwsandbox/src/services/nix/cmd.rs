use crate::{error::AppError, utils};
use std::{io::ErrorKind, path::Path, process::Command};

pub fn list_deps(bin: &Path, with_ro: bool) -> Result<String, AppError> {
    let out = list_deps_cmd(bin, with_ro)
        .output()
        .map_err(AppError::spawn(utils::NIX_STORE))?;

    if !out.status.success() {
        let stderr = std::str::from_utf8(&out.stderr);
        log::error!("Nix store stderr: {stderr:?}");
        return AppError::spawn(utils::NIX_STORE)(ErrorKind::Other.into()).into_err();
    }

    let stdout = String::from_utf8(out.stdout).map_err(AppError::utf8("nix-store output"))?;
    Ok(stdout)
}

fn list_deps_cmd(bin: &Path, with_ro: bool) -> Command {
    let mut command = Command::new(utils::NIX_STORE);
    if with_ro {
        command.arg("--extra-experimental-features");
        command.arg("read-only-local-store");

        command.arg("--store");
        command.arg("local?read-only=true");
    }

    command.arg("--query").arg("--requisites");
    command.arg(bin);
    command
}
