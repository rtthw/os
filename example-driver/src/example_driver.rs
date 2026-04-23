//! # Example Driver

#![no_std]

use {
    core::time::Duration,
    input::{GLOBAL_INPUT_QUEUE, InputEvent},
};

pub extern "C" fn main() -> ! {
    for i in 0..255 {
        let start = time::now();
        while time::now().duration_since(start) < Duration::from_millis(20) {
            // Defer execution.
            unsafe {
                core::arch::asm!("int 0x40");
            }
        }

        GLOBAL_INPUT_QUEUE
            .lock()
            .push(InputEvent::KeyPress { code: i });
    }

    // Exit.
    unsafe {
        core::arch::asm!("int 0x41", options(noreturn));
    }
}
