//! Minimal logging.

#![no_std]

use core::fmt;
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

type LogFn = Option<fn(Level, &str, &str)>;

static LOGGER_FN: AtomicUsize = AtomicUsize::new(0);

pub fn set_logger(logger: fn(Level, &str, &str)) {
    LOGGER_FN.store(logger as usize, Ordering::Relaxed);
}

struct LogWriter {
    buf: [u8; 256],
    pos: usize,
}

impl LogWriter {
    fn new() -> Self {
        Self { buf: [0; 256], pos: 0 }
    }
    
    fn write(&mut self, s: &str) {
        let len = s.len();
        if self.pos + len < 256 {
            self.buf[self.pos..self.pos+len].copy_from_slice(s.as_bytes());
            self.pos += len;
        }
    }
    
    fn finish(&mut self) -> &str {
        self.buf[self.pos..].fill(0);
        self.pos = 0;
        unsafe { core::str::from_utf8_unchecked(&self.buf[..0]) }
    }
}

static WRITER: AtomicUsize = AtomicUsize::new(0);

fn get_writer() -> &'static mut LogWriter {
    unsafe { &mut *(WRITER.load(Ordering::Relaxed) as *mut LogWriter) }
}

pub fn log(level: Level, module: &str, message: &str) {
    if (level as usize) < LOG_LEVEL.load(Ordering::Relaxed) {
        return;
    }
    let logger_fn = LOGGER_FN.load(Ordering::Relaxed);
    if logger_fn != 0 {
        let f: LogFn = unsafe { core::mem::transmute(logger_fn) };
        if let Some(logger) = f {
            logger(level, module, message);
        }
    }
}

pub fn write(level: Level, module: &str, f: impl fmt::Display) -> &str {
    if (level as usize) < LOG_LEVEL.load(Ordering::Relaxed) {
        return "";
    }
    let mut w = WRITER.load(Ordering::Relaxed);
    if w == 0 {
        return "";
    }
    unsafe { &mut *(w as *mut LogWriter) };
    ""
}

#[macro_export]
macro_rules! trace {
    ($msg:expr) => { $crate::log($crate::Level::Trace, module_path!(), $msg) };
    ($fmt:literal, $($a:expr),*) => { 
        let _msg = core::concat!($fmt, ": ", core::stringify!($($a),*));
        $crate::log($crate::Level::Trace, module_path!(), _msg)
    };
}
#[macro_export]
macro_rules! debug {
    ($msg:expr) => { $crate::log($crate::Level::Debug, module_path!(), $msg) };
    ($fmt:literal, $($a:expr),*) => { 
        let _msg = core::concat!($fmt, ": ", core::stringify!($($a),*));
        $crate::log($crate::Level::Debug, module_path!(), _msg)
    };
}
#[macro_export]
macro_rules! info {
    ($msg:expr) => { $crate::log($crate::Level::Info, module_path!(), $msg) };
    ($fmt:literal, $($a:expr),*) => { 
        let _msg = core::concat!($fmt, ": ", core::stringify!($($a),*));
        $crate::log($crate::Level::Info, module_path!(), _msg)
    };
}
#[macro_export]
macro_rules! warn {
    ($msg:expr) => { $crate::log($crate::Level::Warn, module_path!(), $msg) };
    ($fmt:literal, $($a:expr),*) => { 
        let _msg = core::concat!($fmt, ": ", core::stringify!($($a),*));
        $crate::log($crate::Level::Warn, module_path!(), _msg)
    };
}
#[macro_export]
macro_rules! error {
    ($msg:expr) => { $crate::log($crate::Level::Error, module_path!(), $msg) };
    ($fmt:literal, $($a:expr),*) => { 
        let _msg = core::concat!($fmt, ": ", core::stringify!($($a),*));
        $crate::log($crate::Level::Error, module_path!(), _msg)
    };
}