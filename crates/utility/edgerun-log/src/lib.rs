//! Minimal no_std logging facade.

#![no_std]

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "tracing-compat")]
pub use tracing::*;

#[cfg(feature = "tracing-compat")]
pub mod tracing {
    pub use tracing::*;
}

#[cfg(feature = "tracing-opentelemetry-compat")]
pub mod tracing_opentelemetry {
    pub use tracing_opentelemetry::*;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[cfg(not(feature = "tracing-compat"))]
pub type Level = LogLevel;

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

pub type Logger = fn(LogLevel, &str, &str);
pub type FormatLogger = fn(LogLevel, &str, fmt::Arguments<'_>);

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(LogLevel::Info as usize);
static LOGGER_FN: AtomicUsize = AtomicUsize::new(0);
static FORMAT_LOGGER_FN: AtomicUsize = AtomicUsize::new(0);

pub fn set_level(level: LogLevel) {
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
}

pub fn level() -> LogLevel {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        0 => LogLevel::Trace,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        _ => LogLevel::Error,
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

pub fn enabled(level: LogLevel) -> bool {
    (level as usize) >= LOG_LEVEL.load(Ordering::Relaxed)
}

#[cfg(feature = "tracing-compat")]
pub fn tracing_level(level: LogLevel) -> tracing::Level {
    match level {
        LogLevel::Trace => tracing::Level::TRACE,
        LogLevel::Debug => tracing::Level::DEBUG,
        LogLevel::Info => tracing::Level::INFO,
        LogLevel::Warn => tracing::Level::WARN,
        LogLevel::Error => tracing::Level::ERROR,
    }
}

#[cfg(feature = "tracing-compat")]
pub fn from_tracing_level(level: tracing::Level) -> LogLevel {
    match level {
        tracing::Level::TRACE => LogLevel::Trace,
        tracing::Level::DEBUG => LogLevel::Debug,
        tracing::Level::INFO => LogLevel::Info,
        tracing::Level::WARN => LogLevel::Warn,
        tracing::Level::ERROR => LogLevel::Error,
    }
}

pub fn log(level: LogLevel, module: &str, message: &str) {
    log_args(level, module, format_args!("{message}"));
}

pub fn log_args(level: LogLevel, module: &str, args: fmt::Arguments<'_>) {
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

pub fn write(level: LogLevel, module: &str, value: impl fmt::Display) {
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

#[cfg(not(feature = "tracing-compat"))]
#[macro_export]
macro_rules! trace {
    ($msg:expr) => {
        $crate::log($crate::LogLevel::Trace, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::LogLevel::Trace, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[cfg(not(feature = "tracing-compat"))]
#[macro_export]
macro_rules! debug {
    ($msg:expr) => {
        $crate::log($crate::LogLevel::Debug, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::LogLevel::Debug, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[cfg(not(feature = "tracing-compat"))]
#[macro_export]
macro_rules! info {
    ($msg:expr) => {
        $crate::log($crate::LogLevel::Info, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::LogLevel::Info, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[cfg(not(feature = "tracing-compat"))]
#[macro_export]
macro_rules! warn {
    ($msg:expr) => {
        $crate::log($crate::LogLevel::Warn, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::LogLevel::Warn, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[cfg(not(feature = "tracing-compat"))]
#[macro_export]
macro_rules! error {
    ($msg:expr) => {
        $crate::log($crate::LogLevel::Error, module_path!(), $msg)
    };
    ($fmt:literal, $($a:expr),* $(,)?) => {
        $crate::log_args($crate::LogLevel::Error, module_path!(), core::format_args!($fmt, $($a),*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static LAST_LEN: AtomicUsize = AtomicUsize::new(0);

    fn test_logger(_level: LogLevel, _module: &str, message: &str) {
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
        set_level(LogLevel::Trace);

        log_args(LogLevel::Info, "test", format_args!("value={}", 42));

        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(LAST_LEN.load(Ordering::SeqCst), "value=42".len());
        clear_logger();
    }

    #[test]
    fn filters_below_current_level() {
        CALLS.store(0, Ordering::SeqCst);
        clear_format_logger();
        set_logger(test_logger);
        set_level(LogLevel::Warn);

        log_args(LogLevel::Info, "test", format_args!("value={}", 42));

        assert_eq!(CALLS.load(Ordering::SeqCst), 0);
        clear_logger();
        set_level(LogLevel::Info);
    }
}
