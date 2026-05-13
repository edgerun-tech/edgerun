//! Minimal logging facade with a tracing-compatible surface.

#![no_std]

use core::fmt::{self, Write};
use core::future::Future;
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "tracing-compat")]
pub use edgerun_log_macros::instrument;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Level(LogLevel);

impl Level {
    pub const TRACE: Self = Self(LogLevel::Trace);
    pub const DEBUG: Self = Self(LogLevel::Debug);
    pub const INFO: Self = Self(LogLevel::Info);
    pub const WARN: Self = Self(LogLevel::Warn);
    pub const ERROR: Self = Self(LogLevel::Error);

    pub const fn as_log_level(self) -> LogLevel {
        self.0
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        Self(level)
    }
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        level.as_log_level()
    }
}

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

pub fn enabled(level: impl Into<Level>) -> bool {
    (level.into().as_log_level() as usize) >= LOG_LEVEL.load(Ordering::Relaxed)
}

pub fn log(level: LogLevel, module: &str, message: &str) {
    log_args(level, module, format_args!("{message}"));
}

pub fn log_args(level: LogLevel, module: &str, args: fmt::Arguments<'_>) {
    if !enabled(Level(level)) {
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

pub fn tracing_level(level: LogLevel) -> Level {
    Level(level)
}

pub fn from_tracing_level(level: Level) -> LogLevel {
    level.as_log_level()
}

#[derive(Clone, Debug, Default)]
pub struct Span;

impl Span {
    pub fn current() -> Self {
        Self
    }

    pub fn none() -> Self {
        Self
    }

    pub fn new() -> Self {
        Self
    }

    pub fn enter(&self) -> Entered {
        Entered
    }

    pub fn entered(self) -> Entered {
        Entered
    }

    pub fn record(&self, _field: &str, _value: impl fmt::Debug) {}

    pub fn in_scope<T>(&self, f: impl FnOnce() -> T) -> T {
        f()
    }

    pub fn metadata(&self) -> Option<&'static Metadata<'static>> {
        None
    }
}

#[derive(Debug)]
pub struct Entered;

#[derive(Clone, Copy, Debug)]
pub struct Metadata<'a> {
    target: &'a str,
    level: Level,
}

impl<'a> Metadata<'a> {
    pub const fn new(target: &'a str, level: Level) -> Self {
        Self { target, level }
    }

    pub const fn target(&self) -> &'a str {
        self.target
    }

    pub const fn level(&self) -> &Level {
        &self.level
    }
}

pub trait Instrument: Sized {
    fn instrument(self, _span: Span) -> Self {
        self
    }

    fn in_current_span(self) -> Self {
        self
    }
}

impl<T> Instrument for T where T: Future {}

pub mod field {
    use core::fmt;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Empty;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Field;

    pub trait Visit {
        fn record_bool(&mut self, _field: &Field, _value: bool) {}
        fn record_str(&mut self, _field: &Field, _value: &str) {}
        fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {}
    }

    pub struct DebugValue<'a, T: ?Sized>(&'a T);

    impl<T: fmt::Debug + ?Sized> fmt::Debug for DebugValue<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0.fmt(f)
        }
    }

    pub fn debug<T: fmt::Debug + ?Sized>(value: &T) -> DebugValue<'_, T> {
        DebugValue(value)
    }
}

pub mod dispatcher {
    #[derive(Debug)]
    pub struct DefaultGuard;

    pub fn with_default<T>(_dispatch: &crate::Dispatch, f: impl FnOnce() -> T) -> T {
        f()
    }
}

#[derive(Debug)]
pub struct Dispatch;

impl Dispatch {
    pub fn new<T>(_subscriber: T) -> Self {
        Self
    }
}

pub mod subscriber {
    pub fn set_default<T>(_subscriber: T) -> crate::dispatcher::DefaultGuard {
        crate::dispatcher::DefaultGuard
    }
}

pub trait Subscriber {}

pub struct Event<'a> {
    metadata: Metadata<'a>,
}

impl<'a> Event<'a> {
    pub const fn new(metadata: Metadata<'a>) -> Self {
        Self { metadata }
    }

    pub const fn metadata(&self) -> &Metadata<'a> {
        &self.metadata
    }

    pub fn record(&self, _visitor: &mut dyn field::Visit) {}
}

#[macro_export]
macro_rules! enabled {
    ($level:expr $(,)?) => {
        $crate::enabled($level)
    };
    ($($arg:tt)*) => {
        false
    };
}

#[macro_export]
macro_rules! event {
    ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{}};
}

#[macro_export]
macro_rules! info_span {
    ($($arg:tt)*) => {
        $crate::Span::new()
    };
}

#[macro_export]
macro_rules! debug_span {
    ($($arg:tt)*) => {
        $crate::Span::new()
    };
}

#[macro_export]
macro_rules! trace_span {
    ($($arg:tt)*) => {
        $crate::Span::new()
    };
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

#[cfg(feature = "tracing-opentelemetry-compat")]
pub mod tracing_opentelemetry {
    pub trait OpenTelemetrySpanExt {}

    impl OpenTelemetrySpanExt for crate::Span {}
}
