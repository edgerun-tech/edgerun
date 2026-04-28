//! Quarantined ESP32-S3 Wi-Fi blob adapter.
//!
//! This module is not enabled by default. It assumes an external Wi-Fi bring-up
//! path has initialized the Espressif driver enough for promiscuous RX and raw
//! 802.11 TX to work. The rest of Edgerun sees only `Esp32s3WifiRadio`.

use core::cell::UnsafeCell;
use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicI32, Ordering};

use edgerun_wifi::ieee80211::OpenApConfig;

use crate::esp32s3_wifi::Esp32s3WifiRadio;

const WIFI_PKT_MGMT: i32 = 0;
const WIFI_PKT_DATA: i32 = 2;
const WIFI_PROMIS_FILTER_MASK_MGMT: u32 = 1;
const WIFI_PROMIS_FILTER_MASK_DATA: u32 = 1 << 2;

const RAW_80211_MAX: usize = 2352;
const RAW_RX_QUEUE: usize = 8;
const APB_CTRL_WIFI_CLK_EN: *mut u32 = 0x6002_6014 as *mut u32;
const APB_CTRL_WIFI_RST_EN: *mut u32 = 0x6002_6018 as *mut u32;
const RTC_CNTL_DIG_PWC: *mut u32 = 0x6000_8090 as *mut u32;
const RTC_CNTL_DIG_ISO: *mut u32 = 0x6000_8094 as *mut u32;
const WIFI_MAC_RESET_CTRL: *mut u32 = 0x6003_3d14 as *mut u32;
const SYSTEM_WIFI_CLK_WIFI_BT_COMMON: u32 = 0x0078_078f;
const SYSTEM_WIFI_CLK_EN: u32 = 0x00fb_9fcf;
const SYSTEM_WIFI_BT_SDIO_CLK: u32 = (1 << 5) | (1 << 12) | (1 << 13);
const SYSTEM_MAC_RST: u32 = 1 << 2;
const RTC_CNTL_WIFI_FORCE_PD: u32 = 1 << 17;
const RTC_CNTL_WIFI_FORCE_ISO: u32 = 1 << 28;
const MODEM_RESET_FIELD_WHEN_POWERED: u32 =
    (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 9) | (1 << 11) | (1 << 13);
#[cfg(feature = "esp32s3-wifi-phy-probe")]
const PHY_RF_CAL_PARTIAL: c_int = 0;
#[cfg(feature = "esp32s3-wifi-phy-probe")]
const PHY_RF_CAL_NONE: c_int = 1;

static LAST_START_STATUS: AtomicI32 = AtomicI32::new(0);

#[cfg(feature = "esp32s3-wifi-phy-probe")]
#[repr(C)]
struct PhyInitData {
    params: [u8; 128],
}

#[cfg(feature = "esp32s3-wifi-phy-probe")]
#[repr(C)]
struct PhyCalibrationData {
    version: [u8; 4],
    mac: [u8; 6],
    opaque: [u8; 1894],
}

