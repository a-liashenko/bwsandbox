use crate::error::AppError;
use crate::services::BwrapInfo;
use crate::system::{Namespace, NamespaceType};
use std::{io::ErrorKind, os::unix::process::CommandExt, process::Command};

// More info why it needed: https://github.com/rootless-containers/slirp4netns/issues/311
// If --dev is used in bwrap, it creates an intermediate user namespace to handle device mounting.
// pasta and slirp4netns will fail with EPERM on setns(CLONE_NEWNET) if they attempt to enter
// the net namespace from the wrong user namespace context, because the kernel does not allow
// jumping over nested namespaces — you must enter from the owning user namespace.
//
// The net namespace is always owned by the intermediate user namespace (created early by bwrap),
// so we use ioctl(NS_GET_USERNS) on the net namespace fd to get its owner directly from the
// kernel. This is race-free unlike reading /proc/<pid>/ns/user, which may show either the
// intermediate or the inner namespace depending on timing (bwrap creates the inner namespace
// for --dev after reporting child-pid via --json-status-fd).
//
// In pre_exec we enter this owning user namespace, which grants full capabilities inside it,
// allowing pasta/slirp4netns to then enter the net namespace successfully.
//
// Due to data race app forced to check --dev arg for bwrap or add artificial sleep

// Extra info during my research
// Example output with --dev:
// BwrapInfo { pid: 520492, sandbox: SandboxStatus { child_pid: 520493, cgroup_namespace: None, ipc_namespace: None, mnt_namespace: Some(4026533709), net_namespace: Some(4026533710), pid_namespace: None, uts_namespace: None } }
// -- proc ns: 4026531837
// -- bwrap ns: 4026531837
// -- bwrap child ns: 4026533698
// -- bwrap child parent ns: 4026531837
// -- bwrap child grandparent ns: Err(NsGetParent(Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }))
// -- bwrap child ns owner: 4026531837
// -- bwrap child netns: 4026533710
// -- bwrap child nstns userns: 4026533698
// ---- SLEEP 200ms ----
// -- proc ns: 4026531837
// -- bwrap ns: 4026531837
// -- bwrap child ns: 4026534106
// -- bwrap child parent ns: 4026533698
// -- bwrap child grandparent ns: Ok(Ok(4026531837))
// -- bwrap child ns owner: 4026533698
// -- bwrap child netns: 4026533710
// -- bwrap child nstns userns: 4026533698
//
// Example output without --dev:
// BwrapInfo { pid: 521275, sandbox: SandboxStatus { child_pid: 521276, cgroup_namespace: None, ipc_namespace: None, mnt_namespace: Some(4026533709), net_namespace: Some(4026533710), pid_namespace: None, uts_namespace: None } }
// -- proc ns: 4026531837
// -- bwrap ns: 4026531837
// -- bwrap child ns: 4026533698
// -- bwrap child parent ns: 4026531837
// -- bwrap child grandparent ns: Err(NsGetParent(Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }))
// -- bwrap child ns owner: 4026531837
// -- bwrap child netns: 4026533710
// -- bwrap child nstns userns: 4026533698
// ---- SLEEP 200ms ----
// -- proc ns: 4026531837
// -- bwrap ns: 4026531837
// -- bwrap child ns: 4026533698
// -- bwrap child parent ns: 4026531837
// -- bwrap child grandparent ns: Err(NsGetParent(Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }))
// -- bwrap child ns owner: 4026531837
// -- bwrap child netns: 4026533710
// -- bwrap child nstns userns: 4026533698

pub fn pre_exec_enter_ns(service: &mut Command, info: &BwrapInfo) -> Result<(), AppError> {
    let netns = Namespace::open_pid(info.sandbox.child_pid, NamespaceType::Net)?;
    let netns_owner = netns.get_userns()?;

    if tracing::enabled!(tracing::Level::TRACE) {
        print_ns_info(info)?;
    }

    let pre_exec = move || -> Result<(), std::io::Error> {
        let status = netns_owner.enter();
        if tracing::enabled!(tracing::Level::TRACE) {
            print_pre_exec_info();
            eprintln!("pre_exec status {status:?}");
        }
        status.map_err(|_| ErrorKind::Other.into())
    };

    unsafe {
        service.pre_exec(pre_exec);
    }

    Ok(())
}

#[tracing::instrument(skip(info))]
fn print_ns_info(info: &BwrapInfo) -> Result<(), AppError> {
    use tracing::trace;

    let bwc_netns = Namespace::open_pid(info.sandbox.child_pid, NamespaceType::Net)?;
    let bwc_netns_uns = bwc_netns.get_userns()?;

    let own_ns = Namespace::open_pid(std::process::id(), NamespaceType::User)?;
    let bw_ns = Namespace::open_pid(info.pid, NamespaceType::User)?;
    let bwc_ns = Namespace::open_pid(info.sandbox.child_pid, NamespaceType::User)?;
    let bwc_pns = bwc_ns.parent()?;
    let bwc_ppns = bwc_pns.parent().map(|v| v.fd_inode());

    let bwc_owner_ns = bwc_ns.get_userns()?;

    trace!("---- APP NAMESPACES INFO ----");
    trace!("-- proc ns: {}", own_ns.fd_inode()?);
    trace!("-- bwrap ns: {}", bw_ns.fd_inode()?);
    trace!("-- bwrap child ns: {}", bwc_ns.fd_inode()?);
    trace!("-- bwrap child parent ns: {}", bwc_pns.fd_inode()?);
    trace!("-- bwrap child grandparent ns: {:?}", bwc_ppns);
    trace!("-- bwrap child ns owner: {}", bwc_owner_ns.fd_inode()?);
    trace!("-- bwrap child netns: {}", bwc_netns.fd_inode()?);
    trace!("-- bwrap child nstns userns: {}", bwc_netns_uns.fd_inode()?);

    Ok(())
}

#[tracing::instrument]
fn print_pre_exec_info() {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let relevant: String = status
        .lines()
        .filter(|l| l.starts_with("Cap") || l.starts_with("Uid") || l.starts_with("Gid"))
        .collect::<Vec<_>>()
        .join("\n");

    let user_ns = std::fs::read_link("/proc/self/ns/user").unwrap_or_default();
    let net_ns = std::fs::read_link("/proc/self/ns/net").unwrap_or_default();

    // Use eprintln to avoid potential deadlock with tracing lib
    eprintln!("---- PRE EXEC INFO ----");
    eprintln!("-- current user ns: {:?}", user_ns);
    eprintln!("-- current net ns: {:?}", net_ns);
    eprintln!("{}", relevant);
}
