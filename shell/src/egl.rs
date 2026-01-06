//! # EGL Rendering Abstractions

use std::{ffi::c_void, mem::MaybeUninit, os::fd::AsFd, sync::Arc};

use anyhow::{Context as _, Result, bail};
use gbm::AsRaw as _;
use log::info;



mod ffi {
    #![allow(non_camel_case_types)]
    #![allow(unsafe_op_in_unsafe_fn)]
    #![allow(unused)]

    use std::sync::LazyLock;

    use crate::object::Object;

    pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
    pub type khronos_uint64_t = u64;
    pub type khronos_ssize_t = core::ffi::c_long;

    pub type EGLint = i32;
    pub type EGLchar = char;
    pub type EGLLabelKHR = *const core::ffi::c_void;

    pub type EGLNativeDisplayType = NativeDisplayType;
    pub type EGLNativePixmapType = NativePixmapType;
    pub type EGLNativeWindowType = NativeWindowType;

    pub type NativeDisplayType = *const core::ffi::c_void;
    pub type NativePixmapType = *const core::ffi::c_void;
    pub type NativeWindowType = *const core::ffi::c_void;

    include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));

    pub static LIB: LazyLock<Object> =
        LazyLock::new(|| unsafe { Object::open("/usr/lib/x86_64-linux-gnu/libEGL.so.1") }
            .expect("Failed to load libEGL"));

    pub const RESOURCE_BUSY_EXT: u32 = 0x3353;
    pub const DRM_RENDER_NODE_FILE_EXT: u32 = 0x3377;

    /// Raw EGL error
    #[derive(thiserror::Error, Debug)]
    pub enum EGLError {
        /// EGL is not initialized, or could not be initialized, for the specified EGL display
        /// connection.
        #[error(
            "EGL is not initialized, or could not be initialized, for the specified EGL display \
            connection."
        )]
        NotInitialized,
        /// EGL cannot access a requested resource (for example a context is bound in another
        /// thread).
        #[error(
            "EGL cannot access a requested resource (for example a context is bound in another \
            thread)."
        )]
        BadAccess,
        /// EGL failed to allocate resources for the requested operation.
        #[error("EGL failed to allocate resources for the requested operation.")]
        BadAlloc,
        /// An unrecognized attribute or attribute value was passed in the attribute list.
        #[error("An unrecognized attribute or attribute value was passed in the attribute list.")]
        BadAttribute,
        /// An EGLContext argument does not name a valid EGL rendering context.
        #[error("An EGLContext argument does not name a valid EGL rendering context.")]
        BadContext,
        /// An EGLConfig argument does not name a valid EGL frame buffer configuration.
        #[error("An EGLConfig argument does not name a valid EGL frame buffer configuration.")]
        BadConfig,
        /// The current surface of the calling thread is a window, pixel buffer or pixmap that is no longer valid.
        #[error(
            "The current surface of the calling thread is a window, pixel buffer or pixmap that \
            is no longer valid."
        )]
        BadCurrentSurface,
        /// An EGLDevice argument is not valid for this display.
        #[error("An EGLDevice argument is not valid for this display.")]
        BadDevice,
        /// An EGLDisplay argument does not name a valid EGL display connection.
        #[error("An EGLDisplay argument does not name a valid EGL display connection.")]
        BadDisplay,
        /// An EGLSurface argument does not name a valid surface (window, pixel buffer or pixmap) configured for GL rendering.
        #[error("An EGLSurface argument does not name a valid surface (window, pixel buffer or pixmap) configured for GL rendering.")]
        BadSurface,
        /// Arguments are inconsistent (for example, a valid context requires buffers not supplied by a valid surface).
        #[error("Arguments are inconsistent (for example, a valid context requires buffers not supplied by a valid surface).")]
        BadMatch,
        /// One or more argument values are invalid.
        #[error("One or more argument values are invalid.")]
        BadParameter,
        /// A NativePixmapType argument does not refer to a valid native pixmap.
        #[error("A NativePixmapType argument does not refer to a valid native pixmap.")]
        BadNativePixmap,
        /// A NativeWindowType argument does not refer to a valid native window.
        #[error("A NativeWindowType argument does not refer to a valid native window.")]
        BadNativeWindow,
        /// The EGL operation failed due to temporary unavailability of a requested resource, but
        /// the arguments were otherwise valid, and a subsequent attempt may succeed.
        #[error(
            "The EGL operation failed due to temporary unavailability of a requested resource, \
            but the arguments were otherwise valid, and a subsequent attempt may succeed."
        )]
        ResourceBusy,
        /// A power management event has occurred. The application must destroy all contexts and
        /// reinitialize OpenGL ES state and objects to continue rendering.
        #[error(
            "A power management event has occurred. The application must destroy all contexts and \
            reinitialize OpenGL ES state and objects to continue rendering."
        )]
        ContextLost,
        /// An unknown error
        #[error("An unknown error ({0:x})")]
        Unknown(u32),
    }

    impl From<u32> for EGLError {
        #[inline]
        fn from(value: u32) -> Self {
            match value {
                NOT_INITIALIZED => EGLError::NotInitialized,
                BAD_ACCESS => EGLError::BadAccess,
                BAD_ALLOC => EGLError::BadAlloc,
                BAD_ATTRIBUTE => EGLError::BadAttribute,
                BAD_CONTEXT => EGLError::BadContext,
                BAD_CURRENT_SURFACE => EGLError::BadCurrentSurface,
                BAD_DEVICE_EXT => EGLError::BadDevice,
                BAD_DISPLAY => EGLError::BadDisplay,
                BAD_SURFACE => EGLError::BadSurface,
                BAD_MATCH => EGLError::BadMatch,
                BAD_PARAMETER => EGLError::BadParameter,
                BAD_NATIVE_PIXMAP => EGLError::BadNativePixmap,
                BAD_NATIVE_WINDOW => EGLError::BadNativeWindow,
                RESOURCE_BUSY_EXT => EGLError::ResourceBusy,
                CONTEXT_LOST => EGLError::ContextLost,
                other => EGLError::Unknown(other),
            }
        }
    }

    impl EGLError {
        #[inline]
        pub(super) fn from_last_call() -> Option<EGLError> {
            match unsafe { GetError() as u32 } {
                SUCCESS => None,
                x => Some(EGLError::from(x)),
            }
        }
    }

    #[inline]
    pub fn wrap_egl_call_ptr<R, F: FnOnce() -> *const R>(call: F) -> Result<*const R, EGLError> {
        let res = call();
        if !res.is_null() {
            Ok(res)
        } else {
            Err(EGLError::from_last_call().unwrap_or_else(|| {
                println!(
                    "\x1b[33mWARN\x1b[0m \x1b[2m(shell)\x1b[0m: \
                    Erroneous EGL call didn't set EGLError",
                );
                EGLError::Unknown(0)
            }))
        }
    }

    #[inline]
    pub fn wrap_egl_call<R, F>(call: F, err: R) -> Result<R, EGLError>
    where
        R: PartialEq,
        F: FnOnce() -> R,
    {
        let res = call();
        if res != err {
            Ok(res)
        } else {
            Err(EGLError::from_last_call().unwrap_or_else(|| {
                println!(
                    "\x1b[33mWARN\x1b[0m \x1b[2m(shell)\x1b[0m: \
                    Erroneous EGL call didn't set EGLError",
                );
                EGLError::Unknown(0)
            }))
        }
    }

    #[inline]
    pub fn wrap_egl_call_bool<F>(call: F) -> Result<types::EGLBoolean, EGLError>
    where
        F: FnOnce() -> types::EGLBoolean,
    {
        wrap_egl_call(call, FALSE)
    }
}