#[cfg(feature = "esp32s3-wifi-phy-probe")]
static PHY_INIT_DATA: PhyInitData = PhyInitData {
    params: [
        0x00, 0x00, 0x50, 0x50, 0x50, 0x4c, 0x4c, 0x48, 0x4c, 0x48, 0x48, 0x44, 0x4a, 0x46,
        0x46, 0x42, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
};

#[cfg(feature = "esp32s3-wifi-phy-probe")]
static mut PHY_CAL_DATA: PhyCalibrationData = PhyCalibrationData {
    version: [0; 4],
    mac: [0; 6],
    opaque: [0; 1894],
};

// ESP32-S3 wifi_pkt_rx_ctrl_t is 48 bytes in IDF 5.5.x. sig_len is the low 12
// bits of the u32 at offset 44 and includes the FCS when present.
const ESP32S3_PROMISC_RX_CTRL_LEN: usize = 48;
const ESP32S3_PROMISC_SIG_LEN_OFFSET: usize = 44;
const IEEE80211_FCS_LEN: usize = 4;

unsafe extern "C" {
    fn hal_init();
    fn mac_txrx_init();
    fn ic_mac_init() -> i32;
    fn ic_set_current_channel(channel: c_int);
    fn ic_set_promis_filter(mask: u32) -> i32;
    fn ic_register_promis_rx_cb(cb: Option<extern "C" fn(*mut c_void, c_int)>) -> i32;
    fn ic_enable_sniffer();
    fn ic_tx_pkt(buffer: *const c_void) -> i32;
    fn mac_rxbuf_init();
    fn mac_last_rxbuf_init();
    fn hal_mac_rate_autoack_init();
    fn hal_mac_disable_low_rate();
    fn hal_crypto_init();
    fn hal_attenna_init();
    fn hal_timer_update_by_rtc(enable: c_int, rtc_ticks: c_int);
    fn hal_coex_pti_init();
    fn hal_set_rx_active_pti(pti: c_int);
    fn hal_set_rx_ack_pti(pti: c_int);
    fn hal_set_wifi_default_pti(pti: c_int);
    fn phy_wakeup_init();
    fn phy_change_channel(channel: u16, arg1: c_int, arg2: c_int, second: c_int) -> i32;
    fn phy_get_romfuncs() -> *mut c_void;
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn rom_phy_param_addr(param: *mut c_void);
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_temp_to_power();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_pll_vol_cal();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_wifi_set_tx_gain();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_wifi_get_tx_gain();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_bt_get_tx_gain();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_get_i2c_hostid();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_txpwr_cal_track();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_wifi_tx_dig_gain();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_disable_wifi_agc();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_enable_wifi_agc();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_read_sar2_code();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_fe_i2c_reg_renew();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_write_pll_cap();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_bt_track_tx_power();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_wifi_track_tx_power();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_tsens_code_read();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_tsens_temp_read();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_set_pbus_reg();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_phy_dis_hw_set_freq();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_phy_en_hw_set_freq();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_set_noise_floor();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_bt_set_tx_gain();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_chip_i2c_writeReg();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_chip_i2c_readReg();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_phy_i2c_init1();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_i2c_master_reset();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_set_chan_cal_interp();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn spur_coef_cfg_new();
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    fn ram_set_txcap_reg();
    static mut g_phyFuns: *mut c_void;
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    static mut phy_param: u8;
    #[cfg(feature = "esp32s3-wifi-phy-probe")]
    fn phy_bbpll_en_usb(enable: bool);
    #[cfg(feature = "esp32s3-wifi-phy-probe")]
    fn register_chipv7_phy(
        init_data: *const PhyInitData,
        cal_data: *mut PhyCalibrationData,
        cal_mode: c_int,
    ) -> c_int;
}

type PhyFunNoArgI32 = unsafe extern "C" fn() -> i32;
type PhyFunGetU8 = unsafe extern "C" fn(c_int, *mut u8);

#[derive(Clone, Copy)]
struct RawRxFrame {
    used: bool,
    len: usize,
    bytes: [u8; RAW_80211_MAX],
}

impl RawRxFrame {
    const fn empty() -> Self {
        Self {
            used: false,
            len: 0,
            bytes: [0; RAW_80211_MAX],
        }
    }
}

struct RawRxQueue {
    frames: [RawRxFrame; RAW_RX_QUEUE],
    head: usize,
    tail: usize,
    dropped: u32,
}

impl RawRxQueue {
    const fn new() -> Self {
        Self {
            frames: [RawRxFrame::empty(); RAW_RX_QUEUE],
            head: 0,
            tail: 0,
            dropped: 0,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        if bytes.len() > RAW_80211_MAX || self.frames[self.head].used {
            self.dropped = self.dropped.wrapping_add(1);
            return;
        }
        let frame = &mut self.frames[self.head];
        frame.bytes[..bytes.len()].copy_from_slice(bytes);
        frame.len = bytes.len();
        frame.used = true;
        self.head = (self.head + 1) % RAW_RX_QUEUE;
    }

    fn pop(&mut self, out: &mut [u8]) -> Option<usize> {
        if self.head == self.tail && !self.frames[self.tail].used {
            return None;
        }
        let frame = &mut self.frames[self.tail];
        if out.len() < frame.len {
            return None;
        }
        let len = frame.len;
        out[..len].copy_from_slice(&frame.bytes[..len]);
        frame.used = false;
        frame.len = 0;
        self.tail = (self.tail + 1) % RAW_RX_QUEUE;
        Some(len)
    }
}

struct RawRxQueueCell(UnsafeCell<RawRxQueue>);

unsafe impl Sync for RawRxQueueCell {}

impl RawRxQueueCell {
    const fn new() -> Self {
        Self(UnsafeCell::new(RawRxQueue::new()))
    }

    unsafe fn push(&self, bytes: &[u8]) {
        unsafe { (&mut *self.0.get()).push(bytes) };
    }

    unsafe fn pop(&self, out: &mut [u8]) -> Option<usize> {
        unsafe { (&mut *self.0.get()).pop(out) }
    }
}

static RAW_RX: RawRxQueueCell = RawRxQueueCell::new();

pub struct EspressifPromiscRadio;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiDebugRegs {
    pub rtc_dig_pwc: u32,
    pub rtc_dig_iso: u32,
    pub wifi_clk_en: u32,
    pub wifi_rst_en: u32,
    pub mac_reset_ctrl: u32,
    pub phy_funs: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiPhyFunSlots {
    pub slot_008: u32,
    pub slot_00c: u32,
    pub slot_05c: u32,
    pub slot_06c: u32,
    pub slot_110: u32,
    pub slot_148: u32,
    pub slot_160: u32,
    pub slot_164: u32,
    pub slot_190: u32,
    pub slot_1a8: u32,
    pub slot_1d4: u32,
    pub slot_200: u32,
    pub slot_204: u32,
    pub slot_208: u32,
    pub slot_224: u32,
    pub slot_234: u32,
}

impl EspressifPromiscRadio {
    pub const fn new() -> Self {
        Self
    }

    pub fn last_start_status() -> i32 {
        LAST_START_STATUS.load(Ordering::Relaxed)
    }

    pub fn debug_regs() -> WifiDebugRegs {
        unsafe {
            WifiDebugRegs {
                rtc_dig_pwc: RTC_CNTL_DIG_PWC.read_volatile(),
                rtc_dig_iso: RTC_CNTL_DIG_ISO.read_volatile(),
                wifi_clk_en: APB_CTRL_WIFI_CLK_EN.read_volatile(),
                wifi_rst_en: APB_CTRL_WIFI_RST_EN.read_volatile(),
                mac_reset_ctrl: WIFI_MAC_RESET_CTRL.read_volatile(),
                phy_funs: g_phyFuns as usize as u32,
            }
        }
    }

    pub fn debug_phy_fun_slots() -> WifiPhyFunSlots {
        unsafe {
            WifiPhyFunSlots {
                slot_008: read_phy_fun_slot(0x008),
                slot_00c: read_phy_fun_slot(0x00c),
                slot_05c: read_phy_fun_slot(0x05c),
                slot_06c: read_phy_fun_slot(0x06c),
                slot_110: read_phy_fun_slot(0x110),
                slot_148: read_phy_fun_slot(0x148),
                slot_160: read_phy_fun_slot(0x160),
                slot_164: read_phy_fun_slot(0x164),
                slot_190: read_phy_fun_slot(0x190),
                slot_1a8: read_phy_fun_slot(0x1a8),
                slot_1d4: read_phy_fun_slot(0x1d4),
                slot_200: read_phy_fun_slot(0x200),
                slot_204: read_phy_fun_slot(0x204),
                slot_208: read_phy_fun_slot(0x208),
                slot_224: read_phy_fun_slot(0x224),
                slot_234: read_phy_fun_slot(0x234),
            }
        }
    }

    pub fn debug_step(step: u8) -> bool {
        unsafe {
            match step {
                0 => {
                    LAST_START_STATUS.store(1, Ordering::Relaxed);
                    enable_radio_clocks();
                    reset_wifi_mac();
                    LAST_START_STATUS.store(2, Ordering::Relaxed);
                    true
                }
                1 => {
                    LAST_START_STATUS.store(101, Ordering::Relaxed);
                    hal_init();
                    LAST_START_STATUS.store(102, Ordering::Relaxed);
                    true
                }
                2 => {
                    LAST_START_STATUS.store(201, Ordering::Relaxed);
                    mac_txrx_init();
                    LAST_START_STATUS.store(202, Ordering::Relaxed);
                    true
                }
                3 => {
                    LAST_START_STATUS.store(301, Ordering::Relaxed);
                    let status = ic_mac_init();
                    LAST_START_STATUS.store(300 + status, Ordering::Relaxed);
                    status == 0
                }
                4 => {
                    LAST_START_STATUS.store(401, Ordering::Relaxed);
                    ic_set_current_channel(6);
                    LAST_START_STATUS.store(402, Ordering::Relaxed);
                    true
                }
                5 => {
                    LAST_START_STATUS.store(501, Ordering::Relaxed);
                    let status =
                        ic_set_promis_filter(WIFI_PROMIS_FILTER_MASK_MGMT | WIFI_PROMIS_FILTER_MASK_DATA);
                    LAST_START_STATUS.store(500 + status, Ordering::Relaxed);
                    status != 0
                }
                6 => {
                    LAST_START_STATUS.store(601, Ordering::Relaxed);
                    let status = ic_register_promis_rx_cb(Some(promisc_rx_cb));
                    LAST_START_STATUS.store(600 + status, Ordering::Relaxed);
                    status != 0
                }
                7 => {
                    LAST_START_STATUS.store(701, Ordering::Relaxed);
                    ic_enable_sniffer();
                    LAST_START_STATUS.store(702, Ordering::Relaxed);
                    true
                }
                8 => run_noarg_step(801, 802, mac_rxbuf_init),
                9 => run_noarg_step(901, 902, mac_last_rxbuf_init),
                10 => run_noarg_step(1001, 1002, hal_mac_rate_autoack_init),
                11 => run_noarg_step(1101, 1102, hal_mac_disable_low_rate),
                12 => run_noarg_step(1201, 1202, hal_crypto_init),
                13 => run_noarg_step(1301, 1302, hal_attenna_init),
                14 => {
                    LAST_START_STATUS.store(1401, Ordering::Relaxed);
                    hal_timer_update_by_rtc(1, 0);
                    LAST_START_STATUS.store(1402, Ordering::Relaxed);
                    true
                }
                15 => run_noarg_step(1501, 1502, hal_coex_pti_init),
                16 => {
                    LAST_START_STATUS.store(1601, Ordering::Relaxed);
                    hal_set_rx_active_pti(0);
                    hal_set_rx_ack_pti(0);
                    hal_set_wifi_default_pti(0);
                    LAST_START_STATUS.store(1602, Ordering::Relaxed);
                    true
                }
                17 => run_noarg_step(1701, 1702, phy_wakeup_init),
                18 => {
                    LAST_START_STATUS.store(1801, Ordering::Relaxed);
                    let status = phy_change_channel(6, 0, 0, 0);
                    LAST_START_STATUS.store(1800 + status, Ordering::Relaxed);
                    status == 0
                }
                22 => {
                    LAST_START_STATUS.store(2201, Ordering::Relaxed);
                    patch_phy_romfunc_table();
                    LAST_START_STATUS.store(2202, Ordering::Relaxed);
                    !g_phyFuns.is_null()
                }
                23 => probe_phy_timer_ticks(),
                24 => probe_phy_pti(3, 2400),
                25 => probe_phy_pti(15, 2500),
                26 => {
                    LAST_START_STATUS.store(2601, Ordering::Relaxed);
                    let Some(ticks) = read_phy_timer_ticks() else {
                        return false;
                    };
                    hal_timer_update_by_rtc(1, ticks);
                    LAST_START_STATUS.store(2602, Ordering::Relaxed);
                    true
                }
                #[cfg(feature = "esp32s3-wifi-phy-probe")]
                19 => {
                    LAST_START_STATUS.store(1901, Ordering::Relaxed);
                    phy_bbpll_en_usb(true);
                    LAST_START_STATUS.store(1902, Ordering::Relaxed);
                    true
                }
                #[cfg(feature = "esp32s3-wifi-phy-probe")]
                20 => register_phy_step(2000, PHY_RF_CAL_NONE),
                #[cfg(feature = "esp32s3-wifi-phy-probe")]
                21 => register_phy_step(2100, PHY_RF_CAL_PARTIAL),
                _ => false,
            }
        }
    }
}

unsafe fn read_phy_fun_slot(offset: usize) -> u32 {
    let table = unsafe { g_phyFuns };
    if table.is_null() {
        return 0;
    }
    unsafe {
        table
            .cast::<u8>()
            .add(offset)
            .cast::<usize>()
            .read_volatile() as u32
    }
}

unsafe fn read_phy_fun_ptr(offset: usize) -> Option<usize> {
    let table = unsafe { g_phyFuns };
    if table.is_null() {
        return None;
    }
    let ptr = unsafe {
        table
            .cast::<u8>()
            .add(offset)
            .cast::<usize>()
            .read_volatile()
    };
    if ptr == 0 {
        None
    } else {
        Some(ptr)
    }
}

unsafe fn read_phy_timer_ticks() -> Option<c_int> {
    let ptr = unsafe { read_phy_fun_ptr(0x148)? };
    let f: PhyFunNoArgI32 = unsafe { core::mem::transmute(ptr) };
    Some(unsafe { f() })
}

unsafe fn probe_phy_timer_ticks() -> bool {
    LAST_START_STATUS.store(2301, Ordering::Relaxed);
    let Some(ticks) = (unsafe { read_phy_timer_ticks() }) else {
        LAST_START_STATUS.store(2300, Ordering::Relaxed);
        return false;
    };
    LAST_START_STATUS.store(2300 + (ticks & 0xff), Ordering::Relaxed);
    true
}

unsafe fn probe_phy_pti(which: c_int, base_status: i32) -> bool {
    LAST_START_STATUS.store(base_status + 1, Ordering::Relaxed);
    let Some(ptr) = (unsafe { read_phy_fun_ptr(0x1a8) }) else {
        LAST_START_STATUS.store(base_status, Ordering::Relaxed);
        return false;
    };
    let f: PhyFunGetU8 = unsafe { core::mem::transmute(ptr) };
    let mut value = 0u8;
    unsafe { f(which, core::ptr::addr_of_mut!(value)) };
    LAST_START_STATUS.store(base_status + value as i32, Ordering::Relaxed);
    true
}

unsafe fn run_noarg_step(before: i32, after: i32, f: unsafe extern "C" fn()) -> bool {
    LAST_START_STATUS.store(before, Ordering::Relaxed);
    unsafe { f() };
    LAST_START_STATUS.store(after, Ordering::Relaxed);
    true
}

unsafe fn patch_phy_romfunc_table() {
    let table = unsafe { phy_get_romfuncs() };
    unsafe {
        g_phyFuns = table;
    }
    #[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
    unsafe {
        patch_phy_ramfunc_table(table);
    }
}

#[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
unsafe fn patch_phy_ramfunc_table(table: *mut c_void) {
    if table.is_null() {
        return;
    }
    unsafe {
        patch_phy_func(table, 0x144, ram_temp_to_power);
        patch_phy_func(table, 0x208, ram_pll_vol_cal);
        patch_phy_func(table, 0x264, ram_wifi_set_tx_gain);
        patch_phy_func(table, 0x228, ram_wifi_get_tx_gain);
        patch_phy_func(table, 0x218, ram_bt_get_tx_gain);
        patch_phy_func(table, 0x15c, ram_get_i2c_hostid);
        patch_phy_func(table, 0x268, ram_txpwr_cal_track);
        patch_phy_func(table, 0x224, ram_wifi_tx_dig_gain);
        patch_phy_func(table, 0x008, ram_disable_wifi_agc);
        patch_phy_func(table, 0x00c, ram_enable_wifi_agc);
        patch_phy_func(table, 0x128, ram_read_sar2_code);
        patch_phy_func(table, 0x22c, ram_fe_i2c_reg_renew);
        patch_phy_func(table, 0x20c, ram_write_pll_cap);
        patch_phy_func(table, 0x288, ram_bt_track_tx_power);
        patch_phy_func(table, 0x28c, ram_wifi_track_tx_power);
        patch_phy_func(table, 0x1e4, ram_tsens_code_read);
        patch_phy_func(table, 0x258, ram_tsens_temp_read);
        patch_phy_func(table, 0x0c8, ram_set_pbus_reg);
        patch_phy_func(table, 0x204, ram_phy_dis_hw_set_freq);
        patch_phy_func(table, 0x200, ram_phy_en_hw_set_freq);
        patch_phy_func(table, 0x080, ram_set_noise_floor);
        patch_phy_func(table, 0x270, ram_bt_set_tx_gain);
        patch_phy_func(table, 0x18c, ram_chip_i2c_writeReg);
        patch_phy_func(table, 0x16c, ram_chip_i2c_readReg);
        patch_phy_func(table, 0x254, ram_phy_i2c_init1);
        patch_phy_func(table, 0x234, ram_i2c_master_reset);
        patch_phy_func(table, 0x0fc, ram_set_chan_cal_interp);
        patch_phy_func(table, 0x054, spur_coef_cfg_new);
        patch_phy_func(table, 0x100, ram_set_txcap_reg);
        rom_phy_param_addr(core::ptr::addr_of_mut!(phy_param).cast::<c_void>());
    }
}

#[cfg(feature = "esp32s3-wifi-ram-phy-patch")]
unsafe fn patch_phy_func(table: *mut c_void, offset: usize, func: unsafe extern "C" fn()) {
    unsafe {
        table
            .cast::<u8>()
            .add(offset)
            .cast::<usize>()
            .write_volatile(func as usize);
    }
}

#[cfg(feature = "esp32s3-wifi-phy-probe")]
unsafe fn register_phy_step(base: i32, mode: c_int) -> bool {
    LAST_START_STATUS.store(base + 1, Ordering::Relaxed);
    let status = unsafe {
        register_chipv7_phy(
            core::ptr::addr_of!(PHY_INIT_DATA),
            core::ptr::addr_of_mut!(PHY_CAL_DATA),
            mode,
        )
    };
    LAST_START_STATUS.store(base + status, Ordering::Relaxed);
    status == 0
}

impl Esp32s3WifiRadio for EspressifPromiscRadio {
    fn start_open_ap(&mut self, config: &OpenApConfig) -> bool {
        unsafe {
            enable_radio_clocks();
            reset_wifi_mac();
            hal_init();
            mac_txrx_init();
            let status = ic_mac_init();
            if status != 0 {
                LAST_START_STATUS.store(1000 + status, Ordering::Relaxed);
                return false;
            }
            ic_set_current_channel(config.channel as c_int);
            let status = ic_set_promis_filter(WIFI_PROMIS_FILTER_MASK_MGMT | WIFI_PROMIS_FILTER_MASK_DATA);
            if status == 0 {
                LAST_START_STATUS.store(2000 + status, Ordering::Relaxed);
                return false;
            }
            let status = ic_register_promis_rx_cb(Some(promisc_rx_cb));
            if status == 0 {
                LAST_START_STATUS.store(3000 + status, Ordering::Relaxed);
                return false;
            }
            ic_enable_sniffer();
            LAST_START_STATUS.store(1, Ordering::Relaxed);
            true
        }
    }

    fn send_raw_80211(&mut self, frame: &[u8]) -> bool {
        if frame.len() < 24 || frame.len() > 1500 {
            return false;
        }
        unsafe { ic_tx_pkt(frame.as_ptr().cast::<c_void>()) == 0 }
    }

    fn recv_raw_80211(&mut self, out: &mut [u8]) -> Option<usize> {
        unsafe { RAW_RX.pop(out) }
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
        APB_CTRL_WIFI_CLK_EN.write_volatile((current & !SYSTEM_WIFI_BT_SDIO_CLK) | SYSTEM_WIFI_CLK_EN);
    }
    let current = unsafe { WIFI_MAC_RESET_CTRL.read_volatile() };
    unsafe {
        WIFI_MAC_RESET_CTRL.write_volatile(current & !1);
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

extern "C" fn promisc_rx_cb(buf: *mut c_void, packet_type: c_int) {
    if packet_type != WIFI_PKT_MGMT && packet_type != WIFI_PKT_DATA {
        return;
    }
    if buf.is_null() {
        return;
    }
    unsafe {
        let bytes = buf.cast::<u8>();
        let sig_len_raw =
            core::ptr::read_unaligned(bytes.add(ESP32S3_PROMISC_SIG_LEN_OFFSET).cast::<u32>());
        let mut len = (sig_len_raw & 0x0fff) as usize;
        if len > IEEE80211_FCS_LEN {
            len -= IEEE80211_FCS_LEN;
        }
        if len == 0 || len > RAW_80211_MAX {
            return;
        }
        let payload = core::slice::from_raw_parts(bytes.add(ESP32S3_PROMISC_RX_CTRL_LEN), len);
        RAW_RX.push(payload);
    }
}
