//! # Example Program

#![no_std]

use {
    core::sync::atomic::Ordering,
    example_dep::exit,
    framebuffer::Color,
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

    let mut framebuffer = framebuffer::Framebuffer::global().unwrap();
    let display_width = framebuffer::FRAMEBUFFER_WIDTH.load(Ordering::Relaxed);
    let display_height = framebuffer::FRAMEBUFFER_HEIGHT.load(Ordering::Relaxed);
    let mut input_state = InputState {
        mouse_x: display_width as u32 / 2,
        mouse_y: display_height as u32 / 2,
    };

    'main_loop: loop {
        for event in GLOBAL_INPUT_QUEUE.lock().drain() {
            match event {
                InputEvent::KeyPress { code } => {
                    if code == 16 {
                        break 'main_loop;
                    }
                }
                InputEvent::MouseMove { delta_x, delta_y } => {
                    framebuffer.fill_rect(
                        input_state.mouse_x as i32,
                        input_state.mouse_y as i32,
                        framebuffer::font::CHAR_WIDTH as i32,
                        framebuffer::font::CHAR_HEIGHT as i32,
                        Color::rgb(0x2B, 0x2B, 0x33),
                    );

                    input_state.mouse_x = 0
                        .max((display_width as i32 - 1).min(input_state.mouse_x as i32 + delta_x))
                        as u32;
                    input_state.mouse_y = 0
                        .max((display_height as i32 - 1).min(input_state.mouse_y as i32 + delta_y))
                        as u32;

                    framebuffer.draw_ascii_char(
                        '^',
                        Color::rgb(0xaa, 0xaa, 0xad),
                        Color::rgb(0x2B, 0x2B, 0x33),
                        input_state.mouse_x as i32,
                        input_state.mouse_y as i32,
                        0,
                        0,
                    );
                }
                _ => {}
            }
        }

        // Defer execution.
        unsafe {
            core::arch::asm!("int 0x40");
        }
    }

    exit()
}

struct InputState {
    mouse_x: u32,
    mouse_y: u32,
}
