//! Minimal no_std logging facade.

#![no_std]

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub type Logger = fn(Level, &str, &str);
pub type FormatLogger = fn(Level, &str, fmt::Arguments<'_>);

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(Level::Info as usize);
static LOGGER_FN: AtomicUsize = AtomicUsize::new(0);
static FORMAT_LOGGER_FN: AtomicUsize = AtomicUsize::new(0);

pub fn set_level(level: Level) {
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
}

pub fn level() -> Level {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        0 => Level::Trace,
        1 => Level::Debug,
        2 => Level::Info,
        3 => Level::Warn,
        _ => Level::Error,
    }
}

pub fn init_from_env() {}

pub fn set_logger(logger: Logger) {
    LOGGER_FN.store(logger as usize, Ordering::Relaxed);
}

pub fn clear_logger() {
    LOGGER_FN.store(0, Ordering::Relaxed);
}

pub fn set_format_logger(logger: FormatLogger) {
    FORMAT_LOGGER_FN.store(logger as usize, Ordering::Relaxed);
}

pub fn clear_format_logger() {
    FORMAT_LOGGER_FN.store(0, Ordering::Relaxed);
}

pub fn enabled(level: Level) -> bool {
    (level as usize) >= LOG_LEVEL.load(Ordering::Relaxed)
}

pub fn log(level: Level, module: &str, message: &str) {
    log_args(level, module, format_args!("{message}"));
}

pub fn log_args(level: Level, module: &str, args: fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }

    let format_logger = FORMAT_LOGGER_FN.load(Ordering::Relaxed);
    if format_logger != 0 {
        let logger: FormatLogger = unsafe { core::mem::transmute(format_logger) };
        logger(level, module, args);
        return;
    }

    let logger = LOGGER_FN.load(Ordering::Relaxed);
    if logger == 0 {
        return;
    }

    let mut buffer = FixedBuffer::new();
    let _ = buffer.write_fmt(args);
    let logger: Logger = unsafe { core::mem::transmute(logger) };
    logger(level, module, buffer.as_str());
}

pub fn write(level: Level, module: &str, value: impl fmt::Display) {
    log_args(level, module, format_args!("{value}"));
}

struct FixedBuffer {
    bytes: [u8; 512],
    len: usize,
}

impl FixedBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0; 512],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("<invalid log message>")
    }
}

impl Write for FixedBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let available = self.bytes.len().saturating_sub(self.len);
        if available == 0 {
            return Ok(());
        }

        let mut copy_len = available.min(s.len());
        while !s.is_char_boundary(copy_len) {
            copy_len -= 1;
        }
        self.bytes[self.len..self.len + copy_len].copy_from_slice(&s.as_bytes()[..copy_len]);
        self.len += copy_len;
        Ok(())
    }
}

#[macro_export]
macro_rules! trace {
    ($msg:expr) => {
        $crate::log($crate::Level::Trace, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::Level::Trace, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[macro_export]
macro_rules! debug {
    ($msg:expr) => {
        $crate::log($crate::Level::Debug, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::Level::Debug, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[macro_export]
macro_rules! info {
    ($msg:expr) => {
        $crate::log($crate::Level::Info, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::Level::Info, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[macro_export]
macro_rules! warn {
    ($msg:expr) => {
        $crate::log($crate::Level::Warn, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::Level::Warn, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[macro_export]
macro_rules! error {
    ($msg:expr) => {
        $crate::log($crate::Level::Error, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::Level::Error, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static LAST_LEN: AtomicUsize = AtomicUsize::new(0);

    fn test_logger(_level: Level, _module: &str, message: &str) {
        CALLS.fetch_add(1, Ordering::SeqCst);
        LAST_LEN.store(message.len(), Ordering::SeqCst);
        assert_eq!(message, "value=42");
    }

    #[test]
    fn formats_arguments_before_calling_legacy_logger() {
        CALLS.store(0, Ordering::SeqCst);
        LAST_LEN.store(0, Ordering::SeqCst);
        clear_format_logger();
        set_logger(test_logger);
        set_level(Level::Trace);

        log_args(Level::Info, "test", format_args!("value={}", 42));

        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(LAST_LEN.load(Ordering::SeqCst), "value=42".len());
        clear_logger();
    }

    #[test]
    fn filters_below_current_level() {
        CALLS.store(0, Ordering::SeqCst);
        clear_format_logger();
        set_logger(test_logger);
        set_level(Level::Warn);

        log_args(Level::Info, "test", format_args!("value={}", 42));

        assert_eq!(CALLS.load(Ordering::SeqCst), 0);
        clear_logger();
        set_level(Level::Info);
    }
}
