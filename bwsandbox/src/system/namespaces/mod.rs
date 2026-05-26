use crate::system::namespaces::ioctl::NsGetUserNs;

use super::Error;
use ioctl::NsGetParent;
use rustix::fs::{Mode, OFlags};
use rustix::thread::LinkNameSpaceType;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

mod ioctl;

#[derive(Debug, Clone, Copy)]
pub enum NamespaceType {
    User,
    Net,
}

impl AsRef<str> for NamespaceType {
    fn as_ref(&self) -> &str {
        match self {
            Self::User => "user",
            Self::Net => "net",
        }
    }
}

impl NamespaceType {
    fn link_type(self) -> LinkNameSpaceType {
        match self {
            Self::User => LinkNameSpaceType::User,
            Self::Net => LinkNameSpaceType::Network,
        }
    }
}

#[derive(Debug)]
pub struct Namespace {
    ty: NamespaceType,
    fd: OwnedFd,
}

impl Namespace {
    fn new(fd: OwnedFd, ty: NamespaceType) -> Self {
        Self { ty, fd }
    }

    pub fn open_pid(pid: u32, ty: NamespaceType) -> Result<Self, Error> {
        let path = format!("/proc/{pid}/ns/{}", ty.as_ref());
        let fd = rustix::fs::open(&path, OFlags::RDONLY, Mode::empty()).map_err(Error::NsOpen)?;
        Ok(Self::new(fd, ty))
    }

    pub fn parent(&self) -> Result<Self, Error> {
        let fd = unsafe { rustix::ioctl::ioctl(&self.fd, NsGetParent) };
        fd.map(|fd| Self::new(fd, self.ty))
            .map_err(Error::NsGetParent)
    }

    pub fn get_userns(&self) -> Result<Self, Error> {
        let fd = unsafe { rustix::ioctl::ioctl(&self.fd, NsGetUserNs) };
        fd.map(|fd| Self::new(fd, NamespaceType::User))
            .map_err(Error::NsGetParent)
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }

    pub fn enter(&self) -> Result<(), Error> {
        let flags = self.ty.link_type();
        rustix::thread::move_into_link_name_space(self.fd(), Some(flags)).map_err(Error::NsEnter)
    }

    // Allow unused because often used to compare parent -> child ns relationship
    #[allow(unused)]
    pub fn fd_inode(&self) -> Result<u64, Error> {
        let stat = rustix::fs::fstat(&self.fd).map_err(Error::NsIno)?;
        Ok(stat.st_ino)
    }
}

#[cfg(test)]
mod tests {
    use rustix::thread::UnshareFlags;

    use super::{Namespace, NamespaceType};
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    fn spawn_unshare() -> Child {
        let child = unsafe {
            Command::new("sleep")
                .arg("infinity")
                .pre_exec(|| {
                    rustix::thread::unshare_unsafe(UnshareFlags::NEWUSER)?;
                    Ok(())
                })
                .spawn()
                .expect("Failed to spawn")
        };

        // Wait until ready
        std::thread::sleep(std::time::Duration::from_millis(100));
        child
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_ns_parent() {
        let mut fir = spawn_unshare();
        let mut sec = spawn_unshare();

        let my_ns = Namespace::open_pid(std::process::id(), NamespaceType::User).unwrap();

        let fir_ns =
            Namespace::open_pid(fir.id(), NamespaceType::User).expect("Failed to open child NS");
        let fir_pns = fir_ns.parent().expect("Failed to get parent pns");

        let sec_ns =
            Namespace::open_pid(sec.id(), NamespaceType::User).expect("Failed to open child NS");
        let sec_pns = sec_ns.parent().expect("Failed to get parent pns");
        assert_ne!(fir_ns.fd_inode().unwrap(), sec_ns.fd_inode().unwrap());
        assert_eq!(fir_pns.fd_inode().unwrap(), my_ns.fd_inode().unwrap());
        assert_eq!(fir_pns.fd_inode().unwrap(), sec_pns.fd_inode().unwrap());

        fir.kill().unwrap();
        fir.wait().unwrap();
        sec.kill().unwrap();
        sec.wait().unwrap();
    }
}
