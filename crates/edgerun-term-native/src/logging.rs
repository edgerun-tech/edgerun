use once_cell::sync::OnceCell;

/// Initialize global logging with a debug-friendly default so diagnostics are always available.
pub fn init(level: log::LevelFilter) {
    static INIT: OnceCell<()> = OnceCell::new();
    INIT.get_or_init(|| {
        let default_level = match level {
            log::LevelFilter::Off => "off",
            log::LevelFilter::Error => "error",
            log::LevelFilter::Warn => "warn",
            log::LevelFilter::Info => "info",
            log::LevelFilter::Debug => "debug",
            log::LevelFilter::Trace => "trace",
        };
        let _ = edgerun_observability::init(edgerun_observability::InitOptions {
            service_name: "edgerun-term-native",
            default_level,
            enable_log_bridge: true,
        });
        log::set_max_level(level);
    });
}

pub fn set_level(level: log::LevelFilter) {
    log::set_max_level(level);
}
