//! # ABI-Stable `Vec` Type
//!
//! See [`StableVec`] for more information.

use core::{
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
};



/// An FFI-safe version of the standard library's `Vec` type.
///
/// Can be safely converted to and from a `Vec` so long as they were both
/// allocated with the [global allocator](std::alloc::Global). You cannot create
/// an instance of `Vec` without first having a globally allocated `Vec`, so it
/// is safe to convert between the two.
///
/// Therefore, it is perfectly safe to pass this type across FFI boundaries so
/// long as both ends share the same address space.
#[repr(C)]
pub struct StableVec<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
}

impl<T> StableVec<T> {
    pub const fn as_slice(&self) -> &[T] {
        // SAFETY: `self.ptr` is never null, and always valid/aligned.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub const fn as_slice_mut(&mut self) -> &mut [T] {
        // SAFETY: `self.ptr` is never null, and always valid/aligned.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_mut(), self.len) }
    }
}



impl<T> Deref for StableVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> DerefMut for StableVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_slice_mut()
    }
}

impl<T: Debug> Debug for StableVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}



mod alloc_impls {
    use super::*;

    impl<T> From<Vec<T>> for StableVec<T> {
        #[inline]
        fn from(value: Vec<T>) -> Self {
            let len = value.len();
            let cap = value.capacity();
            let ptr = core::mem::ManuallyDrop::new(value).as_mut_ptr();

            Self {
                ptr: unsafe { core::ptr::NonNull::new_unchecked(ptr) }.into(),
                len,
                cap,
            }
        }
    }

    impl<T> Into<Vec<T>> for StableVec<T> {
        #[inline]
        fn into(self) -> Vec<T> {
            let mut this = core::mem::ManuallyDrop::new(self);
            unsafe { Vec::from_raw_parts(this.ptr.as_mut(), this.len, this.cap) }
        }
    }

    impl<T> Drop for StableVec<T> {
        #[inline]
        fn drop(&mut self) {
            unsafe { drop::<Vec<T>>(core::ptr::read(self).into()) }
        }
    }
}
