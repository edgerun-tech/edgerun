//! Blob-free ESP32-S3 Wi-Fi MMIO bring-up probes.
//!
//! This module deliberately does not link Espressif Wi-Fi archives or ROM
//! linker scripts. It owns only direct register reads/writes that we have
//! observed to be boot-safe while isolating the MAC/PHY work from vendor blobs.

use core::sync::atomic::{AtomicI32, Ordering};

const APB_CTRL_WIFI_CLK_EN: *mut u32 = 0x6002_6014 as *mut u32;
const APB_CTRL_WIFI_RST_EN: *mut u32 = 0x6002_6018 as *mut u32;
const RTC_CNTL_DIG_PWC: *mut u32 = 0x6000_8090 as *mut u32;
const RTC_CNTL_DIG_ISO: *mut u32 = 0x6000_8094 as *mut u32;
const WIFI_MAC_RESET_CTRL: *mut u32 = 0x6003_3d14 as *mut u32;
const WIFI_MAC_DMA_CTRL: *mut u32 = 0x6003_3c6c as *mut u32;
const WIFI_MAC_RX_POLICY_BASE: *mut u32 = 0x6003_30d8 as *mut u32;
const WIFI_MAC_CTRL_33C34: *mut u32 = 0x6003_3c34 as *mut u32;
const WIFI_MAC_CTRL_33C40: *mut u32 = 0x6003_3c40 as *mut u32;
const WIFI_MAC_CTRL_33C74: *mut u32 = 0x6003_3c74 as *mut u32;
const WIFI_MAC_RX_CTRL0: *mut u32 = 0x6003_3100 as *mut u32;
const WIFI_MAC_RX_CTRL1: *mut u32 = 0x6003_3104 as *mut u32;
const WIFI_MAC_RX_CTRL2: *mut u32 = 0x6003_3108 as *mut u32;
const WIFI_MAC_RX_CTRL3: *mut u32 = 0x6003_310c as *mut u32;
const WIFI_MAC_CTRL_33114: *mut u32 = 0x6003_3114 as *mut u32;
const WIFI_MAC_CTRL_33118: *mut u32 = 0x6003_3118 as *mut u32;
const WIFI_COEX_CTRL: *mut u32 = 0x6003_5084 as *mut u32;
const WIFI_COEX_PTI: *mut u32 = 0x6003_329c as *mut u32;
const WIFI_COEX_DEFAULT_PTI: *mut u32 = 0x6003_5094 as *mut u32;

const SYSTEM_WIFI_CLK_WIFI_BT_COMMON: u32 = 0x0078_078f;
const SYSTEM_WIFI_CLK_EN: u32 = 0x00fb_9fcf;
const SYSTEM_WIFI_BT_SDIO_CLK: u32 = (1 << 5) | (1 << 12) | (1 << 13);
const SYSTEM_MAC_RST: u32 = 1 << 2;
const RTC_CNTL_WIFI_FORCE_PD: u32 = 1 << 17;
const RTC_CNTL_WIFI_FORCE_ISO: u32 = 1 << 28;
const MODEM_RESET_FIELD_WHEN_POWERED: u32 =
    (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 9) | (1 << 11) | (1 << 13);

