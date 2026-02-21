#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;

#[cfg(not(target_arch = "wasm32"))]
use env_logger::Env;
use once_cell::sync::OnceCell;

/// Initialize global logging with a debug-friendly default so diagnostics are always available.
#[cfg(not(target_arch = "wasm32"))]
pub fn init() {
    static INIT: OnceCell<()> = OnceCell::new();
    INIT.get_or_init(|| {
        let env = Env::default().filter_or("RUST_LOG", "debug");
        let mut builder = env_logger::Builder::from_env(env);
        builder
            .format(|buf, record| {
                writeln!(
                    buf,
                    "[{:<5}] {:<30} {}",
                    record.level(),
                    record.target(),
                    record.args()
                )
            })
            .filter_module("wgpu_core", log::LevelFilter::Error)
            .filter_module("wgpu_hal", log::LevelFilter::Error);

        if env::var("TERM_LOG_TIMESTAMPS").is_ok() {
            builder.format_timestamp_millis();
        } else {
            builder.format_timestamp(None);
        }

        builder.init();
    });
}

#[cfg(target_arch = "wasm32")]
pub fn init() {
    static INIT: OnceCell<()> = OnceCell::new();
    INIT.get_or_init(|| {});
}
