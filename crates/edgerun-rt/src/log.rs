//! Bare-metal adapter for the central `edgerun-log` crate.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

pub use edgerun_log::{
    clear_format_logger, clear_logger, debug, enabled, error, info, level, set_format_logger,
    set_level, trace, warn, Level,
};

static SERIAL_LOGGER_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn init_serial_logger() {
    if SERIAL_LOGGER_INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    edgerun_log::set_format_logger(serial_logger);
}

#[inline]
pub fn log(level: u8, message: &str) {
    init_serial_logger();
    edgerun_log::log(level_from_legacy(level), "edgerun_rt", message);
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

fn serial_logger(level: Level, module: &str, args: fmt::Arguments<'_>) {
    let mut writer = SerialWriter;
    let _ = write!(writer, "[{}] {}: ", level.as_str(), module);
    let _ = writer.write_fmt(args);
    let _ = writer.write_str("\n");
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
    const UART0_FIFO: *mut u32 = 0x6000_0000 as *mut u32;
    const UART0_STATUS: *const u32 = 0x6000_001c as *const u32;
    const UART_FIFO_LEN: u32 = 128;
    const USB_EP1: *mut u32 = 0x6003_8000 as *mut u32;
    const USB_EP1_CONF: *mut u32 = 0x6003_8004 as *mut u32;
    const USB_EP1_CONF_WR_DONE: u32 = 1 << 0;
    const USB_EP1_CONF_DATA_FREE: u32 = 1 << 1;

    for chunk in s.as_bytes().chunks(64) {
        let mut usb_bytes = 0;
        for &byte in chunk {
            unsafe {
                if core::ptr::read_volatile(USB_EP1_CONF) & USB_EP1_CONF_DATA_FREE != 0 {
                    core::ptr::write_volatile(USB_EP1, byte as u32);
                    usb_bytes += 1;
                }
                while ((core::ptr::read_volatile(UART0_STATUS) >> 16) & 0x03ff) >= UART_FIFO_LEN {}
                core::ptr::write_volatile(UART0_FIFO, byte as u32);
            }
        }
        if usb_bytes != 0 {
            unsafe {
                let ep1_conf = core::ptr::read_volatile(USB_EP1_CONF);
                core::ptr::write_volatile(USB_EP1_CONF, ep1_conf | USB_EP1_CONF_WR_DONE);
            }
        }
    }
}

#[cfg(not(any(
    all(target_arch = "x86_64", target_os = "none"),
    all(target_arch = "xtensa", target_os = "none")
)))]
fn serial_write_str(_s: &str) {}
