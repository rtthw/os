//! # Application Binary Interface (ABI)

#![no_std]

#[cfg(feature = "alloc")]
pub extern crate alloc;

pub mod elf;
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
