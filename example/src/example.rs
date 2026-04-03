//! # Example Program

#![no_std]

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    unsafe {
        core::arch::asm!("int 0x41");
    }

    loop {}
}
