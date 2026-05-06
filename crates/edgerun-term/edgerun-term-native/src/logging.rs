use std::env;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

use env_logger::Env;
use once_cell::sync::OnceCell;

/// Initialize global logging with a debug-friendly default so diagnostics are always available.
pub fn init(level: log::LevelFilter) {
    static INIT: OnceCell<()> = OnceCell::new();
    INIT.get_or_init(|| {
        let env = Env::default().filter_or("RUST_LOG", level.as_str());
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

        if let Some(path) = log_file_path() {
            if let Some(parent) = path.parent() {
                let _ = create_dir_all(parent);
            }
            if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
                builder.target(env_logger::Target::Pipe(Box::new(file)));
            }
        }

        if env::var("TERM_LOG_TIMESTAMPS").is_ok() {
            builder.format_timestamp_millis();
        } else {
            builder.format_timestamp(None);
        }

        builder.init();
    });
}

pub fn set_level(level: log::LevelFilter) {
    log::set_max_level(level);
}

fn log_file_path() -> Option<PathBuf> {
    if let Some(dir) = dirs::data_dir() {
        return Some(dir.join("term").join("logs").join("term.log"));
    }
    env::var_os("HOME").map(PathBuf::from).map(|home| {
        home.join(".local")
            .join("share")
            .join("term")
            .join("logs")
            .join("term.log")
    })
}
