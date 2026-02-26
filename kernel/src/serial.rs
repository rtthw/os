use {lazy_static::lazy_static, spin::Mutex, uart_16550::SerialPort};



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



pub struct SerialLogger;

impl log::Log for SerialLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        serial_println!(
            "{} [{}]: {}",
            record.level(),
            record.module_path().unwrap(),
            record.args(),
        );
    }

    fn flush(&self) {}
}
