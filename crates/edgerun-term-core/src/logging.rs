use once_cell::sync::OnceCell;

/// Initialize global logging with a debug-friendly default so diagnostics are always available.
#[cfg(not(target_arch = "wasm32"))]
pub fn init() {
    static INIT: OnceCell<()> = OnceCell::new();
    INIT.get_or_init(|| {
        let _ = edgerun_observability::init(edgerun_observability::InitOptions {
            service_name: "edgerun-term-core",
            default_level: "debug",
            enable_log_bridge: true,
        });
        log::set_max_level(log::LevelFilter::Debug);
    });
}

#[cfg(target_arch = "wasm32")]
pub fn init() {
    static INIT: OnceCell<()> = OnceCell::new();
    INIT.get_or_init(|| {});
}
