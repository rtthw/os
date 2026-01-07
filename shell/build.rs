
use std::{env, fs::File, path::PathBuf};

use gl_generator::{Api, Fallbacks, Profile, Registry};



fn main() {
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

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