static LAST_STATUS: AtomicI32 = AtomicI32::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioDebugRegs {
    pub rtc_dig_pwc: u32,
    pub rtc_dig_iso: u32,
    pub wifi_clk_en: u32,
    pub wifi_rst_en: u32,
    pub mac_reset_ctrl: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioMacRegs {
    pub mac_reset_ctrl: u32,
    pub dma_ctrl: u32,
    pub rx_policy0: u32,
    pub rx_policy1: u32,
    pub rx_policy2: u32,
    pub rx_policy3: u32,
    pub ctrl_33c34: u32,
    pub ctrl_33c40: u32,
    pub ctrl_33c74: u32,
    pub rx_ctrl0: u32,
    pub rx_ctrl1: u32,
    pub rx_ctrl2: u32,
    pub rx_ctrl3: u32,
    pub ctrl_33114: u32,
    pub ctrl_33118: u32,
    pub coex_ctrl: u32,
    pub coex_pti: u32,
    pub coex_default_pti: u32,
}

pub struct Esp32s3WifiMmio;

impl Esp32s3WifiMmio {
    pub fn last_status() -> i32 {
        LAST_STATUS.load(Ordering::Relaxed)
    }

    pub fn debug_regs() -> WifiMmioDebugRegs {
        unsafe {
            WifiMmioDebugRegs {
                rtc_dig_pwc: RTC_CNTL_DIG_PWC.read_volatile(),
                rtc_dig_iso: RTC_CNTL_DIG_ISO.read_volatile(),
                wifi_clk_en: APB_CTRL_WIFI_CLK_EN.read_volatile(),
                wifi_rst_en: APB_CTRL_WIFI_RST_EN.read_volatile(),
                mac_reset_ctrl: WIFI_MAC_RESET_CTRL.read_volatile(),
            }
        }
    }

    pub fn debug_mac_regs() -> WifiMmioMacRegs {
        unsafe {
            WifiMmioMacRegs {
                mac_reset_ctrl: WIFI_MAC_RESET_CTRL.read_volatile(),
                dma_ctrl: WIFI_MAC_DMA_CTRL.read_volatile(),
                rx_policy0: WIFI_MAC_RX_POLICY_BASE.add(0).read_volatile(),
                rx_policy1: WIFI_MAC_RX_POLICY_BASE.add(1).read_volatile(),
                rx_policy2: WIFI_MAC_RX_POLICY_BASE.add(2).read_volatile(),
                rx_policy3: WIFI_MAC_RX_POLICY_BASE.add(3).read_volatile(),
                ctrl_33c34: WIFI_MAC_CTRL_33C34.read_volatile(),
                ctrl_33c40: WIFI_MAC_CTRL_33C40.read_volatile(),
                ctrl_33c74: WIFI_MAC_CTRL_33C74.read_volatile(),
                rx_ctrl0: WIFI_MAC_RX_CTRL0.read_volatile(),
                rx_ctrl1: WIFI_MAC_RX_CTRL1.read_volatile(),
                rx_ctrl2: WIFI_MAC_RX_CTRL2.read_volatile(),
                rx_ctrl3: WIFI_MAC_RX_CTRL3.read_volatile(),
                ctrl_33114: WIFI_MAC_CTRL_33114.read_volatile(),
                ctrl_33118: WIFI_MAC_CTRL_33118.read_volatile(),
                coex_ctrl: WIFI_COEX_CTRL.read_volatile(),
                coex_pti: WIFI_COEX_PTI.read_volatile(),
                coex_default_pti: WIFI_COEX_DEFAULT_PTI.read_volatile(),
            }
        }
    }

    pub fn debug_step(step: u8) -> bool {
        unsafe {
            match step {
                0 => {
                    LAST_STATUS.store(1, Ordering::Relaxed);
                    enable_radio_clocks();
                    reset_wifi_mac();
                    LAST_STATUS.store(2, Ordering::Relaxed);
                    true
                }
                1 => {
                    LAST_STATUS.store(101, Ordering::Relaxed);
                    set_mac_ready_bit();
                    LAST_STATUS.store(102, Ordering::Relaxed);
                    true
                }
                2 => {
                    LAST_STATUS.store(201, Ordering::Relaxed);
                    clear_mac_ready_bit();
                    LAST_STATUS.store(202, Ordering::Relaxed);
                    true
                }
                3 => {
                    LAST_STATUS.store(301, Ordering::Relaxed);
                    request_mac_enable();
                    LAST_STATUS.store(302, Ordering::Relaxed);
                    true
                }
                4 => {
                    LAST_STATUS.store(401, Ordering::Relaxed);
                    init_mac_txrx_slice();
                    LAST_STATUS.store(402, Ordering::Relaxed);
                    true
                }
                _ => false,
            }
        }
    }
}

unsafe fn enable_radio_clocks() {
    let current = unsafe { RTC_CNTL_DIG_PWC.read_volatile() };
    unsafe {
        RTC_CNTL_DIG_PWC.write_volatile(current & !RTC_CNTL_WIFI_FORCE_PD);
    }
    let current = unsafe { APB_CTRL_WIFI_RST_EN.read_volatile() };
    unsafe {
        APB_CTRL_WIFI_RST_EN.write_volatile(current | MODEM_RESET_FIELD_WHEN_POWERED);
    }
    let current = unsafe { APB_CTRL_WIFI_RST_EN.read_volatile() };
    unsafe {
        APB_CTRL_WIFI_RST_EN.write_volatile(current & !MODEM_RESET_FIELD_WHEN_POWERED);
    }
    let current = unsafe { RTC_CNTL_DIG_ISO.read_volatile() };
    unsafe {
        RTC_CNTL_DIG_ISO.write_volatile(current & !RTC_CNTL_WIFI_FORCE_ISO);
    }

    let current = unsafe { APB_CTRL_WIFI_CLK_EN.read_volatile() };
    unsafe {
        APB_CTRL_WIFI_CLK_EN.write_volatile(current | SYSTEM_WIFI_CLK_WIFI_BT_COMMON);
    }
    let current = unsafe { APB_CTRL_WIFI_CLK_EN.read_volatile() };
    unsafe {
        APB_CTRL_WIFI_CLK_EN
            .write_volatile((current & !SYSTEM_WIFI_BT_SDIO_CLK) | SYSTEM_WIFI_CLK_EN);
    }
}

unsafe fn reset_wifi_mac() {
    let current = unsafe { APB_CTRL_WIFI_RST_EN.read_volatile() };
    unsafe {
        APB_CTRL_WIFI_RST_EN.write_volatile(current | SYSTEM_MAC_RST);
    }
    let current = unsafe { APB_CTRL_WIFI_RST_EN.read_volatile() };
    unsafe {
        APB_CTRL_WIFI_RST_EN.write_volatile(current & !SYSTEM_MAC_RST);
    }
}

unsafe fn request_mac_enable() {
    let current = unsafe { WIFI_MAC_RESET_CTRL.read_volatile() };
    unsafe {
        WIFI_MAC_RESET_CTRL.write_volatile(current | 2);
    }
}

unsafe fn set_mac_ready_bit() {
    let current = unsafe { WIFI_MAC_RESET_CTRL.read_volatile() };
    unsafe {
        WIFI_MAC_RESET_CTRL.write_volatile(current | 1);
    }
}

unsafe fn clear_mac_ready_bit() {
    let current = unsafe { WIFI_MAC_RESET_CTRL.read_volatile() };
    unsafe {
        WIFI_MAC_RESET_CTRL.write_volatile(current & !1);
    }
}

unsafe fn init_mac_txrx_slice() {
    update(WIFI_MAC_DMA_CTRL, |v| v | 0x8080_a000);
    update(WIFI_MAC_DMA_CTRL, |v| v | 0x0000_0100);

    let mut index = 0;
    while index < 4 {
        let reg = unsafe { WIFI_MAC_RX_POLICY_BASE.add(index) };
        update(reg, |v| v | 0x40);
        update(reg, |v| v & !0x20);
        index += 1;
    }

    update(WIFI_MAC_CTRL_33C74, |v| v | 0x8);
    update(WIFI_MAC_RX_CTRL0, |v| v & 0x0000_ffff);
    update(WIFI_MAC_RX_CTRL1, |v| v & 0x0000_ffff);
    update(WIFI_MAC_RX_CTRL2, |v| v & 0x0000_ffff);
    update(WIFI_MAC_RX_CTRL3, |v| v & 0x0000_ffff);
    update(WIFI_MAC_RX_CTRL0, |v| v | (1 << 24) | (1 << 26));
    update(WIFI_MAC_RX_CTRL1, |v| v | (1 << 24) | (1 << 26));
    update(WIFI_MAC_DMA_CTRL, |v| v | 0x0000_0200);
    update(WIFI_MAC_CTRL_33114, |v| v & !0xf0);
    update(WIFI_MAC_CTRL_33118, |v| v | 0x8000_0000);
}

unsafe fn update(reg: *mut u32, f: impl FnOnce(u32) -> u32) {
    let value = unsafe { reg.read_volatile() };
    unsafe { reg.write_volatile(f(value)) };
}
