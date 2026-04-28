//! Xtensa (LX series) implementation
//!
//! Xtensa uses the Configurable Interrupt Controller (CIC) and Event/Exception handling

#![allow(unsafe_op_in_unsafe_fn)]

/// Interrupt numbers
pub const INTLEVEL: u32 = 0x0;
pub const NMI: u32 = 2;

/// Get current CPU ID
#[inline]
pub fn this_cpu_id() -> u8 {
    let id: u32;
    unsafe {
        core::arch::asm!("rsr {id}, PRID", id = lateout(reg) id);
    }
    id as u8
}

/// Read CCOUNT (cycle counter)
#[inline]
pub fn ccount() -> u32 {
    let c: u32;
    unsafe {
        core::arch::asm!("rsr {c}, CCOUNT", c = lateout(reg) c);
    }
    c
}

/// Read CCOMPAREn (compare register for timer)
#[inline]
pub fn ccompare() -> u32 {
    let c: u32;
    unsafe {
        core::arch::asm!("rsr {c}, CCOMPARE0", c = lateout(reg) c);
    }
    c
}

/// Set CCOMPARE to trigger timer interrupt
#[inline]
pub fn set_ccompare(deadline: u32) {
    unsafe {
        core::arch::asm!("wsr {deadline}, CCOMPARE0", deadline = in(reg) deadline);
    }
}

/// Enable interrupt
#[inline]
pub fn enable_int(level: u8) {
    let mask = 1u32 << level;
    unsafe {
        core::arch::asm!("wsr {mask}, INTENABLE", mask = in(reg) mask);
    }
}

/// Disable interrupt
#[inline]
pub fn disable_int(level: u8) {
    let mask = !(1u32 << level);
    let enabled: u32;
    unsafe {
        core::arch::asm!("rsr {enabled}, INTENABLE", enabled = lateout(reg) enabled);
        core::arch::asm!("wsr {enabled}, INTENABLE", enabled = in(reg) enabled & mask);
    }
}

/// Check pending interrupts
#[inline]
pub fn pending() -> u32 {
    let p: u32;
    unsafe {
        core::arch::asm!("rsr {p}, INTERRUPT", p = lateout(reg) p);
    }
    p
}

/// Clear interrupt
#[inline]
pub fn clear(irq: u32) {
    unsafe {
        core::arch::asm!("wsr {irq}, INTCLEAR", irq = in(reg) irq);
    }
}

/// End Of Interrupt (clear N-level pending)
#[inline]
pub fn eoi(irq: u32) {
    let mask = 1u32 << irq;
    unsafe {
        core::arch::asm!("wsr {mask}, INTCLEAR", mask = in(reg) mask);
    }
}

/// Set dispatch handler (called on IRQ)
#[inline]
pub fn set_dispatch_handler(ptr: usize) {
    unsafe {
        core::arch::asm!("wsr {ptr}, EXCSAVE1", ptr = in(reg) ptr as u32);
    }
}

/// Read EXCSAVE (dispatch handler pointer)
#[inline]
pub fn dispatch_handler() -> usize {
    let ptr: u32;
    unsafe {
        core::arch::asm!("rsr {ptr}, EXCSAVE1", ptr = lateout(reg) ptr);
    }
    ptr as usize
}

/// Memory barrier
#[inline]
pub fn mem_wmb() {
    unsafe {
        core::arch::asm!("memw");
    }
}

/// Instruction barrier
#[inline]
pub fn instr_sync() {
    unsafe {
        core::arch::asm!("isync");
    }
}

/// Enable global interrupt
#[inline]
pub fn enable() {
    unsafe {
        core::arch::asm!("rsil a2, 0", out("a2") _);
    }
}

/// Set interrupt level
#[inline]
pub fn set_level(level: u32) {
    unsafe {
        match level.min(15) {
            0 => core::arch::asm!("rsil a2, 0", out("a2") _),
            1 => core::arch::asm!("rsil a2, 1", out("a2") _),
            2 => core::arch::asm!("rsil a2, 2", out("a2") _),
            3 => core::arch::asm!("rsil a2, 3", out("a2") _),
            4 => core::arch::asm!("rsil a2, 4", out("a2") _),
            5 => core::arch::asm!("rsil a2, 5", out("a2") _),
            6 => core::arch::asm!("rsil a2, 6", out("a2") _),
            7 => core::arch::asm!("rsil a2, 7", out("a2") _),
            8 => core::arch::asm!("rsil a2, 8", out("a2") _),
            9 => core::arch::asm!("rsil a2, 9", out("a2") _),
            10 => core::arch::asm!("rsil a2, 10", out("a2") _),
            11 => core::arch::asm!("rsil a2, 11", out("a2") _),
            12 => core::arch::asm!("rsil a2, 12", out("a2") _),
            13 => core::arch::asm!("rsil a2, 13", out("a2") _),
            14 => core::arch::asm!("rsil a2, 14", out("a2") _),
            _ => core::arch::asm!("rsil a2, 15", out("a2") _),
        }
    }
}

