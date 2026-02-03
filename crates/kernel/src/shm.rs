//! # Shared Memory

use crate::{Error, Result, c_str::AsCStr};



pub struct SharedMemory {
    owned: bool,
    fd: i32,
    size: usize,
    ptr: *mut u8,
}

impl SharedMemory {
    pub fn create<S: AsCStr + ?Sized>(name: &S, size: usize) -> Result<Self> {
        if size == 0 {
            return Err(Error::INVAL);
        }

        let res = name.map_cstr(|name| unsafe {
            libc::shm_open(
                name.as_ptr(),
                libc::O_CREAT
                    | libc::O_EXCL  // Exclusive access (errors if collision).
                    | libc::O_RDWR, // Allow resize.
                libc::S_IRUSR | libc::S_IWUSR, // Read/write permissions.
            )
        })?;
        if res == -1 {
            return Err(Error::latest());
        }

        let mut map = Self {
            owned: true,
            fd: res,
            size,
            ptr: core::ptr::null_mut(),
        };

        // Enlarge the new memory file descriptor size to the requested size.
        let res = unsafe { libc::ftruncate(map.fd, map.size as _) };
        if res == -1 {
            return Err(Error::latest());
        }

        // Put the mapping into this process's address space.
        let res = unsafe {
            libc::mmap(
                core::ptr::null_mut(), /* Address, NULL means "choose the next page-aligned
                                        * address". */
                map.size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                map.fd,
                0, // Offset.
            )
        };
        if res == libc::MAP_FAILED {
            return Err(Error::latest());
        }
        map.ptr = res as *mut _;

        Ok(map)
    }

    pub fn open<S: AsCStr + ?Sized>(name: &S) -> Result<Self> {
        let res = name.map_cstr(|name| unsafe {
            libc::shm_open(name.as_ptr(), libc::O_RDWR, libc::S_IRUSR)
        })?;

        let mut map = Self {
            owned: false,
            fd: res,
            size: 0,
            ptr: core::ptr::null_mut(),
        };

        map.size = {
            let mut buf = core::mem::MaybeUninit::uninit();
            if unsafe { libc::fstat(map.fd, buf.as_mut_ptr()) } == -1 {
                return Err(Error::latest());
            }
            (unsafe { buf.assume_init().st_size }) as usize
        };

        if map.size == 0 {
            return Err(Error::BADFD);
        }

        // Put the mapping into this process's address space.
        let res = unsafe {
            libc::mmap(
                core::ptr::null_mut(), // Address, see `create` for details.
                map.size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                map.fd,
                0, // Offset.
            )
        };
        if res == libc::MAP_FAILED {
            return Err(Error::latest());
        }
        map.ptr = res as *mut _;

        Ok(map)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn owned(&self) -> bool {
        self.owned
    }
}
