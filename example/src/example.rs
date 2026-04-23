//! # Example Program

#![no_std]

use {
    example_dep::exit,
    input::{GLOBAL_INPUT_QUEUE, InputEvent},
};

const TEST_PAGE_FAULT: bool = false;
const TEST_WRITE_TIME: bool = false;

pub extern "C" fn main() -> ! {
    if TEST_PAGE_FAULT {
        let ptr = 0xab0de as *mut u8;
        unsafe {
            ptr.write(43);
        }
    }
    if TEST_WRITE_TIME {
        unsafe {
            time::set_monotonic_clock_period(1);
        }
    }

    if !time::monotonic_clock_ready() {
        panic!("CLOCK NOT READY");
    }

    let mut fb = framebuffer::Framebuffer::global().unwrap();

    let mut seen_events = 0;
    while seen_events < 255 {
        for event in GLOBAL_INPUT_QUEUE.lock().drain() {
            match event {
                InputEvent::KeyPress { code } => {
                    let value = code as u8;
                    fb.clear_screen(framebuffer::Color::new(
                        value.min(0x2b),
                        value.min(0x2b),
                        value.min(0x33),
                        value,
                    ));
                }
                _ => {}
            }
            seen_events += 1;
        }

        // Defer execution.
        unsafe {
            core::arch::asm!("int 0x40");
        }
    }

    exit()
}
