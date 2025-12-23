
use crate::raw;



#[repr(transparent)]
pub struct File {
    fd: i32,
}

impl File {
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
