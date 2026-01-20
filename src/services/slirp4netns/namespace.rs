use super::Error;
use super::ioctl::NsGetParent;
use rustix::fs::{Mode, OFlags};
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};

#[derive(Debug)]
pub struct Namespace {
    fd: OwnedFd,
}

impl Namespace {
    fn new(fd: OwnedFd) -> Self {
        Self { fd }
    }

    pub fn open_pid(pid: u32) -> Result<Self, Error> {
        let path = format!("/proc/{pid}/ns/user");
        let fd = rustix::fs::open(&path, OFlags::RDONLY, Mode::empty()).map_err(Error::NsOpen)?;
        Ok(Self { fd })
    }

    pub fn parent(&self) -> Result<Self, Error> {
        let fd = unsafe { rustix::ioctl::ioctl(&self.fd, NsGetParent) };
        fd.map(Self::new).map_err(Error::NsGetParent)
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd.as_raw_fd()) }
    }

    pub fn enter(&self) -> Result<(), Error> {
        let flags = rustix::thread::LinkNameSpaceType::User;
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

    use super::Namespace;
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
    fn test_ns_parent() {
        let fir = spawn_unshare();
        let sec = spawn_unshare();

        let my_ns = Namespace::open_pid(std::process::id()).unwrap();

        let fir_ns = Namespace::open_pid(fir.id()).expect("Failed to open child NS");
        let fir_pns = fir_ns.parent().expect("Failed to get parent pns");

        let sec_ns = Namespace::open_pid(sec.id()).expect("Failed to open child NS");
        let sec_pns = sec_ns.parent().expect("Failed to get parent pns");
        assert_ne!(fir_ns.fd_inode().unwrap(), sec_ns.fd_inode().unwrap());
        assert_eq!(fir_pns.fd_inode().unwrap(), my_ns.fd_inode().unwrap());
        assert_eq!(fir_pns.fd_inode().unwrap(), sec_pns.fd_inode().unwrap());
    }
}
