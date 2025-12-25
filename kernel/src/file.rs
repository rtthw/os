
use crate::{c_str::{AsCStr, InvalidCStr}, raw};



#[repr(transparent)]
pub struct File {
    fd: i32,
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


#[derive(Debug)]
pub enum OpenError {
    InvalidPath,
}

impl From<InvalidCStr> for OpenError {
    fn from(_value: InvalidCStr) -> Self {
        Self::InvalidPath
    }
}
