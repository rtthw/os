//! # Example Dependency

#![no_std]



pub fn exit() -> ! {
    unsafe {
        core::arch::asm!("int 0x41", options(noreturn));
    }
}

#[cfg(not(test))]
#[panic_handler]
pub fn panic_handler(_info: &core::panic::PanicInfo<'_>) -> ! {
    exit()
}
