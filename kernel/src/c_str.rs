
use core::ffi::CStr;

use alloc::ffi::CString;

use crate::{Error, Result};



pub const NULL_CSTR: Option<&CStr> = None;

pub trait AsCStr {
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn map_cstr<F, R>(&self, op: F) -> Result<R>
    where
        F: FnOnce(&CStr) -> R;
}



impl AsCStr for str {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn map_cstr<F, R>(&self, op: F) -> Result<R>
    where
        F: FnOnce(&CStr) -> R
    {
        self.as_bytes().map_cstr(op)
    }
}

impl AsCStr for CStr {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.count_bytes()
    }

    fn map_cstr<F, R>(&self, op: F) -> Result<R>
    where
        F: FnOnce(&CStr) -> R
    {
        Ok(op(self))
    }
}

impl AsCStr for [u8] {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn map_cstr<F, R>(&self, op: F) -> Result<R>
    where
        F: FnOnce(&CStr) -> R,
    {
        // NOTE: The real `PATH_MAX` is typically 4096, but it's statistically unlikely to have a
        //       cstr longer than ~300 bytes. See the `nix` PR description for more information:
        //           https://github.com/nix-rust/nix/pull/1656
        //       By being smaller than a memory page, we also avoid the compiler inserting a probe
        //       frame:
        //           https://docs.rs/compiler_builtins/latest/compiler_builtins/probestack
        const MAX_STACK_ALLOCATION: usize = 1024;

        if self.len() >= MAX_STACK_ALLOCATION {
            return map_cstr_alloc(self, op);
        }

        let mut buf = core::mem::MaybeUninit::<[u8; MAX_STACK_ALLOCATION]>::uninit();
        let buf_ptr = buf.as_mut_ptr().cast();

        unsafe {
            core::ptr::copy_nonoverlapping(self.as_ptr(), buf_ptr, self.len());
            buf_ptr.add(self.len()).write(0);
        }

        match CStr::from_bytes_with_nul(unsafe {
            core::slice::from_raw_parts(buf_ptr, self.len() + 1)
        }) {
            Ok(s) => Ok(op(s)),
            Err(_) => Err(Error::INVAL),
        }
    }
}



#[cold]
#[inline(never)]
fn map_cstr_alloc<F, R>(from: &[u8], op: F) -> Result<R>
where
    F: FnOnce(&CStr) -> R,
{
    match CString::new(from) {
        Ok(s) => Ok(op(&s)),
        Err(_) => Err(Error::INVAL),
    }
}

pub(crate) fn map_cstr_opt<S, F, R>(from: Option<&S>, op: F) -> Result<R>
where
    S: AsCStr + ?Sized,
    F: FnOnce(Option<&CStr>) -> R,
{
    match from {
        Some(path) => path.map_cstr(|c_str| op(Some(c_str))),
        None => Ok(op(None)),
    }
}
