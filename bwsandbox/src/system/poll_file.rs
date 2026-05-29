use crate::error::AppError;
use crate::system::Error;
use crate::system::poll::Poll;
use std::ffi::{CStr, CString};
use std::os::fd::OwnedFd;
use std::{io::ErrorKind, path::Path, time::Duration};

pub struct PollFile<'a> {
    path: &'a Path,
    file_name: CString,
    inot: OwnedFd,
}

impl<'a> PollFile<'a> {
    pub fn watch(path: &'a Path) -> Result<Self, AppError> {
        use rustix::path::Arg;

        let file_dir = match path.parent() {
            Some(dir) if dir.is_dir() => Ok(dir),
            Some(dir) => AppError::File(dir.into(), ErrorKind::NotADirectory.into()).into_err(),
            None => AppError::File(path.into(), ErrorKind::NotFound.into()).into_err(),
        }?;

        // Always will be Cow::Owned because path segments in rust not null terminated
        let file_name = path
            .file_name()
            .and_then(|v| v.into_c_str().map(CString::from).ok())
            .ok_or_else(|| AppError::File(path.into(), ErrorKind::InvalidFilename.into()))?;

        let inot = Self::inotify_init(file_dir)?;

        Ok(Self {
            path,
            file_name,
            inot,
        })
    }

    fn inotify_init(dir: &Path) -> Result<OwnedFd, Error> {
        use rustix::fs::inotify::{self, CreateFlags, WatchFlags};

        let inot = inotify::init(CreateFlags::CLOEXEC).map_err(Error::InotInit)?;
        inotify::add_watch(&inot, dir, WatchFlags::CREATE).map_err(Error::InotWatch)?;
        Ok(inot)
    }

    fn poll_once(&self, timeout: Duration) -> Result<Option<CString>, AppError> {
        use linux_raw_sys::general::{NAME_MAX, inotify_event};
        use std::mem::MaybeUninit;

        const INOTIFY_BUF_SIZE: usize = size_of::<inotify_event>() + NAME_MAX as usize + 1;

        Poll::new(&self.inot)
            .poll_in(timeout)
            .map_err(AppError::file(self.path))?;

        let mut buf = [MaybeUninit::uninit(); INOTIFY_BUF_SIZE];
        let mut reader = rustix::fs::inotify::Reader::new(&self.inot, &mut buf);
        let evt = reader.next().map_err(Error::InotRead)?;
        Ok(evt.file_name().map(CStr::to_owned))
    }

    pub fn wait_exists(self, timeout: Duration) -> Result<(), AppError> {
        use std::time::Instant;

        // Skip polling if file was created in-between watch and wait_exists
        if self.path.exists() {
            return Ok(());
        }

        let now = Instant::now();
        while now.elapsed() < timeout {
            let timeout = timeout.saturating_sub(now.elapsed());
            let Some(name) = self.poll_once(timeout)? else {
                // Missing filename from inot?
                continue;
            };

            if name == self.file_name.as_ref() {
                return Ok(());
            }
        }

        Err(AppError::file(self.path)(ErrorKind::TimedOut.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::rand_id;
    use std::{io::Write, path::PathBuf, time::Duration};

    fn spawn_writter(path: PathBuf, sleep_ms: u64) {
        let sleep_ms = Duration::from_millis(sleep_ms);
        std::thread::spawn(move || {
            std::thread::sleep(sleep_ms);
            let mut f = std::fs::File::create_new(path).unwrap();
            f.write_all(b"Hello world").unwrap();
        });
    }

    #[test]
    fn test_file_created_poll() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.me");
        let poll = PollFile::watch(&file).expect("Failed to init watcher");

        for i in 0..10 {
            spawn_writter(dir.path().join(rand_id(10)), i * 10);
        }
        spawn_writter(file.clone(), 200);

        let res = poll.wait_exists(Duration::from_millis(500));
        assert!(res.is_ok());
    }

    #[test]
    fn test_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.me");
        let poll = PollFile::watch(&file).expect("Failed to init watcher");

        match poll.wait_exists(Duration::from_millis(100)) {
            Err(AppError::File(_, err)) if err.kind() == ErrorKind::TimedOut => {}
            a => panic!("Unexpected result {a:?}"),
        }
    }
}
