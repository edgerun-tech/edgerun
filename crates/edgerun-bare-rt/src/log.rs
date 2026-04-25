//! Logging - built-in minimal logger.

#![no_std]

extern crate alloc;

use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(Level::Info as usize);

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

type LoggerFn = Option<fn(Level, &str, &str)>;

static LOGGER: AtomicUsize = AtomicUsize::new(0);

pub fn set_logger(logger: fn(Level, &str, &str)) {
    LOGGER.store(logger as usize, Ordering::Relaxed);
}

pub fn log(level: Level, module: &str, message: &str) {
    if (level as usize) < LOG_LEVEL.load(Ordering::Relaxed) {
        return;
    }
    let logger_fn = unsafe { core::mem::transmute::<usize, LoggerFn>(LOGGER.load(Ordering::Relaxed)) };
    if let Some(logger) = logger_fn {
        logger(level, module, message);
    }
}