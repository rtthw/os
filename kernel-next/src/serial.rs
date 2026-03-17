//! # Serial Port

use {
    core::fmt::Write,
    log::LogLevel,
    spin_mutex::Mutex,
    x86_64::instructions::{interrupts::without_interrupts, port::Port},
};


pub static SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    SERIAL1.lock().init();
    log::set_logger(&SerialLogger).unwrap();
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("failed to write to serial port");
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n")
    };
    ($fmt:expr) => {
        $crate::serial_print!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::serial_print!(concat!($fmt, "\n"), $($arg)*)
    };
}



const ANSI_SGR_RESET: u8 = 0;
const ANSI_SGR_BOLD: u8 = 0;
const ANSI_SGR_DIM: u8 = 2;

const ANSI_SGR_FG_RED: u8 = 31;
const ANSI_SGR_FG_GREEN: u8 = 32;
const ANSI_SGR_FG_YELLOW: u8 = 33;
const ANSI_SGR_FG_BLUE: u8 = 34;

pub struct SerialLogger;

impl log::Log for SerialLogger {
    fn log(
        &self,
        level: LogLevel,
        target: &str,
        _module_path: &'static str,
        _location: &'static core::panic::Location,
        args: core::fmt::Arguments,
    ) {
        let level_color_code = match level {
            LogLevel::Error => ANSI_SGR_FG_RED,
            LogLevel::Warn => ANSI_SGR_FG_YELLOW,
            LogLevel::Info => ANSI_SGR_FG_GREEN,
            LogLevel::Debug => ANSI_SGR_FG_BLUE,
            LogLevel::Trace => ANSI_SGR_DIM,
        };
        let use_bold = level == LogLevel::Error;

        serial_println!(
            "\x1b[{}m{:<6}\x1b[0m\x1b[2m[{}]\x1b[0m \x1b[{}m{}\x1b[0m",
            level_color_code,
            level.as_str(),
            target,
            if use_bold {
                ANSI_SGR_BOLD
            } else {
                ANSI_SGR_RESET
            },
            args,
        );
    }
}



pub struct SerialPort(u16); // Must be public for macro.

const INTERRUPT_ENABLE_REGISTER: u16 = 1;
const FIFO_CONTROL_REGISTER: u16 = 2;
const LINE_CONTROL_REGISTER: u16 = 3;
const MODEM_CONTROL_REGISTER: u16 = 4;
const LINE_STATUS_REGISTER: u16 = 5;
const _MODEM_STATUS_REGISTER: u16 = 6;

const LINE_STATUS_OUTPUT_EMPTY: u8 = 1 << 5;

impl SerialPort {
    pub const unsafe fn new(port: u16) -> Self {
        Self(port)
    }

    // https://wiki.osdev.org/Serial_Ports#Programming_the_Serial_Communications_Port
    pub fn init(&mut self) {
        const DLAB: u8 = 1 << 7;
        const WORD_LEN_8BIT: u8 = 0b_0000_0011;
        const DIVISOR_LSB: u8 = 3;
        const DIVISOR_MSB: u8 = 0;
        const FIFO_ENABLE: u8 = 0b_0000_0001;
        const FIFO_CLEAR_SEND: u8 = 0b_0000_0100;
        const FIFO_CLEAR_RECV: u8 = 0b_0000_0010;
        const FIFO_INT_LEVEL_14: u8 = 0b_1100_0000;
        const DATA_TERMINAL_READY: u8 = 0b_0000_0001;
        const REQUEST_TO_SEND: u8 = 0b_0000_0010;
        const PIN_OUT2: u8 = 0b_0000_1000;

        unsafe {
            let mut data_port = Port::<u8>::new(self.0);
            let mut interrupt_enable = Port::<u8>::new(self.0 + INTERRUPT_ENABLE_REGISTER);
            let mut fifo_control = Port::<u8>::new(self.0 + FIFO_CONTROL_REGISTER);
            let mut line_control = Port::<u8>::new(self.0 + LINE_CONTROL_REGISTER);
            let mut modem_control = Port::<u8>::new(self.0 + MODEM_CONTROL_REGISTER);

            // Disable interrupts.
            interrupt_enable.write(0);

            // Set the divisor latch access bit (DLAB).
            line_control.write(DLAB);

            // Set the baud rate. See the OSDev article for more:
            //      https://wiki.osdev.org/Serial_Ports#Baud_Rate
            data_port.write(DIVISOR_LSB);
            interrupt_enable.write(DIVISOR_MSB);

            // Finish setting the baud rate by clearing the DLAB, and at the same time set
            // the word length to 8 bits. I know `& !DLAB` isn't doing anything, it's easier
            // to read this way.
            line_control.write(WORD_LEN_8BIT & !DLAB);

            // Enable FIFO, clear it, and set the interrupt trigger level to 14 bytes.
            fifo_control.write(FIFO_ENABLE | FIFO_CLEAR_SEND | FIFO_CLEAR_RECV | FIFO_INT_LEVEL_14);

            // Set the data terminal ready pin, signal request to send, and enable hardware
            // pin OUT2 (enable IRQ).
            modem_control.write(DATA_TERMINAL_READY | REQUEST_TO_SEND | PIN_OUT2);

            // Enable interrupts.
            interrupt_enable.write(1);
        }
    }

    pub fn write(&mut self, byte: u8) {
        match byte {
            0x08 /* BS */ | 0x7F /* DEL */ => {
                self.send(0x08); // Move back 1 character.
                self.send(b' '); // Write a space (also moves forward 1 character).
                self.send(0x08); // Go back to before the space.
            }
            b'\n' => {
                self.send(b'\r');
                self.send(b'\n');
            }

            other => {
                self.send(other);
            }
        }
    }

    pub fn send(&mut self, byte: u8) {
        while !self.try_send(byte) {
            core::hint::spin_loop();
        }
    }

    pub fn try_send(&mut self, byte: u8) -> bool {
        if self.line_output_empty() {
            unsafe {
                Port::<u8>::new(self.0).write(byte);
            }

            true
        } else {
            false
        }
    }

    pub fn read_line_status(&self) -> u8 {
        unsafe { Port::<u8>::new(self.0 + LINE_STATUS_REGISTER).read() }
    }

    pub fn line_output_empty(&self) -> bool {
        self.read_line_status() & LINE_STATUS_OUTPUT_EMPTY == LINE_STATUS_OUTPUT_EMPTY
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write(byte);
        }

        Ok(())
    }
}
