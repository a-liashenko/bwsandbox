use std::{
    ffi::OsStr,
    path::PathBuf,
    process::{Child, Command, Output, Stdio},
};

pub trait OutputExtra {
    fn stdout_str(&self) -> &str;
    fn stderr_str(&self) -> &str;
}

impl OutputExtra for Output {
    fn stdout_str(&self) -> &str {
        std::str::from_utf8(&self.stdout).expect("Non utf8 stdout")
    }

    fn stderr_str(&self) -> &str {
        std::str::from_utf8(&self.stderr).expect("Non utf8 stderr")
    }
}

pub(super) fn cargo_spawn<T: AsRef<OsStr>>(args: Vec<T>) -> Result<Child, std::io::Error> {
    let mut command = cargo_command();
    let child = command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(child)
}

pub(super) fn cargo_spawn_out<T: AsRef<OsStr>>(args: Vec<T>) -> Result<Output, std::io::Error> {
    let child = cargo_spawn(args)?;
    child.wait_with_output()
}

pub(super) fn cargo_command() -> Command {
    let working_dir = working_dir();
    let mut cmd = Command::new("cargo");
    cmd.current_dir(working_dir).arg("run").arg("--");
    cmd.env("NO_COLOR", "1");
    cmd
}

pub(super) fn working_dir() -> PathBuf {
    let mut this_dir = PathBuf::from(file!());
    this_dir.pop();

    let mut root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root_dir.pop();

    root_dir.join(this_dir)
}