/// Initialize timer for async
pub fn timer_init() {
    // Set up CCOMPARE for 1ms tick
    let now = ccount();
    set_ccompare(now.wrapping_add(10_000));
}

/// Get Tls pointer from EXCSAVE area
#[inline]
pub fn tls() -> usize {
    dispatch_handler()
}

const USB_EP1: *mut u32 = 0x6003_8000 as *mut u32;
const USB_EP1_CONF: *mut u32 = 0x6003_8004 as *mut u32;
const USB_CONF0: *mut u32 = 0x6003_8018 as *mut u32;
const SYSTEM_PERIP_CLK_EN1: *mut u32 = 0x600c_001c as *mut u32;
const SYSTEM_PERIP_RST_EN1: *mut u32 = 0x600c_0024 as *mut u32;
const USB_DEVICE_CLK_RST_BIT: u32 = 1 << 10;
const USB_CONF0_DEFAULT: u32 = 0x4200;
const USB_EP1_CONF_WR_DONE: u32 = 1 << 0;
const USB_EP1_CONF_DATA_FREE: u32 = 1 << 1;
const WDT_WKEY: u32 = 0x50D8_3AA1;

const TIMG0_WDT_CONFIG0: *mut u32 = (0x6001_F000 + 0x48) as *mut u32;
const TIMG0_WDT_WPROTECT: *mut u32 = (0x6001_F000 + 0x64) as *mut u32;
const TIMG1_WDT_CONFIG0: *mut u32 = (0x6002_0000 + 0x48) as *mut u32;
const TIMG1_WDT_WPROTECT: *mut u32 = (0x6002_0000 + 0x64) as *mut u32;
const RTC_WDT_CONFIG0: *mut u32 = (0x6000_8000 + 0x98) as *mut u32;
const RTC_WDT_WPROTECT: *mut u32 = (0x6000_8000 + 0xb0) as *mut u32;

/// Enable the ESP32-S3 USB Serial/JTAG peripheral without resetting the USB link.
///
/// The ROM/second-stage bootloader already leaves this peripheral usable on
/// USB-connected boards. We only make the clock/reset state explicit and keep
/// the default internal-pad configuration.
#[inline]
pub unsafe fn esp32s3_usb_serial_jtag_init() {
    let clk = core::ptr::read_volatile(SYSTEM_PERIP_CLK_EN1);
    core::ptr::write_volatile(SYSTEM_PERIP_CLK_EN1, clk | USB_DEVICE_CLK_RST_BIT);

    let rst = core::ptr::read_volatile(SYSTEM_PERIP_RST_EN1);
    core::ptr::write_volatile(SYSTEM_PERIP_RST_EN1, rst & !USB_DEVICE_CLK_RST_BIT);

    core::ptr::write_volatile(USB_CONF0, USB_CONF0_DEFAULT);
}

/// Write one byte to ESP32-S3 USB Serial/JTAG.
#[inline]
pub unsafe fn esp32s3_usb_serial_jtag_write_byte(byte: u8) {
    wait_usb_serial_jtag_data_free();
    core::ptr::write_volatile(USB_EP1, byte as u32);
    let ep1_conf = core::ptr::read_volatile(USB_EP1_CONF);
    core::ptr::write_volatile(USB_EP1_CONF, ep1_conf | USB_EP1_CONF_WR_DONE);
}

/// Write bytes to ESP32-S3 USB Serial/JTAG in endpoint-sized chunks.
#[inline]
pub unsafe fn esp32s3_usb_serial_jtag_write(bytes: &[u8]) {
    for chunk in bytes.chunks(64) {
        wait_usb_serial_jtag_data_free();
        for &byte in chunk {
            core::ptr::write_volatile(USB_EP1, byte as u32);
        }
        let ep1_conf = core::ptr::read_volatile(USB_EP1_CONF);
        core::ptr::write_volatile(USB_EP1_CONF, ep1_conf | USB_EP1_CONF_WR_DONE);
    }
}

#[inline]
unsafe fn wait_usb_serial_jtag_data_free() {
    let mut spins = 100_000;
    while core::ptr::read_volatile(USB_EP1_CONF) & USB_EP1_CONF_DATA_FREE == 0 && spins != 0 {
        core::arch::asm!("nop");
        spins -= 1;
    }
}

/// Disable ESP32-S3 watchdogs left armed by the ROM/bootloader.
#[inline]
pub unsafe fn esp32s3_disable_watchdogs() {
    disable_wdt(TIMG0_WDT_WPROTECT, TIMG0_WDT_CONFIG0);
    disable_wdt(TIMG1_WDT_WPROTECT, TIMG1_WDT_CONFIG0);
    disable_wdt(RTC_WDT_WPROTECT, RTC_WDT_CONFIG0);
}

#[inline]
unsafe fn disable_wdt(wprotect: *mut u32, config0: *mut u32) {
    core::ptr::write_volatile(wprotect, WDT_WKEY);
    core::ptr::write_volatile(config0, 0);
    core::ptr::write_volatile(wprotect, 0);
}
