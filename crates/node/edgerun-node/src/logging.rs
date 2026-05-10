//! Node-owned logging boundary.
//!
//! Lower-level crates emit through `edgerun-log`'s tiny global facade. The node
//! owns the installed sink and is the place where those records become host
//! diagnostics, stderr output, or later stream-backed node events.

use core::fmt;

pub use edgerun_log::LogLevel as Level;

pub fn set_level(level: Level) {
    edgerun_log::set_level(level);
}

pub fn level() -> Level {
    edgerun_log::level()
}

pub fn enabled(level: Level) -> bool {
    edgerun_log::enabled(level)
}

pub fn log_args(level: Level, module: &str, args: fmt::Arguments<'_>) {
    edgerun_log::log_args(level, module, args);
}

pub fn trace(module: &str, args: fmt::Arguments<'_>) {
    log_args(Level::Trace, module, args);
}

pub fn debug(module: &str, args: fmt::Arguments<'_>) {
    log_args(Level::Debug, module, args);
}

pub fn info(module: &str, args: fmt::Arguments<'_>) {
    log_args(Level::Info, module, args);
}

pub fn warn(module: &str, args: fmt::Arguments<'_>) {
    log_args(Level::Warn, module, args);
}

pub fn error(module: &str, args: fmt::Arguments<'_>) {
    log_args(Level::Error, module, args);
}

#[cfg(not(target_os = "none"))]
pub fn install_stderr_logger() {
    use std::io::Write;

    edgerun_log::set_format_logger(|level, module, args| {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "[{}] {}: {}", level.as_str(), module, args);
        let _ = stderr.flush();
    });
}

#[cfg(target_os = "none")]
pub fn install_stderr_logger() {
    crate::rt::log::init_serial_logger();
}

#[macro_export]
macro_rules! node_trace {
    ($fmt:literal $(, $($a:tt)+)?) => {
        $crate::logging::trace(module_path!(), core::format_args!($fmt $(, $($a)+)?))
    };
}

#[macro_export]
macro_rules! node_debug {
    ($fmt:literal $(, $($a:tt)+)?) => {
        $crate::logging::debug(module_path!(), core::format_args!($fmt $(, $($a)+)?))
    };
}

#[macro_export]
macro_rules! node_info {
    ($fmt:literal $(, $($a:tt)+)?) => {
        $crate::logging::info(module_path!(), core::format_args!($fmt $(, $($a)+)?))
    };
}

#[macro_export]
macro_rules! node_warn {
    ($fmt:literal $(, $($a:tt)+)?) => {
        $crate::logging::warn(module_path!(), core::format_args!($fmt $(, $($a)+)?))
    };
}

#[macro_export]
macro_rules! node_error {
    ($fmt:literal $(, $($a:tt)+)?) => {
        $crate::logging::error(module_path!(), core::format_args!($fmt $(, $($a)+)?))
    };
}
