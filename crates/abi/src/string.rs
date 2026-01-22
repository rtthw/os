//! # ABI-Stable `String` Type
//!
//! See [`String`] for more information.

use core::{
    cmp,
    fmt::{self, Debug, Display},
    hash::{self, Hash},
    ops::{Deref, DerefMut},
    str::Utf8Error,
};

use crate::Vec;



/// An FFI-safe version of the standard library's `String` type.
///
/// See [`crate::Vec`] for more information as to how this remains FFI-safe.
#[repr(transparent)]
pub struct String {
    bytes: Vec<u8>,
}

pub struct FromUtf8Error {
    pub bytes: Vec<u8>,
    pub error: Utf8Error,
}

impl String {
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: `self.bytes` is guaranteed to be valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.bytes.as_slice()) }
    }

    #[inline]
    pub const fn as_str_mut(&mut self) -> &mut str {
        // SAFETY: `self.bytes` is guaranteed to be valid UTF-8.
        unsafe { str::from_utf8_unchecked_mut(self.bytes.as_slice_mut()) }
    }

    pub const fn from_utf8(bytes: Vec<u8>) -> Result<Self, FromUtf8Error> {
        if let Err(error) = str::from_utf8(bytes.as_slice()) {
            Err(FromUtf8Error { bytes, error })
        } else {
            Ok(Self { bytes })
        }
    }

    #[inline]
    pub const unsafe fn from_utf8_unchecked(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}



unsafe impl Send for String {}
unsafe impl Sync for String {}

impl Deref for String {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for String {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl PartialEq for String {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for String {}

impl PartialOrd for String {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for String {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for String {
    #[inline]
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}



#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::*;

    impl From<alloc::string::String> for String {
        fn from(value: alloc::string::String) -> Self {
            Self {
                bytes: value.into_bytes().into(),
            }
        }
    }

    impl Into<alloc::string::String> for String {
        fn into(self) -> alloc::string::String {
            unsafe { alloc::string::String::from_utf8_unchecked(self.bytes.into()) }
        }
    }
}
