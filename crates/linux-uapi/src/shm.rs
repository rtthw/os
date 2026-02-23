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



/// A mutual exclusion primitive useful for protecting shared memory.
pub struct Mutex<T: Sized> {
    ptr: *mut libc::pthread_mutex_t,
    data: core::cell::UnsafeCell<*mut T>,
}

// Public.
impl<T: Sized> Mutex<T> {
    /// The size of the mutex header.
    pub const HEADER_SIZE: usize = size_of::<libc::pthread_mutex_t>();

    /// Creates a new shared mutex at the given `base` pointer.
    ///
    /// # Safety
    ///
    /// The provided pointer **MUST** point to a memory region at least as large
    /// as [`Self::HEADER_SIZE`] + `size_of::<T>()`.
    pub unsafe fn new(base: *mut u8) -> Result<Self> {
        let data: *mut T = unsafe { base.add(Self::HEADER_SIZE) } as *mut T;
        let ptr = base as *mut libc::pthread_mutex_t;

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

        let res = unsafe { libc::pthread_mutex_init(ptr, &lock_attr) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        Ok(Self {
            ptr,
            data: core::cell::UnsafeCell::new(data),
        })
    }

    /// Opens an existing shared mutex at the given `base` pointer.
    ///
    /// # Safety
    ///
    /// The provided pointer **MUST** point to an already initialized mutex.
    pub unsafe fn from_existing(base: *mut u8) -> Result<Self> {
        let data: *mut T = unsafe { base.add(Self::HEADER_SIZE) } as *mut T;
        let ptr = base as *mut libc::pthread_mutex_t;

        Ok(Self {
            ptr,
            data: core::cell::UnsafeCell::new(data),
        })
    }

    /// Acquires a mutex, blocking the current thread until it is able to do so.
    ///
    /// **Warning:** This will cause a deadlock if the current thread is already
    /// holding this mutex.
    pub fn lock(&self) -> Result<MutexGuard<'_, T>> {
        let res = unsafe { libc::pthread_mutex_lock(self.ptr) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        Ok(MutexGuard { mutex: self })
    }
}

// Private.
impl<T: Sized> Mutex<T> {
    fn unlock(&self) -> Result<()> {
        let res = unsafe { libc::pthread_mutex_unlock(self.ptr) };
        if res != 0 {
            return Err(Error::from_raw(res));
        }

        Ok(())
    }

    unsafe fn get_inner(&self) -> &mut *mut T {
        unsafe { &mut *self.data.get() }
    }
}



pub struct MutexGuard<'lock, T: Sized> {
    mutex: &'lock Mutex<T>,
}

impl<T: Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // ???: Maybe don't unwrap here?
        self.mutex.unlock().unwrap();
    }
}

impl<T: Sized> Deref for MutexGuard<'_, T> {
    type Target = *mut T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: This is safe to access as long as the guard lives.
        unsafe { self.mutex.get_inner() }
    }
}

impl<T: Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: This is safe to access as long as the guard lives.
        unsafe { self.mutex.get_inner() }
    }
}
