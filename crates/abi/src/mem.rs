//! # Memory Management

use std::{
    alloc::{Layout, alloc},
    ops::{Deref, DerefMut},
};


/// An array of bytes within memory.
#[derive(Debug)]
pub struct MemoryMap {
    ptr: *mut u8,
    len: usize,
}

impl MemoryMap {
    /// Allocate an unitialized array of bytes with the given length. See
    /// [`alloc_zeroed`](Self::alloc_zeroed) for a safe alternative.
    pub unsafe fn alloc_uninit(len: usize) -> Result<Self, &'static str> {
        let ptr =
            unsafe { alloc(Layout::array::<u8>(len).map_err(|_| "memory mapping too large")?) };

        Ok(Self { ptr, len })
    }

    /// Allocate an array of bytes with the given length, and set them to all
    /// zeroes.
    pub fn alloc_zeroed(len: usize) -> Result<Self, &'static str> {
        let mut this = unsafe { Self::alloc_uninit(len)? };
        this.fill(0);
        Ok(this)
    }

    /// Get a raw shared pointer to the underlying byte array.
    #[inline]
    pub const unsafe fn ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get a raw unique pointer to the underlying byte array.
    #[inline]
    pub const unsafe fn ptr_mut(&mut self) -> *mut u8 {
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
    /// use abi::mem::MemoryMap;
    /// let mut map = MemoryMap::alloc_zeroed(4).unwrap();
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
    /// use abi::mem::MemoryMap;
    /// let mut map = MemoryMap::alloc_zeroed(4).unwrap();
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



#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use super::*;

    #[test]
    fn works() {
        let mut map = MemoryMap::alloc_zeroed(8).unwrap();
        map.as_slice_mut(3, 5).fill(1);
        map.as_slice_mut(0, 2).fill(2);
        assert_eq!(map.as_slice(0, 8), &[2, 2, 0, 1, 1, 1, 1, 1]);
    }

    #[test]
    #[should_panic]
    fn overflow_checks() {
        let Ok(map) = MemoryMap::alloc_zeroed(8) else {
            return; // Fail the test if allocation fails.
        };
        black_box(map.as_slice(3, 6));
    }
}
