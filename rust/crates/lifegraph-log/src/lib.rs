//! Minimal structured logging — replaces `tracing` + `tracing-subscriber`.

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level { Trace = 0, Debug = 1, Info = 2, Warn = 3, Error = 4 }

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE", Level::Debug => "DEBUG", Level::Info => "INFO",
            Level::Warn => "WARN", Level::Error => "ERROR",
        }
    }
}

static LOG_LEVEL: AtomicUsize = AtomicUsize::new(Level::Info as usize);

pub fn init_from_env() {
    let var = env::var("RUST_LOG")
        .or_else(|_| env::var("LIFEGRAH_LOG"))
        .unwrap_or_else(|_| "info".to_string());
    let level = match var.to_lowercase().as_str() {
        "trace" => Level::Trace, "debug" => Level::Debug,
        "info" | "" => Level::Info, "warn" => Level::Warn,
        "error" => Level::Error, _ => Level::Info,
    };
    LOG_LEVEL.store(level as usize, Ordering::Relaxed);
}

pub fn set_level(level: Level) { LOG_LEVEL.store(level as usize, Ordering::Relaxed); }

pub fn log(level: Level, module: &str, message: &str) {
    if (level as usize) < LOG_LEVEL.load(Ordering::Relaxed) { return; }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    eprintln!("{}.{:03} {} [{}] {}", now.as_secs(), now.subsec_millis(), level.as_str(), module, message);
}

#[macro_export] macro_rules! trace { ($($tt:tt)*) => { $crate::__log!($crate::Level::Trace, $($tt)*) }; }
#[macro_export] macro_rules! debug { ($($tt:tt)*) => { $crate::__log!($crate::Level::Debug, $($tt)*) }; }
#[macro_export] macro_rules! info  { ($($tt:tt)*) => { $crate::__log!($crate::Level::Info, $($tt)*) }; }
#[macro_export] macro_rules! warn  { ($($tt:tt)*) => { $crate::__log!($crate::Level::Warn, $($tt)*) }; }
#[macro_export] macro_rules! error { ($($tt:tt)*) => { $crate::__log!($crate::Level::Error, $($tt)*) }; }

/// The core macro: captures everything after the level, finds the format string,
/// and formats all tokens into the final message.
#[doc(hidden)]
#[macro_export]
macro_rules! __log {
    ($level:expr, ) => {};
    ($level:expr, $fmt:literal) => {{
        $crate::log($level, module_path!(), &$fmt.to_string());
    }};
    ($level:expr, $fmt:literal $(, $args:expr)+ $(,)?) => {{
        $crate::log($level, module_path!(), &format!($fmt, $($args),+));
    }};
    ($level:expr, $($all:tt)*) => {{
        // For any other pattern (key = value, "fmt", args...),
        // just format everything as a string.
        // We stringify and pass through format! with a catch-all approach.
        let _formatted = format!($($all)*);
        $crate::log($level, module_path!(), &_formatted);
    }};
}
