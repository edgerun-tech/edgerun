//! Bare-metal adapter for the central `edgerun-log` crate.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

pub use edgerun_log::{
    clear_format_logger, clear_logger, debug, enabled, error, info, level, set_format_logger,
    set_level, trace, warn, Level,
};

static SERIAL_LOGGER_INSTALLED: AtomicBool = AtomicBool::new(false);

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
pub fn init_serial_logger() {
    if SERIAL_LOGGER_INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    edgerun_log::set_format_logger(serial_logger);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
pub fn init_serial_logger() {
    let _ = SERIAL_LOGGER_INSTALLED.swap(true, Ordering::Relaxed);
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
#[inline]
pub fn log(level: u8, message: &str) {
    init_serial_logger();
    edgerun_log::log(level_from_legacy(level), "edgerun_rt", message);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[inline]
pub fn log(level: u8, message: &str) {
    let level = level_from_legacy(level);
    serial_write_str("[");
    serial_write_str(level.as_str());
    serial_write_str("] edgerun_rt: ");
    serial_write_str(message);
    serial_write_str("\n");
}

fn level_from_legacy(level: u8) -> Level {
    match level {
        0 => Level::Trace,
        1 => Level::Info,
        2 => Level::Warn,
        3 => Level::Error,
        4 => Level::Debug,
        _ => Level::Trace,
    }
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
fn serial_logger(level: Level, module: &str, args: fmt::Arguments<'_>) {
    let mut writer = SerialWriter;
    let _ = writer.write_str("[");
    let _ = writer.write_str(level.as_str());
    let _ = writer.write_str("] ");
    let _ = writer.write_str(module);
    let _ = writer.write_str(": ");
    let _ = writer.write_fmt(args);
    let _ = writer.write_str("\n");
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn serial_logger(level: Level, module: &str, args: fmt::Arguments<'_>) {
    serial_write_str("[");
    serial_write_str(level.as_str());
    serial_write_str("] ");
    serial_write_str(module);
    serial_write_str(": ");
    serial_write_str(args.as_str().unwrap_or("<fmt>"));
    serial_write_str("\n");
}

struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial_write_str(s);
        Ok(())
    }
}

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
fn serial_write_str(s: &str) {
    let serial_port = 0x3F8u16;
    for byte in s.bytes() {
        unsafe {
            core::arch::asm!("out dx, al", in("al") byte, in("dx") serial_port);
        }
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn serial_write_str(s: &str) {
    for chunk in s.as_bytes().chunks(64) {
        unsafe {
            edgerun_platform::arch::xtensa::esp32s3_usb_serial_jtag_write(chunk);
        }
    }
}

#[cfg(not(any(
    all(target_arch = "x86_64", target_os = "none"),
    all(target_arch = "xtensa", target_os = "none")
)))]
fn serial_write_str(_s: &str) {}
