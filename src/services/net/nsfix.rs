use crate::{error::AppError, services::BwrapInfo, system::Namespace};
use std::{io::ErrorKind, os::unix::process::CommandExt, process::Command};

// More info why it needed: https://github.com/rootless-containers/slirp4netns/issues/311
// If --dev used in bwrap it will create intermediate ns
// And to avoid setns(CLONE_NEWNET): Operation not permitted we need enter intermediate ns
// If --dev not used, parent ns will be our process ns

pub fn fix(service: &mut Command, info: &BwrapInfo, arg: &str) -> Result<(), AppError> {
    let own_ns = Namespace::open_pid(std::process::id())?;
    let bw_pns = Namespace::open_pid(info.sandbox.child_pid)?.parent()?;

    let dev_used = own_ns.fd_inode()? != bw_pns.fd_inode()?;
    if dev_used {
        service.arg(arg);
        unsafe {
            service.pre_exec(move || {
                let result = bw_pns.enter();
                tracing::trace!("pre_exec status {result:?}");
                result.map_err(|_| ErrorKind::Other.into())
            })
        };
    }
    Ok(())
}
