
use std::ffi::CStr;

use anyhow::Result;
use kernel::c_str::AsCStr;



pub struct Object {
    handle: *mut core::ffi::c_void,
}

unsafe impl Send for Object {}
unsafe impl Sync for Object {}

impl Object {
    pub unsafe fn open<P>(path: &P) -> Result<Self>
    where
        P: AsCStr + ?Sized,
    {
        Ok(path
            .map_cstr(|path| unsafe {
                Self::open_with_path_ptr(path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL)
            })??)
    }

    pub unsafe fn open_this() -> Result<Self> {
        unsafe { Self::open_with_path_ptr(core::ptr::null(), libc::RTLD_LAZY | libc::RTLD_LOCAL) }
    }

    unsafe fn open_with_path_ptr(path: *const i8, flags: i32) -> Result<Self> {
        let result = unsafe { libc::dlopen(path, flags) };

        if result.is_null() {
            Err(unsafe {
                let error_str_ptr = libc::dlerror();
                if error_str_ptr.is_null() {
                    unreachable!("object is being loaded by some other library")
                } else {
                    CStr::from_ptr(error_str_ptr)
                        .to_str()
                        .map_err(|utf8_error| {
                            anyhow::anyhow!(
                                "dlopen error did not contain valid UTF-8: {utf8_error}"
                            )
                        })
                        .map(|string| anyhow::anyhow!(string))?
                }
            })
        } else {
            Ok(Self { handle: result })
        }
    }

    pub fn get<N, T>(&self, name: &N) -> Option<Ptr<T>>
    where
        N: AsCStr + ?Sized,
    {
        if size_of::<*mut core::ffi::c_void>() != size_of::<T>() {
            return None;
        }

        name
            .map_cstr(|name| {
                let value = unsafe { libc::dlsym(self.handle, name.as_ptr()) };
                if value.is_null() {
                    None
                } else {
                    Some(Ptr {
                        ptr: value,
                        _type: core::marker::PhantomData,
                    })
                }
            })
            .ok()?
    }

    pub fn get_untyped<N>(&self, name: &N) -> Option<Ptr<()>>
    where
        N: AsCStr + ?Sized,
    {
        name
            .map_cstr(|name| {
                let value = unsafe { libc::dlsym(self.handle, name.as_ptr()) };
                if value.is_null() {
                    None
                } else {
                    Some(Ptr {
                        ptr: value,
                        _type: core::marker::PhantomData,
                    })
                }
            })
            .ok()?
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        unsafe { libc::dlclose(self.handle); }
    }
}



pub struct Ptr<T> {
    ptr: *mut core::ffi::c_void,
    _type: core::marker::PhantomData<T>,
}

impl<T> Ptr<Option<T>> {
    pub fn lift_option(self) -> Option<Ptr<T>> {
        if self.ptr.is_null() {
            None
        } else {
            Some(Ptr {
                ptr: self.ptr,
                _type: core::marker::PhantomData,
            })
        }
    }
}

unsafe impl<T: Send> Send for Ptr<T> {}
unsafe impl<T: Sync> Sync for Ptr<T> {}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Ptr<T> {
        Ptr { ..*self }
    }
}

impl<T> core::ops::Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*(&self.ptr as *const *mut _ as *const T) }
    }
}

impl<T> core::fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        unsafe {
            let mut info = core::mem::MaybeUninit::<libc::Dl_info>::uninit();
            if libc::dladdr(self.ptr, info.as_mut_ptr()) != 0 {
                let info = info.assume_init();
                if info.dli_sname.is_null() {
                    f.write_fmt(format_args!(
                        "@{:p} from {:?}",
                        self.ptr,
                        CStr::from_ptr(info.dli_fname)
                    ))
                } else {
                    f.write_fmt(format_args!(
                        "{:?}@{:p} from {:?}",
                        CStr::from_ptr(info.dli_sname),
                        self.ptr,
                        CStr::from_ptr(info.dli_fname)
                    ))
                }
            } else {
                f.write_fmt(format_args!("@{:p}", self.ptr))
            }
        }
    }
}
