//! # ABI-Stable `Path` Type
//!
//! See [`Path`] for more information.

use core::{
    cmp,
    fmt::{self, Debug, Display},
    hash::{self, Hash},
    ops::{Deref, DerefMut},
};

use crate::{StableString, StableVec};



/// An FFI-safe version of the standard library's `PathBuf` type.
///
/// See [`crate::Vec`] for more information as to how this remains FFI-safe.
#[repr(transparent)]
pub struct Path {
    bytes: StableVec<u8>,
}

pub struct PathValidationError {
    pub bytes: StableVec<u8>,
    pub valid_up_to: usize,
}

impl Path {
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

    pub const fn from_bytes(bytes: StableVec<u8>) -> Result<Self, PathValidationError> {
        if let Err(invalid_byte) = check(bytes.as_slice()) {
            Err(PathValidationError {
                bytes,
                valid_up_to: invalid_byte,
            })
        } else {
            Ok(Self { bytes })
        }
    }

    #[inline]
    pub const unsafe fn from_bytes_unchecked(bytes: StableVec<u8>) -> Self {
        Self { bytes }
    }

    #[inline]
    pub fn into_bytes(self) -> StableVec<u8> {
        self.bytes
    }

    #[inline]
    pub fn into_string(self) -> StableString {
        // SAFETY: `self.bytes` is guaranteed to be valid UTF-8.
        unsafe { StableString::from_utf8_unchecked(self.bytes) }
    }
}



unsafe impl Send for Path {}
unsafe impl Sync for Path {}

impl Deref for Path {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for Path {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_str_mut()
    }
}

impl PartialEq for Path {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Eq for Path {}

impl PartialOrd for Path {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for Path {
    #[inline]
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher)
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}



const fn check(bytes: &[u8]) -> Result<(), usize> {
    let mut index = 0;
    let len = bytes.len();

    while index < len {
        let ch = bytes[index];

        match ch {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-'..=b'9' => {}
            _ => return Err(index),
        }

        index += 1;
    }

    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation() {
        assert!(check(b"0123456789").is_ok());
        assert!(check(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ").is_ok());
        assert!(check(b"abcdefghijklmnopqrstuvwxyz").is_ok());
        assert!(check(b".-_/").is_ok());

        assert!(check(b"/path/to-a-place-with-hyphens").is_ok());
        assert!(check(b"/path/to-a-place-with-hyphens_and_underscores").is_ok());
        assert!(check(b"path-without-any-slashes").is_ok());
        assert!(check(b"../relative-path").is_ok());
        assert!(check(b"..").is_ok());

        assert!(check(b"~/relative-to-home").is_err());
        assert!(check(b"$ENV_VAR").is_err());
        assert!(check(b";").is_err());
        assert!(check(b":").is_err());
        assert!(check(b"\"").is_err());
        assert!(check(b"\'").is_err());
        assert!(check(b" ").is_err());
    }
}
