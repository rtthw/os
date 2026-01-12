//! # Application Binary Interface (ABI)

#![no_std]

#[cfg(feature = "alloc")]
pub extern crate alloc;

pub mod string;
pub mod vec;

pub use {string::String, vec::Vec};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