pub fn init() -> Result<()> {
    ffi::load_with(|sym| {
        let symbol = ffi::LIB.get::<_, *mut std::ffi::c_void>(sym);
        match symbol {
            Some(x) => *x as *const _,
            None => std::ptr::null(),
        }
    });
    ffi::load_with(|sym| unsafe {
        let addr = std::ffi::CString::new(sym.as_bytes()).unwrap();
        let addr = addr.as_ptr();
        ffi::GetProcAddress(addr) as *const _
    });

    Ok(())
}

pub fn extensions() -> Result<Vec<String>> {
    unsafe {
        let ptr = ffi::wrap_egl_call_ptr(|| {
            ffi::QueryString(ffi::NO_DISPLAY, ffi::EXTENSIONS as i32)
        }).context("`EGL_EXT_client_extensions` not supported")?;

        // NOTE: This is only possible with EGL 1.5 or `EGL_EXT_platform_base`, otherwise
        //       `eglQueryString` would have returned an error.
        if ptr.is_null() {
            bail!("Extension not supported: `EGL_EXT_platform_base`")
        } else {
            let p = std::ffi::CStr::from_ptr(ptr);
            let list = String::from_utf8(p.to_bytes().to_vec()).unwrap_or_else(|_| String::new());

            Ok(list.split(' ').map(|e| e.to_string()).collect::<Vec<_>>())
        }
    }
}



pub struct Display {
    inner: Arc<DisplayHandle>,
    egl_version: (i32, i32),
}

