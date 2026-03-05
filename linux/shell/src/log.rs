//! # Logging

use anyhow::Result;
use log::{Level, LevelFilter, Log};

pub struct Logger {
    pub use_stderr: bool,
    pub default_level: LevelFilter,
    pub targets: Vec<(String, LevelFilter)>,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            use_stderr: false,
            default_level: LevelFilter::Trace,
            targets: Vec::new(),
        }
    }
}

impl Logger {
    pub fn with_target(mut self, target: &str, level: LevelFilter) -> Logger {
        self.targets.push((target.to_string(), level));
        self.targets
            .sort_by_key(|(name, _level)| name.len().wrapping_neg());
        self
    }

    pub fn max_level(&self) -> LevelFilter {
        let max_level = self
            .targets
            .iter()
            .map(|(_name, level)| level)
            .copied()
            .max();
        max_level
            .map(|lvl| lvl.max(self.default_level))
            .unwrap_or(self.default_level)
    }

    pub fn init(self) -> Result<()> {
        log::set_max_level(self.max_level());
        Ok(log::set_boxed_logger(Box::new(self))?)
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        &metadata.level().to_level_filter()
            <= self
                .targets
                .iter()
                .find(|(name, _level)| metadata.target().starts_with(name))
                .map(|(_name, level)| level)
                .unwrap_or(&self.default_level)
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level_color_code = match record.level() {
            Level::Error => 31, // ANSI SGR "red"
            Level::Warn => 33,  // ANSI SGR "yellow"
            Level::Info => 0,   // ANSI SGR "reset"
            Level::Debug => 34, // ANSI SGR "blue"
            Level::Trace => 2,  // ANSI SGR "dim"
        };
        let target = if !record.target().is_empty() {
            record.target()
        } else {
            record.module_path().unwrap_or_default()
        };
        let use_bold = record.level() == Level::Error;

        let msg = format!(
            "\x1b[{}m{:<6}\x1b[0m\x1b[2m[{}]\x1b[0m \x1b[{}m{}\x1b[0m",
            level_color_code,
            record.level().as_str(),
            target,
            if use_bold {
                1 // ANSI SGR "bold"
            } else {
                0 // ANSI SGR "reset"
            },
            record.args(),
        );

        if self.use_stderr {
            eprintln!("{msg}");
        } else {
            println!("{msg}");
        }
    }

    fn flush(&self) {}
}
