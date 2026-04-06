//! # Example Program

#![no_std]

extern crate example_dep;

pub extern "C" fn main() -> ! {
    // let addr: usize = 0x11c260;
    // unsafe {
    //     *(addr as *mut u8) = 43;
    // }
    example_dep::exit()
}
