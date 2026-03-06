use {lazy_static::lazy_static, log::Level, spin::Mutex, uart_16550::SerialPort};



lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut port = unsafe { SerialPort::new(0x3F8) };
        port.init();
        Mutex::new(port)
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use {core::fmt::Write, x86_64::instructions::interrupts::without_interrupts};

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
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level_color_code = match record.level() {
            Level::Error => ANSI_SGR_FG_RED,
            Level::Warn => ANSI_SGR_FG_YELLOW,
            Level::Info => ANSI_SGR_FG_GREEN,
            Level::Debug => ANSI_SGR_FG_BLUE,
            Level::Trace => ANSI_SGR_DIM,
        };
        let target = if !record.target().is_empty() {
            record.target()
        } else {
            record.module_path().unwrap_or_default()
        };
        let use_bold = record.level() == Level::Error;

        serial_println!(
            "\x1b[{}m{:<6}\x1b[0m\x1b[2m[{}]\x1b[0m \x1b[{}m{}\x1b[0m",
            level_color_code,
            record.level().as_str(),
            target,
            if use_bold {
                ANSI_SGR_BOLD
            } else {
                ANSI_SGR_RESET
            },
            record.args(),
        );
    }

    fn flush(&self) {}
}
