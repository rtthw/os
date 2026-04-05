//! # Example Program

#![no_std]

extern crate example_dep;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    example_dep::exit()
}
