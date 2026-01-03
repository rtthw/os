
use std::{env, fs::File, path::PathBuf};

use gl_generator::{Api, Fallbacks, Profile, Registry};



fn main() {
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

    let mut file = File::create(dest.join("egl_bindings.rs")).unwrap();
    Registry::new(
        Api::Egl,
        (1, 5),
        Profile::Core,
        Fallbacks::All,
        [
            "EGL_KHR_create_context",
            "EGL_EXT_create_context_robustness",
            "EGL_KHR_create_context_no_error",
            "EGL_KHR_no_config_context",
            "EGL_EXT_pixel_format_float",
            "EGL_EXT_device_base",
            "EGL_EXT_device_enumeration",
            "EGL_EXT_device_query",
            "EGL_EXT_device_drm",
            "EGL_EXT_device_drm_render_node",
            "EGL_KHR_stream",
            "EGL_KHR_stream_producer_eglsurface",
            "EGL_EXT_platform_base",
            "EGL_KHR_platform_x11",
            "EGL_EXT_platform_x11",
            "EGL_KHR_platform_wayland",
            "EGL_EXT_platform_wayland",
            "EGL_KHR_platform_gbm",
            "EGL_MESA_platform_gbm",
            "EGL_MESA_platform_surfaceless",
            "EGL_EXT_platform_device",
            "EGL_WL_bind_wayland_display",
            "EGL_KHR_image_base",
            "EGL_EXT_image_dma_buf_import",
            "EGL_EXT_image_dma_buf_import_modifiers",
            "EGL_MESA_image_dma_buf_export",
            "EGL_KHR_gl_image",
            "EGL_EXT_buffer_age",
            "EGL_EXT_swap_buffers_with_damage",
            "EGL_KHR_swap_buffers_with_damage",
            "EGL_KHR_fence_sync",
            "EGL_ANDROID_native_fence_sync",
            "EGL_IMG_context_priority",
        ],
    )
        .write_bindings(gl_generator::GlobalGenerator, &mut file)
        .unwrap();

    let mut file = File::create(dest.join("gl_bindings.rs")).unwrap();
    Registry::new(
        Api::Gles2,
        (3, 2),
        Profile::Compatibility,
        Fallbacks::None,
        [
            "GL_OES_EGL_image",
            "GL_OES_EGL_image_external",
            "GL_EXT_texture_format_BGRA8888",
            "GL_EXT_unpack_subimage",
            "GL_OES_EGL_sync",
        ],
    )
        .write_bindings(gl_generator::StructGenerator, &mut file)
        .unwrap();
}
