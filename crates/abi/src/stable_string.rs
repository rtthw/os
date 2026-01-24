//! # ABI-Stable `String` Type
//!
//! See [`StableString`] for more information.

use core::{
    cmp,
    fmt::{self, Debug, Display},
    hash::{self, Hash},
    ops::{Deref, DerefMut},
    str::Utf8Error,
};

use crate::StableVec;



/// An FFI-safe version of the standard library's `String` type.
///
/// See [`crate::StableVec`] for more information as to how this remains
/// FFI-safe.
#[repr(transparent)]
pub struct StableString {
    bytes: StableVec<u8>,
}

pub struct FromUtf8Error {
    pub bytes: StableVec<u8>,
    pub error: Utf8Error,
}

impl StableString {
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

    pub const fn from_utf8(bytes: StableVec<u8>) -> Result<Self, FromUtf8Error> {
        if let Err(error) = str::from_utf8(bytes.as_slice()) {
            Err(FromUtf8Error { bytes, error })
        } else {
            Ok(Self { bytes })
        }
    }

    #[inline]
    pub const unsafe fn from_utf8_unchecked(bytes: StableVec<u8>) -> Self {
        Self { bytes }
    }
}



unsafe impl Send for StableString {}
unsafe impl Sync for StableString {}

impl Deref for StableString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for StableString {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl PartialEq for StableString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for StableString {}

impl PartialOrd for StableString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StableString {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for StableString {
    #[inline]
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl Debug for StableString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Display for StableString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}



mod alloc_impls {
    use super::*;

    impl From<String> for StableString {
        fn from(value: String) -> Self {
            Self {
                bytes: value.into_bytes().into(),
            }
        }
    }

    impl Into<String> for StableString {
        fn into(self) -> String {
            unsafe { String::from_utf8_unchecked(self.bytes.into()) }
        }
    }
}
