//! Minimal structured logging — replaces `tracing` + `tracing-subscriber`.

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(Level::Trace < Level::Debug);
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn level_equality() {
        assert_eq!(Level::Trace, Level::Trace);
        assert_eq!(Level::Debug, Level::Debug);
        assert_eq!(Level::Info, Level::Info);
        assert_eq!(Level::Warn, Level::Warn);
        assert_eq!(Level::Error, Level::Error);
    }

    #[test]
    fn level_as_str() {
        assert_eq!(Level::Trace.as_str(), "TRACE");
        assert_eq!(Level::Debug.as_str(), "DEBUG");
        assert_eq!(Level::Info.as_str(), "INFO");
        assert_eq!(Level::Warn.as_str(), "WARN");
        assert_eq!(Level::Error.as_str(), "ERROR");
    }

    #[test]
    fn level_clone_copy() {
        let level = Level::Debug;
        let cloned = level.clone();
        assert_eq!(level, cloned);
        // Copy trait
        let _copied = level;
        let _also = level;
    }

    #[test]
    fn init_from_env_defaults_to_info() {
        // With no env vars set, default is Info
        std::env::remove_var("RUST_LOG");
        std::env::remove_var("LIFEGRAH_LOG");
        init_from_env();
        // Verify by checking that Debug-level log would be suppressed
        assert!((Level::Debug as usize) < LOG_LEVEL.load(Ordering::Relaxed));
        assert!((Level::Info as usize) >= LOG_LEVEL.load(Ordering::Relaxed));
    }

    #[test]
    fn set_level_changes_threshold() {
        set_level(Level::Debug);
        assert_eq!(LOG_LEVEL.load(Ordering::Relaxed), Level::Debug as usize);
        set_level(Level::Error);
        assert_eq!(LOG_LEVEL.load(Ordering::Relaxed), Level::Error as usize);
        set_level(Level::Trace);
        assert_eq!(LOG_LEVEL.load(Ordering::Relaxed), Level::Trace as usize);
    }

    #[test]
    fn log_respects_level_filter() {
        set_level(Level::Warn);
        // Debug should not log (below threshold) — we can't easily capture stderr,
        // but we can verify the level check logic
        let threshold = LOG_LEVEL.load(Ordering::Relaxed);
        assert!((Level::Debug as usize) < threshold);
        assert!((Level::Info as usize) < threshold);
        assert!((Level::Warn as usize) >= threshold);
        assert!((Level::Error as usize) >= threshold);
    }

    #[test]
    fn log_outputs_at_or_above_threshold() {
        set_level(Level::Info);
        let threshold = LOG_LEVEL.load(Ordering::Relaxed);
        assert!((Level::Info as usize) >= threshold);
        assert!((Level::Warn as usize) >= threshold);
        assert!((Level::Error as usize) >= threshold);
    }

    #[test]
    fn trace_macro_expands() {
        set_level(Level::Trace);
        // These should compile without error
        trace!("test trace message");
        trace!("test trace with arg: {}", 42);
    }

    #[test]
    fn debug_macro_expands() {
        set_level(Level::Debug);
        debug!("test debug message");
        debug!("test debug with arg: {}", "hello");
    }

    #[test]
    fn info_macro_expands() {
        set_level(Level::Info);
        info!("test info message");
        info!("test info with arg: {}", 123);
    }

    #[test]
    fn warn_macro_expands() {
        set_level(Level::Warn);
        warn!("test warn message");
        warn!("test warn with arg: {}", "warning");
    }

    #[test]
    fn error_macro_expands() {
        set_level(Level::Error);
        error!("test error message");
        error!("test error with arg: {}", "error");
    }

    #[test]
    fn log_macro_with_no_args() {
        set_level(Level::Trace);
        // Empty log should compile
        info!();
    }

    #[test]
    fn log_macro_with_literal_only() {
        set_level(Level::Trace);
        info!("just a literal");
    }

    #[test]
    fn log_macro_with_format_args() {
        set_level(Level::Trace);
        info!("value is {}", 42);
        info!("a={} b={}", 1, 2);
        info!("a={} b={} c={}", 1, 2, 3);
    }
}