impl Display {
    pub fn new<D: AsFd>(device: &gbm::Device<D>) -> Result<Self> {
        let extensions = extensions()?;
        let gbm_ptr = device.as_raw();
        let display = {
            if extensions.iter().any(|e| e == "EGL_KHR_platform_gbm") {
                ffi::wrap_egl_call_ptr(|| unsafe {
                    ffi::GetPlatformDisplayEXT(
                        ffi::PLATFORM_GBM_KHR,
                        gbm_ptr as _,
                        core::ptr::null(),
                    )
                }).context("Failed to get KHR display")?
            } else if extensions.iter().any(|e| e == "EGL_MESA_platform_gbm") {
                ffi::wrap_egl_call_ptr(|| unsafe {
                    ffi::GetPlatformDisplayEXT(
                        ffi::PLATFORM_GBM_MESA,
                        gbm_ptr as _,
                        core::ptr::null(),
                    )
                }).context("Failed to get MESA display")?
            } else {
                bail!("Failed to select a valid EGL platform for device");
            }
        };
        if display == ffi::NO_DISPLAY {
            bail!("Unsupported platform display");
        }

        let egl_version = unsafe {
            let mut major: MaybeUninit<ffi::types::EGLint> = MaybeUninit::uninit();
            let mut minor: MaybeUninit<ffi::types::EGLint> = MaybeUninit::uninit();

            ffi::wrap_egl_call_bool(|| {
                ffi::Initialize(display, major.as_mut_ptr(), minor.as_mut_ptr())
            }).context("Failed to initialize EGL display")?;

            let major = major.assume_init();
            let minor = minor.assume_init();

            (major, minor)
        };

        info!(target: "graphics", "Initialized EGL v{}.{}", egl_version.0, egl_version.1);

        ffi::wrap_egl_call_bool(|| unsafe { ffi::BindAPI(ffi::OPENGL_ES_API) })
            .context("OpenGL ES not supported")?;

        Ok(Self {
            inner: Arc::new(DisplayHandle {
                ptr: display,
                _gbm: gbm_ptr as _,
            }),
            egl_version,
        })
    }

    pub fn extensions(&self) -> Result<Vec<String>> {
        if self.egl_version < (1, 2) {
            return Ok(Vec::new());
        }

        let ptr = ffi::wrap_egl_call_ptr(|| unsafe {
            ffi::QueryDeviceStringEXT(self.inner.ptr, ffi::EXTENSIONS as ffi::types::EGLint)
        }).context("Failed to query display extensions")?;

        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };

        Ok(c_str
            // NOTE: EGL ensures the string is valid UTF-8.
            .to_str().expect("found non-UTF8 display extension name")
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>())
    }
}

struct DisplayHandle {
    ptr: *const c_void,
    _gbm: *const c_void,
}

pub struct Device {
    inner: ffi::types::EGLDeviceEXT,
}

impl Device {
    pub fn for_display(display: &Display) -> Result<Self> {
        let mut device: ffi::types::EGLAttrib = 0;
        if unsafe {
            ffi::QueryDisplayAttribEXT(
                display.inner.ptr,
                ffi::DEVICE_EXT as i32,
                &mut device as *mut _,
            )
        } != ffi::TRUE {
            bail!("No device attributes supported for display");
        }

        let device = device as ffi::types::EGLDeviceEXT;

        if device == ffi::NO_DEVICE_EXT {
            bail!("Unsupported display");
        }

        Ok(Device {
            inner: device,
        })
    }

    pub fn extensions(&self) -> Result<Vec<String>> {
        let ptr = ffi::wrap_egl_call_ptr(|| unsafe {
            ffi::QueryDeviceStringEXT(self.inner, ffi::EXTENSIONS as ffi::types::EGLint)
        }).context("Failed to query device extensions")?;

        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };

        Ok(c_str
            // NOTE: EGL ensures the string is valid UTF-8.
            .to_str().expect("found non-UTF8 device extension name")
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>())
    }
}

pub struct Context {
    inner: ffi::types::EGLContext,
    display: Arc<DisplayHandle>,
}

impl Context {
    pub fn new(display: &Display) -> Result<Self> {
        let attributes = vec![
            ffi::NONE as i32,
        ];
        let context = ffi::wrap_egl_call_ptr(|| unsafe {
            ffi::CreateContext(
                display.inner.ptr,
                ffi::NO_CONFIG_KHR,
                ffi::NO_CONTEXT,
                attributes.as_ptr(),
            )
        })
        .context("Failed to create context")?;

        Ok(Self {
            inner: context,
            display: display.inner.clone(),
        })
    }

    pub unsafe fn make_current(&self) -> Result<()> {
        ffi::wrap_egl_call_bool(|| unsafe {
            ffi::MakeCurrent(
                self.display.ptr,
                ffi::NO_SURFACE,
                ffi::NO_SURFACE,
                self.inner,
            )
        })
        .context("Failed to make EGL context current")?;

        Ok(())
    }
}
