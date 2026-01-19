//! # Application Binary Interface (ABI)

#![no_std]

#[cfg(feature = "alloc")]
pub extern crate alloc;

pub mod elf;
pub mod layout;
pub mod path;
pub mod string;
pub mod vec;

pub use {path::Path, string::String, vec::Vec};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");



#[derive(Debug)]
pub struct Manifest {
    pub name: &'static str,
    pub entry_point: extern "C" fn(),
    pub dependencies: &'static [&'static str],
    pub abi_version: &'static str,
}

#[macro_export]
macro_rules! manifest {
    (
        name: $name_def:expr,
        entry_point: $entry_point_def:expr,
        dependencies: $dependencies_def:expr,
    ) => {
        #[unsafe(no_mangle)]
        pub static __MANIFEST: $crate::Manifest = $crate::Manifest {
            name: $name_def,
            entry_point: $entry_point_def,
            dependencies: $dependencies_def,
            abi_version: $crate::VERSION,
        };
    };
}

#[macro_export]
macro_rules! include {
    (
        mod $name:ident {
            $(
                fn $fn_name:ident ($( $fn_arg:ident : $fn_arg_ty:ty ),*) $( -> $fn_ret_ty:ty )?;
            )*
        }
    ) => {
        pub mod $name {
            unsafe extern "Rust" {
                $(
                    #[link_name = concat!("__", stringify!($name), "_", stringify!($fn_name))]
                    pub fn $fn_name($($fn_arg: $fn_arg_ty)*) $(-> $fn_ret_ty)? ;
                )*
            }
        }
    };
}

#[macro_export]
macro_rules! declare {
    (
        mod $name:ident {
            $(
                fn $fn_name:ident ($( $fn_arg:ident : $fn_arg_ty:ty ),*) $( -> $fn_ret_ty:ty )? {
                    $( $fn_body:tt )*
                }
            )*
        }
    ) => {
        $(
            #[unsafe(export_name = concat!("__", stringify!($name), "_", stringify!($fn_name)))]
            pub unsafe extern "Rust" fn $fn_name($($fn_arg: $fn_arg_ty)*) $(-> $fn_ret_ty)? {
                $($fn_body)*
            }
        )*
    };
}



#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Aabb2D<V> {
    pub x_min: V,
    pub x_max: V,
    pub y_min: V,
    pub y_max: V,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Rgba<V> {
    pub r: V,
    pub g: V,
    pub b: V,
    pub a: V,
}



#[repr(C)]
pub struct RenderPass {
    pub bounds: Aabb2D<f32>,
    pub layers: Vec<RenderLayer>,
}

#[repr(C)]
pub struct RenderLayer {
    pub objects: Vec<RenderObject>,
}

#[repr(C)]
pub enum RenderObject {
    Quad {
        bounds: Aabb2D<f32>,
        color: Rgba<u8>,
    },
}
