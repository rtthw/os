//! # Shared Memory

use core::ops::{Deref, DerefMut};

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

    #[inline]
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }
}



pub struct Mutex {
    ptr: *mut libc::pthread_mutex_t,
    data: core::cell::UnsafeCell<*mut u8>,
}

impl Mutex {
    pub unsafe fn new(base: *mut u8) -> Result<Self> {
        let padding = base.align_offset(size_of::<*mut u8>() as _);
        let data: *mut u8 = unsafe { base.add(padding + size_of::<libc::pthread_mutex_t>()) };

        let mut lock_attr = core::mem::MaybeUninit::uninit();

        let res = unsafe { libc::pthread_mutexattr_init(lock_attr.as_mut_ptr()) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        let mut lock_attr = unsafe { lock_attr.assume_init() };

        let res = unsafe {
            libc::pthread_mutexattr_setpshared(&mut lock_attr, libc::PTHREAD_PROCESS_SHARED)
        };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        let ptr = unsafe { base.add(padding) } as *mut _;

        let res = unsafe { libc::pthread_mutex_init(ptr, &lock_attr) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        Ok(Self {
            ptr,
            data: core::cell::UnsafeCell::new(data),
        })
    }

    pub unsafe fn from_existing(base: *mut u8) -> Result<Self> {
        let padding = base.align_offset(size_of::<*mut u8>() as _);
        let data: *mut u8 = unsafe { base.add(padding + size_of::<libc::pthread_mutex_t>()) };
        let ptr = unsafe { base.add(padding) } as *mut _;

        Ok(Self {
            ptr,
            data: core::cell::UnsafeCell::new(data),
        })
    }
}

impl Mutex {
    pub fn lock(&self) -> Result<MutexGuard<'_>> {
        let res = unsafe { libc::pthread_mutex_lock(self.ptr) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        Ok(MutexGuard { mutex: self })
    }

    pub fn unlock(&self) -> Result<()> {
        let res = unsafe { libc::pthread_mutex_unlock(self.ptr) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        Ok(())
    }

    unsafe fn get_inner(&self) -> &mut *mut u8 {
        unsafe { &mut *self.data.get() }
    }
}



pub struct MutexGuard<'lock> {
    mutex: &'lock Mutex,
}

impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        // ???: Maybe don't unwrap here?
        self.mutex.unlock().unwrap();
    }
}

impl Deref for MutexGuard<'_> {
    type Target = *mut u8;
    fn deref(&self) -> &Self::Target {
        // SAFETY: This is safe to access as long as the guard lives.
        unsafe { self.mutex.get_inner() }
    }
}

impl DerefMut for MutexGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: This is safe to access as long as the guard lives.
        unsafe { self.mutex.get_inner() }
    }
}
