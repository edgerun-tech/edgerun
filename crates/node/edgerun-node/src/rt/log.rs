//! Bare-metal adapter for the central `edgerun-log` crate.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub use edgerun_log::{
    LogLevel as Level, clear_format_logger, clear_logger, debug, enabled, error, info, level,
    set_format_logger, set_level, trace, warn,
};

static SERIAL_LOGGER_INSTALLED: AtomicBool = AtomicBool::new(false);
static MIRROR_LOGGER: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

pub type MirrorLogger = fn(&str);

pub fn set_mirror_logger(logger: MirrorLogger) {
    MIRROR_LOGGER.store(logger as *mut (), Ordering::Relaxed);
}

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
    mirror_log(message);
    edgerun_log::log(level_from_legacy(level), "edgerun_node_rt", message);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
#[inline]
pub fn log(level: u8, message: &str) {
    let level = level_from_legacy(level);
    mirror_log(message);
    mux_log("[");
    mux_log(level.as_str());
    mux_log("] edgerun_node_rt: ");
    mux_log(message);
    mux_log("\n");
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

fn mirror_log(message: &str) {
    let logger = MIRROR_LOGGER.load(Ordering::Relaxed);
    if logger.is_null() {
        return;
    }
    let logger: MirrorLogger = unsafe { core::mem::transmute(logger) };
    logger(message);
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
    mux_log("[");
    mux_log(level.as_str());
    mux_log("] ");
    mux_log(module);
    mux_log(": ");
    mux_log(args.as_str().unwrap_or("<fmt>"));
    mux_log("\n");
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

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
fn mux_log(s: &str) {
    crate::rt::serial_mux::write(crate::rt::serial_mux::CHANNEL_LOG, s.as_bytes());
}

#[cfg(not(any(
    all(target_arch = "x86_64", target_os = "none"),
    all(target_arch = "xtensa", target_os = "none")
)))]
fn serial_write_str(_s: &str) {}
