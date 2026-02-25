//! # Memory Management

use std::{
    ffi::{c_int, c_void},
    ops::{Deref, DerefMut},
};


unsafe extern "C" {
    pub fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
}

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const PROT_EXEC: c_int = 4;

const MAP_PRIVATE: c_int = 0x0002;
const MAP_ANONYMOUS: c_int = 0x0020;

const MAP_FAILED: *mut c_void = !0 as *mut c_void;


/// An array of bytes within memory.
#[derive(Debug)]
pub struct MemoryMap {
    ptr: *mut c_void,
    len: usize,
}

impl MemoryMap {
    /// Allocate an unitialized array of bytes with the given length. See
    /// [`alloc_zeroed`](Self::alloc_zeroed) for a safe alternative.
    pub unsafe fn alloc_uninit(len: usize, flags: MapFlags) -> Result<Self, &'static str> {
        if len == 0 {
            return Err("memory map must have non-zero length");
        }
        unsafe {
            let ptr = mmap(
                core::ptr::null_mut(),
                len,
                flags.0,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1, // File descriptor is ignored for anonymous mappings.
                0,
            );

            if ptr == MAP_FAILED {
                // FIXME: This is not helpful.
                return Err("failed to allocate memory map");
            }

            Ok(Self { ptr, len })
        }
    }

    /// Allocate an array of bytes with the given length, and set them to all
    /// zeroes.
    pub fn alloc_zeroed(len: usize, flags: MapFlags) -> Result<Self, &'static str> {
        let mut this = unsafe { Self::alloc_uninit(len, flags)? };
        this.fill(0);
        Ok(this)
    }

    /// Get a raw shared pointer to the underlying byte array.
    #[inline]
    pub const unsafe fn ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get a raw unique pointer to the underlying byte array.
    #[inline]
    pub const unsafe fn ptr_mut(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Get the numeric address of the underlying byte array.
    #[inline]
    pub fn addr(&self) -> usize {
        self.ptr as usize
    }

    /// Get the length of the underlying byte array.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Get an immutable slice into the underlying byte array at the given
    /// offset, with the given length.
    ///
    /// # Examples
    /// ```rust
    /// use abi::mem::{MapFlags, MemoryMap};
    /// let mut map = MemoryMap::alloc_zeroed(4, MapFlags::READ_WRITE).unwrap();
    /// map.as_slice_mut(0, 4).copy_from_slice(&[0, 1, 2, 3]);
    /// assert_eq!(map.as_slice(0, 4), &[0, 1, 2, 3]);
    /// assert_eq!(map.as_slice(1, 3),    &[1, 2, 3]);
    /// assert_eq!(map.as_slice(2, 2),       &[2, 3]);
    /// assert_eq!(map.as_slice(3, 1),          &[3]);
    /// assert_eq!(map.as_slice(0, 1), &[0]         );
    /// assert_eq!(map.as_slice(0, 2), &[0, 1]      );
    /// assert_eq!(map.as_slice(0, 3), &[0, 1, 2]   );
    /// ```
    pub fn as_slice(&self, offset: usize, len: usize) -> &[u8] {
        debug_assert!(
            offset + len <= self.len(),
            "requested offset and length would overflow memory mapping",
        );
        let addr = self.addr() + offset;

        unsafe { core::slice::from_raw_parts(addr as *mut u8, len) }
    }

    /// Get a mutable slice into the underlying byte array at the given offset,
    /// with the given length.
    ///
    /// # Examples
    /// ```rust
    /// use abi::mem::{MapFlags, MemoryMap};
    /// let mut map = MemoryMap::alloc_zeroed(4, MapFlags::READ_WRITE).unwrap();
    /// map.as_slice_mut(3, 1).fill(1);
    /// map.as_slice_mut(0, 2).fill(2);
    /// assert_eq!(map.as_slice(0, 4), &[2, 2, 0, 1]);
    /// ```
    pub fn as_slice_mut(&mut self, offset: usize, len: usize) -> &mut [u8] {
        debug_assert!(
            offset + len <= self.len(),
            "requested offset and length would overflow memory mapping",
        );
        let addr = self.addr() + offset;

        unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, len) }
    }
}

impl Deref for MemoryMap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice(0, self.len)
    }
}

impl DerefMut for MemoryMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut(0, self.len)
    }
}



/// Some combination of readable, writable, and executable.
#[repr(transparent)]
pub struct MapFlags(c_int);

impl MapFlags {
    pub const READ_ONLY: Self = Self::none().read();
    pub const READ_WRITE: Self = Self::none().read().write();
    pub const READ_WRITE_EXEC: Self = Self::all();

    pub const fn none() -> Self {
        Self(PROT_NONE)
    }

    pub const fn all() -> Self {
        Self::none().read().write().execute()
    }

    pub const fn read(self) -> Self {
        Self(self.0 | PROT_READ)
    }

    pub const fn write(self) -> Self {
        Self(self.0 | PROT_WRITE)
    }

    pub const fn execute(self) -> Self {
        Self(self.0 | PROT_EXEC)
    }
}



#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use super::*;

    #[test]
    fn works() {
        let mut map = MemoryMap::alloc_zeroed(8, MapFlags::READ_WRITE).unwrap();
        map.as_slice_mut(3, 5).fill(1);
        map.as_slice_mut(0, 2).fill(2);
        assert_eq!(map.as_slice(0, 8), &[2, 2, 0, 1, 1, 1, 1, 1]);
    }

    #[test]
    #[should_panic]
    fn overflow_checks() {
        let Ok(map) = MemoryMap::alloc_zeroed(8, MapFlags::READ_WRITE) else {
            return; // Fail the test if allocation fails.
        };
        black_box(map.as_slice(3, 6));
    }
}
