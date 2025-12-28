//! # Linux Kernel Interfaces

#![no_std]

extern crate alloc;

pub mod c_str;
mod error;
pub mod file;
pub mod mount;
pub mod proc;
pub mod raw;
pub mod signal;
pub mod traits;



pub use error::Error;
pub type Result<T> = core::result::Result<T, error::Error>;
