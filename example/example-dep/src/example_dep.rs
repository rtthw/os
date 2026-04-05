//! # Example Dependency

#![no_std]



pub fn exit() -> ! {
    unsafe {
        core::arch::asm!("int 0x41", options(noreturn));
    }
}
