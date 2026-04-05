//! # Example Program

#![no_std]

extern crate time;

use core::time::Duration;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    let start = time::now();
    while time::now().duration_since(start) < Duration::from_secs(3) {
        core::hint::spin_loop();
    }

    unsafe {
        core::arch::asm!("int 0x41");
    }

    loop {}
}
