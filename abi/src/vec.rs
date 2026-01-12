//! # ABI-Stable `Vec` Type
//!
//! See [`Vec`] for more information.

use core::{
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
};



/// An FFI-safe version of the standard library's `Vec` type.
///
/// Can be safely converted to and from a `alloc::vec::Vec` so long as they were
/// both allocated with the [global allocator](alloc::alloc::Global). You cannot
/// create an instance of `Vec` without first having a globally allocated
/// `alloc::vec::Vec`, so it is safe to convert between the two.
///
/// Therefore, it is perfectly safe to pass this type across FFI boundaries so
/// long as both ends share the same address space.
#[repr(C)]
pub struct Vec<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
}



impl<T> Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        // SAFETY: `self.ptr` is never null, and always valid/aligned.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: `self.ptr` is never null, and always valid/aligned.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_mut(), self.len) }
    }
}

impl<T: Debug> Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&**self, f)
    }
}



#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::*;

    impl<T> From<alloc::vec::Vec<T>> for Vec<T> {
        #[inline]
        fn from(value: alloc::vec::Vec<T>) -> Self {
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

    impl<T> Into<alloc::vec::Vec<T>> for Vec<T> {
        #[inline]
        fn into(self) -> alloc::vec::Vec<T> {
            let mut this = core::mem::ManuallyDrop::new(self);
            unsafe { alloc::vec::Vec::from_raw_parts(this.ptr.as_mut(), this.len, this.cap) }
        }
    }

    impl<T> Drop for Vec<T> {
        #[inline]
        fn drop(&mut self) {
            unsafe { drop::<alloc::vec::Vec<T>>(core::ptr::read(self).into()) }
        }
    }
}
