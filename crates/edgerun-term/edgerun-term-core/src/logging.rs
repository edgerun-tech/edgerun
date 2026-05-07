use std::env;
use std::io::Write;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_log::Level;

/// Initialize global logging with a debug-friendly default so diagnostics are always available.
pub fn init() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        edgerun_log::set_format_logger(write_log);
        edgerun_log::set_level(level_from_env(Level::Debug));
    });
}

fn level_from_env(default: Level) -> Level {
    env::var("RUST_LOG")
        .ok()
        .and_then(|value| parse_level(&value))
        .unwrap_or(default)
}

fn parse_level(value: &str) -> Option<Level> {
    match value.trim().to_ascii_lowercase().as_str() {
        "error" => Some(Level::Error),
        "warn" | "warning" => Some(Level::Warn),
        "info" => Some(Level::Info),
        "debug" => Some(Level::Debug),
        "trace" => Some(Level::Trace),
        _ => None,
    }
}

fn write_log(level: Level, module: &str, args: std::fmt::Arguments<'_>) {
    let mut stderr = std::io::stderr();
    if env::var("TERM_LOG_TIMESTAMPS").is_ok() {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let _ = writeln!(
            stderr,
            "{millis} [{:<5}] {:<30} {}",
            level.as_str(),
            module,
            args
        );
    } else {
        let _ = writeln!(stderr, "[{:<5}] {:<30} {}", level.as_str(), module, args);
    }
}
