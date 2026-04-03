//! # Window Manager

use {
    crate::{
        BOOT_INFO, input, rtc,
        scheduler::{self, with_scheduler},
    },
    boot_info::BootInfo,
    crossbeam_queue::ArrayQueue,
    framebuffer::{Color, Framebuffer},
    log::{trace, warn},
    memory_types::PAGE_SIZE,
};



static mut EVENT_QUEUE: Option<ArrayQueue<Event>> = None;

pub fn init() {
    unsafe { EVENT_QUEUE = Some(ArrayQueue::new(128)) };
    with_scheduler(|scheduler| {
        scheduler.run_process(
            "window_manager",
            window_manager as *const fn() -> !,
            Some(PAGE_SIZE * 16),
        );
    });
}

pub fn send_event(event: Event) {
    if let Err(event) = unsafe {
        EVENT_QUEUE
            .as_ref()
            .expect("event queue should be initialized")
            .push(event)
    } {
        warn!("Event queue exceeded its capacity, missed 1 event: {event:?}")
    }
}

#[derive(Debug)]
pub enum Event {
    ClockUpdate,
    UserInput(input::InputEvent),
}

fn window_manager() -> ! {
    let mut wm = WindowManager::new();

    loop {
        while let Some(event) = unsafe {
            EVENT_QUEUE
                .as_ref()
                .expect("event queue should be initialized")
                .pop()
        } {
            wm.handle_event(event);
        }
    }
}

struct WindowManager {
    input_state: InputState,
    framebuffer: Framebuffer,
    display_width: usize,
    display_height: usize,
    clock_start_time: rtc::Time,
    clock_start_instant: time::Instant,
}

impl WindowManager {
    fn new() -> Self {
        let boot_info = unsafe {
            BOOT_INFO.expect("window manager should have access to the boot information")
        };

        let mut framebuffer = Framebuffer::from_display_info(&boot_info.display_info);
        framebuffer.clear_screen(Color::rgb(0x2B, 0x2B, 0x33));

        for (col, ch) in "KERNEL v0.0.0".char_indices() {
            framebuffer.draw_ascii_char(
                ch,
                Color::rgb(0xaa, 0xaa, 0xad),
                Color::rgb(0x2B, 0x2B, 0x33),
                10,
                10,
                col,
                0,
            );
            framebuffer.draw_ascii_char(
                '-',
                Color::rgb(0xaa, 0xaa, 0xad),
                Color::rgb(0x2B, 0x2B, 0x33),
                10,
                10,
                col,
                1,
            );
        }

        Self {
            input_state: InputState::new(boot_info),
            framebuffer,
            display_width: boot_info.display_info.width as usize,
            display_height: boot_info.display_info.height as usize,
            clock_start_time: rtc::Time::now(),
            clock_start_instant: time::Instant::now(),
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::ClockUpdate => {
                let dur = self.clock_start_instant.elapsed();
                let current_time = self.clock_start_time + rtc::Time::from_seconds(dur.as_secs());
                let time_string = format!("{current_time}");

                let col_count = self.display_width / framebuffer::font::CHAR_WIDTH;
                let row_count = self.display_height / framebuffer::font::CHAR_HEIGHT;
                let start_col = col_count - 20; // MM/DD/YYYY HH:MM:SS <- 19 chars
                let row = row_count.saturating_sub(2);

                for (col, ch) in time_string.char_indices() {
                    self.framebuffer.draw_ascii_char(
                        ch,
                        Color::rgb(0xaa, 0xaa, 0xad),
                        Color::rgb(0x2B, 0x2B, 0x33),
                        10,
                        10,
                        start_col + col,
                        row,
                    );
                }
            }
            Event::UserInput(input_event) => match input_event {
                input::InputEvent::MouseMove { delta_x, delta_y } => {
                    self.framebuffer.fill_rect(
                        self.input_state.mouse_x as i32,
                        self.input_state.mouse_y as i32,
                        framebuffer::font::CHAR_WIDTH as i32,
                        framebuffer::font::CHAR_HEIGHT as i32,
                        Color::rgb(0x2B, 0x2B, 0x33),
                    );

                    self.input_state.mouse_x = 0.max(
                        (self.display_width as i32 - 1)
                            .min(self.input_state.mouse_x as i32 + delta_x),
                    ) as u32;
                    self.input_state.mouse_y = 0.max(
                        (self.display_height as i32 - 1)
                            .min(self.input_state.mouse_y as i32 + delta_y),
                    ) as u32;

                    self.framebuffer.draw_ascii_char(
                        '^',
                        Color::rgb(0xaa, 0xaa, 0xad),
                        Color::rgb(0x2B, 0x2B, 0x33),
                        self.input_state.mouse_x as i32,
                        self.input_state.mouse_y as i32,
                        0,
                        0,
                    );
                }
                input::InputEvent::KeyPress {
                    code: virtio::virtio_input::codes::KEY_Q,
                } => {
                    scheduler::exit();
                }
                other => {
                    trace!("UNHANDLED USER INPUT: {other:?}");
                }
            },
        }
    }
}

struct InputState {
    mouse_x: u32,
    mouse_y: u32,
}

impl InputState {
    fn new(boot_info: &BootInfo) -> Self {
        Self {
            mouse_x: boot_info.display_info.width / 2,
            mouse_y: boot_info.display_info.height / 2,
        }
    }
}
