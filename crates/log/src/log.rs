//! # Logging
//!
//! Based on [the `log` crate](https://crates.io/crates/log).

#![no_std]

use core::{
    fmt::Arguments,
    panic::Location,
    sync::atomic::{AtomicUsize, Ordering},
};

static mut LOGGER: &dyn Log = &DummyLogger;

static STATE: AtomicUsize = AtomicUsize::new(0);

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;
const INITIALIZED: usize = 2;

pub trait Log: Sync + Send {
    fn log(
        &self,
        level: LogLevel,
        target: &str,
        module_path: &'static str,
        location: &'static Location,
        args: Arguments,
    );
}

pub fn get_logger() -> &'static dyn Log {
    if STATE.load(Ordering::Acquire) != INITIALIZED {
        static NOP: DummyLogger = DummyLogger;
        &NOP
    } else {
        unsafe { LOGGER }
    }
}

pub fn set_logger(logger: &'static dyn Log) -> Result<(), ()> {
    match STATE.compare_exchange(
        UNINITIALIZED,
        INITIALIZING,
        Ordering::Acquire,
        Ordering::Relaxed,
    ) {
        Ok(UNINITIALIZED) => {
            unsafe {
                LOGGER = logger;
            }
            STATE.store(INITIALIZED, Ordering::Release);

            Ok(())
        }
        Err(INITIALIZING) => {
            while STATE.load(Ordering::Relaxed) == INITIALIZING {
                core::hint::spin_loop();
            }

            Err(())
        }

        _ => Err(()),
    }
}



#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LogLevel {
    Error = 1,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub const fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}



#[macro_export]
macro_rules! error {
    // error!(target: "thing", "some {} message", "error")
    (target: $target:expr, $($arg:tt)+) => ({
        $crate::log!(target: $target, $crate::LogLevel::Error, $($arg)+)
    });

    // error!("some {} message", "error")
    ($($arg:tt)+) => ($crate::log!($crate::LogLevel::Error, $($arg)+))
}

#[macro_export]
macro_rules! warn {
    // warn!(target: "thing", "some {}", "warning")
    (target: $target:expr, $($arg:tt)+) => ({
        $crate::log!(target: $target, $crate::LogLevel::Warn, $($arg)+)
    });

    // warn!("some {}", "warning")
    ($($arg:tt)+) => ($crate::log!($crate::LogLevel::Warn, $($arg)+))
}

#[macro_export]
macro_rules! info {
    // info!(target: "thing", "some {} message", "info")
    (target: $target:expr, $($arg:tt)+) => ({
        $crate::log!(target: $target, $crate::LogLevel::Info, $($arg)+)
    });

    // info!("some {} message", "info")
    ($($arg:tt)+) => ($crate::log!($crate::LogLevel::Info, $($arg)+))
}

#[macro_export]
macro_rules! debug {
    // debug!(target: "thing", "some {} message", "debug")
    (target: $target:expr, $($arg:tt)+) => ({
        $crate::log!(target: $target, $crate::LogLevel::Debug, $($arg)+)
    });

    // debug!("some {} message", "debug")
    ($($arg:tt)+) => ($crate::log!($crate::LogLevel::Debug, $($arg)+))
}

#[macro_export]
macro_rules! trace {
    // trace!(target: "thing", "some {} message", "trace")
    (target: $target:expr, $($arg:tt)+) => ({
        $crate::log!(target: $target, $crate::LogLevel::Trace, $($arg)+)
    });

    // trace!("some {} message", "trace")
    ($($arg:tt)+) => ($crate::log!($crate::LogLevel::Trace, $($arg)+))
}

#[macro_export]
macro_rules! log {
    // log!(target: "thing", LogLevel::Info, "some info message")
    (target: $target:expr, $level:expr, $($arg:tt)+) => ({
        $crate::__private::log(
            $level,
            $target,
            $crate::__private::module_path!(),
            $crate::__private::location(),
            $crate::__private::format_args!($($arg)+),
        );
    });

    // log!(LogLevel::Info, "some info message")
    ($level:expr, $($arg:tt)+) => ({
        $crate::log!(
            target: $crate::__private::module_path!(),
            $level,
            $($arg)+
        )
    });
}



struct DummyLogger;

impl Log for DummyLogger {
    fn log(
        &self,
        _level: LogLevel,
        _target: &str,
        _module_path: &'static str,
        _location: &'static Location,
        _args: Arguments,
    ) {
    }
}

pub mod __private {
    use core::{fmt::Arguments, panic::Location};

    pub use core::{format_args, module_path};


    pub fn log(
        level: super::LogLevel,
        target: &str,
        module_path: &'static str,
        location: &'static Location,
        args: Arguments,
    ) {
        super::get_logger().log(level, target, module_path, location, args);
    }

    #[track_caller]
    pub const fn location() -> &'static Location<'static> {
        Location::caller()
    }
}
