
use crate::{c_str::{AsCStr, InvalidCStr}, proc::{ProcessGroup, Session}, raw};



#[derive(Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct File {
    pub(crate) fd: i32,
}

impl File {
    pub const STDIN: Self = Self { fd: libc::STDIN_FILENO };
    pub const STDOUT: Self = Self { fd: libc::STDOUT_FILENO };
    pub const STDERR: Self = Self { fd: libc::STDERR_FILENO };
}

impl File {
    // https://www.man7.org/linux/man-pages/man2/open.2.html
    pub fn open<P: AsCStr + ?Sized>(path: &P, flags: OpenFlags) -> Result<Self, OpenError> {
        let ret = path.map_cstr(|path| raw::open(path, flags.0))?;
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(Self { fd: ret })
        }
    }

    // https://www.man7.org/linux/man-pages/man2/close.2.html
    pub fn close(self) -> Result<(), (/* TODO */)> {
        let ret = raw::close(self.fd);
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man2/read.2.html
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, (/* TODO */)> {
        let ret = raw::read(self.fd, buf, buf.len().min(isize::MAX as usize));
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(ret as usize)
        }
    }

    // https://www.man7.org/linux/man-pages/man2/write.2.html
    pub fn write(&self, buf: &[u8]) -> Result<usize, (/* TODO */)> {
        let ret = raw::write(self.fd, buf, buf.len().min(isize::MAX as usize));
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(ret as usize)
        }
    }

    // https://www.man7.org/linux/man-pages/man2/fchmod.2.html
    pub fn change_mode(&self, new_mode: u32) -> Result<(), (/* TODO */)> {
        let ret = raw::fchmod(self.fd, new_mode);
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man2/fchown.2.html
    pub fn change_owner(&self, new_owner: u32, new_group: u32) -> Result<(), (/* TODO */)> {
        let ret = raw::fchown(self.fd, new_owner, new_group);
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man2/dup.2.html
    pub fn duplicate(&self) -> Result<Self, (/* TODO */)> {
        let ret = raw::dup(self.fd);
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(Self { fd: ret })
        }
    }
}

impl File {
    pub fn is_a_tty(&self) -> bool {
        // NOTE: We ignore the error here because it doesn't matter whether the file descriptor is
        //       valid at this point, just whether it's a TTY.
        unsafe { libc::isatty(self.fd) == 1 }
    }

    pub fn terminal_session(&self) -> Result<Session, (/* TODO */)> {
        let ret = unsafe { libc::tcgetsid(self.fd) };
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(Session { id: ret })
        }
    }

    // https://www.man7.org/linux/man-pages/man2/TIOCNOTTY.2const.html
    pub fn release_terminal_control(&self) -> Result<(), (/* TODO */)> {
        let ret = unsafe { libc::ioctl(self.fd, libc::TIOCNOTTY) };
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man2/TIOCSCTTY.2const.html
    pub fn take_terminal_control(&self) -> Result<(), (/* TODO */)> {
        let ret = unsafe { libc::ioctl(self.fd, libc::TIOCSCTTY, 1) };
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man3/tcsetpgrp.3.html
    pub fn set_foreground_process_group(&self, group: ProcessGroup) -> Result<(), (/* TODO */)> {
        let ret = unsafe { libc::tcsetpgrp(self.fd, group.id) };
        if ret == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }
}



#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct OpenFlags(i32);

pub const O_APPEND: OpenFlags = OpenFlags(libc::O_APPEND);
pub const O_ASYNC: OpenFlags = OpenFlags(libc::O_ASYNC);
pub const O_CLOEXEC: OpenFlags = OpenFlags(libc::O_CLOEXEC);
pub const O_CREAT: OpenFlags = OpenFlags(libc::O_CREAT);
pub const O_DIRECT: OpenFlags = OpenFlags(libc::O_DIRECT);
pub const O_DIRECTORY: OpenFlags = OpenFlags(libc::O_DIRECTORY);
pub const O_DSYNC: OpenFlags = OpenFlags(libc::O_DSYNC);
pub const O_EXCL: OpenFlags = OpenFlags(libc::O_EXCL);
pub const O_LARGEFILE: OpenFlags = OpenFlags(libc::O_LARGEFILE);
pub const O_NOATIME: OpenFlags = OpenFlags(libc::O_NOATIME);
pub const O_NOCTTY: OpenFlags = OpenFlags(libc::O_NOCTTY);
pub const O_NOFOLLOW: OpenFlags = OpenFlags(libc::O_NOFOLLOW);
pub const O_NONBLOCK: OpenFlags = OpenFlags(libc::O_NONBLOCK);
pub const O_PATH: OpenFlags = OpenFlags(libc::O_PATH);
pub const O_RDONLY: OpenFlags = OpenFlags(libc::O_RDONLY);
pub const O_RDWR: OpenFlags = OpenFlags(libc::O_RDWR);
pub const O_SYNC: OpenFlags = OpenFlags(libc::O_SYNC);
pub const O_TMPFILE: OpenFlags = OpenFlags(libc::O_TMPFILE);
pub const O_TRUNC: OpenFlags = OpenFlags(libc::O_TRUNC);
pub const O_WRONLY: OpenFlags = OpenFlags(libc::O_WRONLY);

impl core::ops::BitOr for OpenFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}



#[derive(Debug)]
pub enum OpenError {
    InvalidPath,
}

impl From<InvalidCStr> for OpenError {
    fn from(_value: InvalidCStr) -> Self {
        Self::InvalidPath
    }
}
