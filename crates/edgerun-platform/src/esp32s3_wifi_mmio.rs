//! Blob-free ESP32-S3 Wi-Fi MMIO bring-up probes.
//!
//! This module deliberately does not link Espressif Wi-Fi archives or ROM
//! linker scripts. It owns only direct register reads/writes that we have
//! observed to be boot-safe while isolating the MAC/PHY work from vendor blobs.

use core::sync::atomic::{AtomicI32, Ordering};

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe extern "C" {
    fn phy_get_romfuncs() -> *mut core::ffi::c_void;
}

const APB_CTRL_WIFI_CLK_EN: *mut u32 = 0x6002_6014 as *mut u32;
const APB_CTRL_WIFI_RST_EN: *mut u32 = 0x6002_6018 as *mut u32;
const RTC_CNTL_DIG_PWC: *mut u32 = 0x6000_8090 as *mut u32;
const RTC_CNTL_DIG_ISO: *mut u32 = 0x6000_8094 as *mut u32;
const WIFI_MAC_RESET_CTRL: *mut u32 = 0x6003_3d14 as *mut u32;
const WIFI_MAC_DMA_CTRL: *mut u32 = 0x6003_3c6c as *mut u32;
const WIFI_MAC_RX_POLICY_BASE: *mut u32 = 0x6003_30d8 as *mut u32;
const WIFI_MAC_CTRL_33C00: *mut u32 = 0x6003_3c00 as *mut u32;
const WIFI_MAC_CTRL_33C34: *mut u32 = 0x6003_3c34 as *mut u32;
const WIFI_MAC_CTRL_33C40: *mut u32 = 0x6003_3c40 as *mut u32;
const WIFI_MAC_CTRL_33C74: *mut u32 = 0x6003_3c74 as *mut u32;
const WIFI_MAC_CTRL_33C78: *mut u32 = 0x6003_3c78 as *mut u32;
const WIFI_MAC_TX_CTRL_33C10: *mut u32 = 0x6003_3c10 as *mut u32;
const WIFI_MAC_TX_CTRL_33C14: *mut u32 = 0x6003_3c14 as *mut u32;
const WIFI_MAC_TX_CTRL_33C18: *mut u32 = 0x6003_3c18 as *mut u32;
const WIFI_MAC_TX_CTRL_33C54: *mut u32 = 0x6003_3c54 as *mut u32;
const WIFI_MAC_TX_CTRL_33C88: *mut u32 = 0x6003_3c88 as *mut u32;
const WIFI_MAC_TX_CTRL_33C94: *mut u32 = 0x6003_3c94 as *mut u32;
const WIFI_MAC_RATE_CTRL_33404: *mut u32 = 0x6003_3404 as *mut u32;
const WIFI_MAC_RATE_CTRL_33408: *mut u32 = 0x6003_3408 as *mut u32;
const WIFI_MAC_RATE_CTRL_3340C: *mut u32 = 0x6003_340c as *mut u32;
const WIFI_MAC_RATE_CTRL_33410: *mut u32 = 0x6003_3410 as *mut u32;
const WIFI_MAC_RATE_CTRL_33414: *mut u32 = 0x6003_3414 as *mut u32;
const WIFI_MAC_RATE_CTRL_33418: *mut u32 = 0x6003_3418 as *mut u32;
const WIFI_MAC_CRYPTO_CTRL_33800: *mut u32 = 0x6003_3800 as *mut u32;
const WIFI_MAC_CRYPTO_CTRL_33804: *mut u32 = 0x6003_3804 as *mut u32;
const WIFI_MAC_CRYPTO_CTRL_33808: *mut u32 = 0x6003_3808 as *mut u32;
const WIFI_MAC_CRYPTO_CTRL_3380C: *mut u32 = 0x6003_380c as *mut u32;
const WIFI_MAC_CRYPTO_CTRL_33810: *mut u32 = 0x6003_3810 as *mut u32;
const WIFI_MAC_CRYPTO_CTRL_33840: *mut u32 = 0x6003_3840 as *mut u32;
const WIFI_MAC_ANT_CTRL_START: *mut u32 = 0x6003_4314 as *mut u32;
const WIFI_MAC_ANT_CTRL_STRIDE_WORDS: usize = 76 / 4;
const WIFI_MAC_ANT_CTRL_COUNT: usize = 8;
const WIFI_MAC_ANT_CTRL_332A8: *mut u32 = 0x6003_32a8 as *mut u32;
const WIFI_PHY_LOW_RATE_CTRL0: *mut u32 = 0x6001_c860 as *mut u32;
const WIFI_PHY_LOW_RATE_CTRL1: *mut u32 = 0x6001_c87c as *mut u32;
const WIFI_PHY_TX_SEED: *mut u32 = 0x6001_c400 as *mut u32;
const WIFI_PHY_RX_11B_CTRL0: *mut u32 = 0x6001_c044 as *mut u32;
const WIFI_PHY_RX_11B_CTRL1: *mut u32 = 0x6001_c124 as *mut u32;
const WIFI_PHY_RX_11B_CTRL2: *mut u32 = 0x6001_c804 as *mut u32;
const WIFI_PHY_RX_11B_CTRL3: *mut u32 = 0x6001_c104 as *mut u32;
const WIFI_PHY_RX_2440M_CTRL: *mut u32 = 0x6001_c02c as *mut u32;
const WIFI_PHY_BB_CTRL_1CC48: *mut u32 = 0x6001_cc48 as *mut u32;
const WIFI_MODEM_WIFI_ENABLE: *mut u32 = 0x6002_600c as *mut u32;
const WIFI_MODEM_CTRL_26010: *mut u32 = 0x6002_6010 as *mut u32;
const WIFI_PBUS_ADDR_CTRL: *mut u32 = 0x6000_60c8 as *mut u32;
const WIFI_PBUS_DATA: *mut u32 = 0x6000_60cc as *mut u32;
const WIFI_PBUS_BANK0: *mut u32 = 0x6000_60e0 as *mut u32;
const WIFI_PBUS_BANK1: *mut u32 = 0x6000_60e4 as *mut u32;
const WIFI_PBUS_BANK2: *mut u32 = 0x6000_60e8 as *mut u32;
const WIFI_PBUS_BANK3: *mut u32 = 0x6000_60ec as *mut u32;
const WIFI_PBUS_BANK4: *mut u32 = 0x6000_60f0 as *mut u32;
const WIFI_PBUS_BANK5: *mut u32 = 0x6000_60f4 as *mut u32;
const WIFI_TXRATE_POWER0: *mut u32 = 0x6000_6180 as *mut u32;
const WIFI_TXRATE_POWER15: *mut u32 = 0x6000_61bc as *mut u32;
const WIFI_I2C_XPD_CTRL0: *mut u32 = 0x6000_8034 as *mut u32;
const WIFI_I2C_XPD_CTRL1: *mut u32 = 0x6000_8000 as *mut u32;
const WIFI_RF_CTRL: *mut u32 = 0x6000_e130 as *mut u32;
const WIFI_TXRX_CTRL: *mut u32 = 0x6000_6110 as *mut u32;
const WIFI_FREQ_PLL_CAP: *mut u32 = 0x6000_e0c0 as *mut u32;
const WIFI_FREQ_HW_CTRL: *mut u32 = 0x6000_e0c4 as *mut u32;
const WIFI_FREQ_MEM_CTRL: *mut u32 = 0x6000_e148 as *mut u32;
const WIFI_FREQ_SW_CTRL: *mut u32 = 0x6000_e150 as *mut u32;
const WIFI_FREQ_BUSY: *mut u32 = 0x6000_e168 as *mut u32;
const WIFI_FREQ_STATUS: *mut u32 = 0x6000_e170 as *mut u32;
const WIFI_FREQ_MODE_CTRL: *mut u32 = 0x6003_509c as *mut u32;
const WIFI_SYSTIMER_VALUE: *mut u32 = 0x6003_5000 as *mut u32;
const WIFI_SYSTIMER_AUX: *mut u32 = 0x6003_5004 as *mut u32;
const WIFI_FREQ_I2C_ADDR_CTRL: *mut u32 = 0x6000_e164 as *mut u32;
const WIFI_FREQ_I2C_NIB0: *mut u32 = 0x6000_e100 as *mut u32;
const WIFI_FREQ_I2C_NIB1: *mut u32 = 0x6000_e104 as *mut u32;
const WIFI_FREQ_I2C_NIB2: *mut u32 = 0x6000_e108 as *mut u32;
const WIFI_FREQ_I2C_PAIR0: *mut u32 = 0x6000_e0d8 as *mut u32;
const WIFI_FREQ_I2C_PAIR1: *mut u32 = 0x6000_e0dc as *mut u32;
const WIFI_FREQ_I2C_PAIR2: *mut u32 = 0x6000_e0e0 as *mut u32;
const WIFI_FREQ_I2C_PAIR3: *mut u32 = 0x6000_e0e4 as *mut u32;
const WIFI_FREQ_I2C_PAIR4: *mut u32 = 0x6000_e0e8 as *mut u32;
const WIFI_FREQ_I2C_PAIR5: *mut u32 = 0x6000_e0ec as *mut u32;
const WIFI_FREQ_I2C_PAIR6: *mut u32 = 0x6000_e0f0 as *mut u32;
const WIFI_FREQ_I2C_PAIR7: *mut u32 = 0x6000_e0f4 as *mut u32;
const WIFI_FREQ_I2C_PAIR8: *mut u32 = 0x6000_e10c as *mut u32;
const WIFI_FREQ_I2C_PAIR9: *mut u32 = 0x6000_e110 as *mut u32;
const WIFI_FREQ_I2C_HI_A: *mut u32 = 0x6000_e128 as *mut u32;
const WIFI_FREQ_I2C_HI_B: *mut u32 = 0x6000_e12c as *mut u32;
const WIFI_FREQ_I2C_LOW_A0: *mut u32 = 0x6000_e0d0 as *mut u32;
const WIFI_FREQ_I2C_LOW_A1: *mut u32 = 0x6000_e0d4 as *mut u32;
const WIFI_FREQ_I2C_LOW_A2: *mut u32 = 0x6000_e124 as *mut u32;
const WIFI_FREQ_I2C_LOW_B0: *mut u32 = 0x6000_e11c as *mut u32;
const WIFI_FREQ_I2C_LOW_B1: *mut u32 = 0x6000_e120 as *mut u32;
const WIFI_FREQ_I2C_BYTE0: *mut u32 = 0x6000_e0c8 as *mut u32;
const WIFI_FREQ_I2C_BYTE1: *mut u32 = 0x6000_e0cc as *mut u32;
const WIFI_FREQ_I2C_BYTE2: *mut u32 = 0x6000_e114 as *mut u32;
const WIFI_FREQ_I2C_BYTE3: *mut u32 = 0x6000_e118 as *mut u32;
const WIFI_MAC_RX_CTRL0: *mut u32 = 0x6003_3100 as *mut u32;
const WIFI_MAC_RX_CTRL1: *mut u32 = 0x6003_3104 as *mut u32;
const WIFI_MAC_RX_CTRL2: *mut u32 = 0x6003_3108 as *mut u32;
const WIFI_MAC_RX_CTRL3: *mut u32 = 0x6003_310c as *mut u32;
const WIFI_MAC_RX_GLOBAL: *mut u32 = 0x6003_309c as *mut u32;
const WIFI_MAC_RX_ADDR0: *mut u32 = 0x6003_3040 as *mut u32;
const WIFI_MAC_RX_ADDR1: *mut u32 = 0x6003_3044 as *mut u32;
const WIFI_MAC_RX_ADDR_MASK0: *mut u32 = 0x6003_3060 as *mut u32;
const WIFI_MAC_RX_ADDR_MASK1: *mut u32 = 0x6003_3064 as *mut u32;
const WIFI_MAC_RX_CFG0: *mut u32 = 0x6003_3c5c as *mut u32;
const WIFI_MAC_RX_CFG1: *mut u32 = 0x6003_3c60 as *mut u32;
const WIFI_MAC_RX_CFG2: *mut u32 = 0x6003_3c64 as *mut u32;
const WIFI_MAC_RX_CFG3: *mut u32 = 0x6003_3080 as *mut u32;
const WIFI_MAC_RX_RELOAD: *mut u32 = 0x6003_3084 as *mut u32;
const WIFI_MAC_RX_BASE: *mut u32 = 0x6003_3088 as *mut u32;
const WIFI_MAC_RX_NEXT: *mut u32 = 0x6003_308c as *mut u32;
const WIFI_MAC_RX_LAST: *mut u32 = 0x6003_3090 as *mut u32;
const WIFI_MAC_RX_AUX0: *mut u32 = 0x6003_3094 as *mut u32;
const WIFI_MAC_RX_AUX1: *mut u32 = 0x6003_3098 as *mut u32;
const WIFI_MAC_RX_END_STATE: *mut u32 = 0x6003_30a8 as *mut u32;
const WIFI_MAC_RX_STATE0: *mut u32 = 0x6003_30ac as *mut u32;
const WIFI_MAC_RX_STATE1: *mut u32 = 0x6003_30b0 as *mut u32;
const WIFI_MAC_RX_FILTER_COUNT: *mut u32 = 0x6003_311c as *mut u32;
const WIFI_MAC_RX_FILTER_CTRL_BASE: *mut u32 = 0x6003_3120 as *mut u32;
const WIFI_MAC_RX_FILTER_PATTERN_BASE: *mut u32 = 0x6003_313c as *mut u32;
const WIFI_MAC_RX_FILTER_MASK_BASE: *mut u32 = 0x6003_3158 as *mut u32;
const WIFI_MAC_RX_INFO2: *mut u32 = 0x6003_3314 as *mut u32;
const WIFI_MAC_RX_INFO1: *mut u32 = 0x6003_3318 as *mut u32;
const WIFI_MAC_RX_INFO0: *mut u32 = 0x6003_331c as *mut u32;
const WIFI_MAC_RX_INFO3: *mut u32 = 0x6003_3320 as *mut u32;
const WIFI_PHY_NOISE_STATUS: *mut u32 = 0x6001_c06c as *mut u32;
const WIFI_MAC_RX_POLICY_A: *mut u32 = 0x6003_30dc as *mut u32;
const WIFI_MAC_RX_POLICY_B: *mut u32 = 0x6003_30e0 as *mut u32;
const WIFI_MAC_RX_POLICY_C: *mut u32 = 0x6003_30e4 as *mut u32;
const WIFI_MAC_CTRL_33114: *mut u32 = 0x6003_3114 as *mut u32;
const WIFI_MAC_CTRL_33118: *mut u32 = 0x6003_3118 as *mut u32;
const WIFI_MAC_SNIFFER_CTRL: *mut u32 = 0x6003_30e4 as *mut u32;
const WIFI_MAC_SNIFFER_MISC0: *mut u32 = 0x6003_30f4 as *mut u32;
const WIFI_MAC_SNIFFER_MISC1: *mut u32 = 0x6003_30f8 as *mut u32;
const WIFI_MAC_CTRL_332B8: *mut u32 = 0x6003_32b8 as *mut u32;
const WIFI_MAC_CTRL_33084: *mut u32 = 0x6003_3084 as *mut u32;
const WIFI_MAC_INTERRUPT_STATUS: *mut u32 = 0x6003_3c3c as *mut u32;
const WIFI_MAC_INTERRUPT_CLEAR: *mut u32 = 0x6003_3c40 as *mut u32;
const WIFI_MAC_RX_END0: *mut u32 = 0x6003_3d50 as *mut u32;
const WIFI_MAC_RX_END1: *mut u32 = 0x6003_3d54 as *mut u32;
const WIFI_MAC_DMA_STATE: *mut u32 = 0x6003_5128 as *mut u32;
const WIFI_COEX_CTRL: *mut u32 = 0x6003_5084 as *mut u32;
const WIFI_COEX_PTI: *mut u32 = 0x6003_32ac as *mut u32;
const WIFI_COEX_DEFAULT_PTI: *mut u32 = 0x6003_5094 as *mut u32;
const ROM_STA_RXCB: *mut u32 = 0x3fce_f838 as *mut u32;
const ROM_G_IC_PTR: *mut u32 = 0x3fce_f84c as *mut u32;
const ROM_G_EB_LIST_DESC_PTR: *mut u32 = 0x3fce_f92c as *mut u32;
const ROM_G_LMAC_CNT_PTR: *mut u32 = 0x3fce_f934 as *mut u32;
const ROM_WDEV_CTRL_PTR: *mut u32 = 0x3fce_f93c as *mut u32;
const ROM_PP_WDEV_FUNCS: *mut u32 = 0x3fce_f944 as *mut u32;
const ROM_LMAC_CONF_MIB_PTR: *mut u32 = 0x3fce_f950 as *mut u32;
const ROM_P_TX_RX: *mut u32 = 0x3fce_f954 as *mut u32;

const SYSTEM_WIFI_CLK_WIFI_BT_COMMON: u32 = 0x0078_078f;
const SYSTEM_WIFI_CLK_EN: u32 = 0x00fb_9fcf;
const SYSTEM_WIFI_BT_SDIO_CLK: u32 = (1 << 5) | (1 << 12) | (1 << 13);
const SYSTEM_MAC_RST: u32 = 1 << 2;
const RTC_CNTL_WIFI_FORCE_PD: u32 = 1 << 17;
const RTC_CNTL_WIFI_FORCE_ISO: u32 = 1 << 28;
const MODEM_RESET_FIELD_WHEN_POWERED: u32 =
    (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 9) | (1 << 11) | (1 << 13);

static LAST_STATUS: AtomicI32 = AtomicI32::new(0);

const RX_DESC_COUNT: usize = 4;
const RX_BUFFER_LEN: usize = 0x8a8;
const RX_BUFFER_USABLE_LEN: u32 = (RX_BUFFER_LEN as u32) - 4;
const RX_BUFFER_SENTINEL: u32 = 0xdead_beef;

#[repr(C)]
#[derive(Clone, Copy)]
struct RxDescriptor {
    control: u32,
    buffer: u32,
    next: u32,
}

impl RxDescriptor {
    const fn empty() -> Self {
        Self {
            control: 0,
            buffer: 0,
            next: 0,
        }
    }
}

#[repr(align(16))]
struct AlignedRxDescriptors([RxDescriptor; RX_DESC_COUNT]);

#[repr(align(16))]
struct AlignedRxBuffers([[u8; RX_BUFFER_LEN]; RX_DESC_COUNT]);

#[repr(align(16))]
struct AlignedWdevRxControl([u32; 14]);

static mut RX_DESCRIPTORS: AlignedRxDescriptors =
    AlignedRxDescriptors([RxDescriptor::empty(); RX_DESC_COUNT]);
static mut RX_BUFFERS: AlignedRxBuffers = AlignedRxBuffers([[0; RX_BUFFER_LEN]; RX_DESC_COUNT]);
static mut WDEV_RX_CONTROL: AlignedWdevRxControl = AlignedWdevRxControl([0; 14]);

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
    pub ctrl_33c00: u32,
    pub ctrl_33c34: u32,
    pub ctrl_33c40: u32,
    pub ctrl_33c74: u32,
    pub rx_ctrl0: u32,
    pub rx_ctrl1: u32,
    pub rx_ctrl2: u32,
    pub rx_ctrl3: u32,
    pub rx_global: u32,
    pub rx_addr0: u32,
    pub rx_addr1: u32,
    pub rx_addr_mask0: u32,
    pub rx_addr_mask1: u32,
    pub rx_cfg0: u32,
    pub rx_cfg1: u32,
    pub rx_cfg2: u32,
    pub rx_cfg3: u32,
    pub rx_base: u32,
    pub rx_filter_count: u32,
    pub rx_filter_ctrl0: u32,
    pub rx_filter_ctrl5: u32,
    pub rx_filter_pattern0: u32,
    pub rx_filter_pattern5: u32,
    pub rx_filter_mask0: u32,
    pub rx_filter_mask5: u32,
    pub ctrl_33114: u32,
    pub ctrl_33118: u32,
    pub sniffer_ctrl: u32,
    pub sniffer_misc0: u32,
    pub sniffer_misc1: u32,
    pub coex_ctrl: u32,
    pub coex_pti: u32,
    pub coex_default_pti: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioTxRegs {
    pub ctrl_33118: u32,
    pub ctrl_33c78: u32,
    pub tx_ctrl_33c10: u32,
    pub tx_ctrl_33c14: u32,
    pub tx_ctrl_33c18: u32,
    pub tx_ctrl_33c54: u32,
    pub tx_ctrl_33c88: u32,
    pub tx_ctrl_33c94: u32,
    pub ctrl_332b8: u32,
    pub ctrl_33084: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioRxScratchRegs {
    pub base: u32,
    pub next: u32,
    pub last: u32,
    pub aux0: u32,
    pub aux1: u32,
    pub ctrl_33c00: u32,
    pub reload: u32,
    pub interrupt_status: u32,
    pub interrupt_clear: u32,
    pub dma_state: u32,
    pub rx_end0: u32,
    pub rx_end1: u32,
    pub rx_end_state: u32,
    pub rx_state0: u32,
    pub rx_state1: u32,
    pub rx_info0: u32,
    pub rx_info1: u32,
    pub rx_info2: u32,
    pub rx_info3: u32,
    pub phy_noise_status: u32,
    pub systimer_value: u32,
    pub systimer_aux: u32,
    pub rom_wifi_ptrs: [u32; 8],
    pub ctrl_words: [u32; 4],
    pub desc_words: [u32; RX_DESC_COUNT * 3],
    pub buffer_words: [u32; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioRateRegs {
    pub rate_33404: u32,
    pub rate_33408: u32,
    pub rate_3340c: u32,
    pub rate_33410: u32,
    pub rate_33414: u32,
    pub rate_33418: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioCryptoRegs {
    pub crypto_33800: u32,
    pub crypto_33804: u32,
    pub crypto_33808: u32,
    pub crypto_3380c: u32,
    pub crypto_33810: u32,
    pub crypto_33840: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioAntennaRegs {
    pub ant0: u32,
    pub ant1: u32,
    pub ant7: u32,
    pub ant_aux: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioPhyRegs {
    pub low_rate_ctrl0: u32,
    pub low_rate_ctrl1: u32,
    pub tx_seed: u32,
    pub rx_11b_ctrl0: u32,
    pub rx_11b_ctrl1: u32,
    pub rx_11b_ctrl2: u32,
    pub rx_11b_ctrl3: u32,
    pub rx_2440m_ctrl: u32,
    pub bb_ctrl_1cc48: u32,
    pub modem_wifi_enable: u32,
    pub modem_ctrl_26010: u32,
    pub pbus_addr_ctrl: u32,
    pub pbus_data: u32,
    pub pbus_bank0: u32,
    pub pbus_bank1: u32,
    pub pbus_bank2: u32,
    pub pbus_bank3: u32,
    pub pbus_bank4: u32,
    pub pbus_bank5: u32,
    pub txrate_power0: u32,
    pub txrate_power15: u32,
    pub i2c_xpd_ctrl0: u32,
    pub i2c_xpd_ctrl1: u32,
    pub rf_ctrl: u32,
    pub txrx_ctrl: u32,
    pub freq_pll_cap: u32,
    pub freq_hw_ctrl: u32,
    pub freq_mem_ctrl: u32,
    pub freq_sw_ctrl: u32,
    pub freq_busy: u32,
    pub freq_status: u32,
    pub freq_mode_ctrl: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WifiMmioPhyFunSlots {
    pub table: u32,
    pub slot_008: u32,
    pub slot_00c: u32,
    pub slot_05c: u32,
    pub slot_06c: u32,
    pub slot_078: u32,
    pub slot_088: u32,
    pub slot_0c8: u32,
    pub slot_0d0: u32,
    pub slot_0fc: u32,
    pub slot_100: u32,
    pub slot_110: u32,
    pub slot_148: u32,
    pub slot_160: u32,
    pub slot_164: u32,
    pub slot_190: u32,
    pub slot_198: u32,
    pub slot_1a8: u32,
    pub slot_1b4: u32,
    pub slot_1d4: u32,
    pub slot_200: u32,
    pub slot_204: u32,
    pub slot_208: u32,
    pub slot_20c: u32,
    pub slot_224: u32,
    pub slot_22c: u32,
    pub slot_234: u32,
    pub slot_254: u32,
    pub slot_264: u32,
    pub slot_268: u32,
    pub slot_288: u32,
    pub slot_28c: u32,
}

pub struct Esp32s3WifiMmio;

impl Esp32s3WifiMmio {
    pub fn quiesce_after_soft_reset() {
        unsafe {
            WIFI_MAC_DMA_CTRL.write_volatile(0);
            WIFI_MAC_RX_RELOAD.write_volatile(0);
            WIFI_MAC_RX_BASE.write_volatile(0);
            WIFI_MAC_RX_NEXT.write_volatile(0);
            WIFI_MAC_RX_LAST.write_volatile(0);
            reset_wifi_mac();
        }
    }

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
                ctrl_33c00: WIFI_MAC_CTRL_33C00.read_volatile(),
                ctrl_33c34: WIFI_MAC_CTRL_33C34.read_volatile(),
                ctrl_33c40: WIFI_MAC_CTRL_33C40.read_volatile(),
                ctrl_33c74: WIFI_MAC_CTRL_33C74.read_volatile(),
                rx_ctrl0: WIFI_MAC_RX_CTRL0.read_volatile(),
                rx_ctrl1: WIFI_MAC_RX_CTRL1.read_volatile(),
                rx_ctrl2: WIFI_MAC_RX_CTRL2.read_volatile(),
                rx_ctrl3: WIFI_MAC_RX_CTRL3.read_volatile(),
                rx_global: WIFI_MAC_RX_GLOBAL.read_volatile(),
                rx_addr0: WIFI_MAC_RX_ADDR0.read_volatile(),
                rx_addr1: WIFI_MAC_RX_ADDR1.read_volatile(),
                rx_addr_mask0: WIFI_MAC_RX_ADDR_MASK0.read_volatile(),
                rx_addr_mask1: WIFI_MAC_RX_ADDR_MASK1.read_volatile(),
                rx_cfg0: WIFI_MAC_RX_CFG0.read_volatile(),
                rx_cfg1: WIFI_MAC_RX_CFG1.read_volatile(),
                rx_cfg2: WIFI_MAC_RX_CFG2.read_volatile(),
                rx_cfg3: WIFI_MAC_RX_CFG3.read_volatile(),
                rx_base: WIFI_MAC_RX_BASE.read_volatile(),
                rx_filter_count: WIFI_MAC_RX_FILTER_COUNT.read_volatile(),
                rx_filter_ctrl0: WIFI_MAC_RX_FILTER_CTRL_BASE.add(0).read_volatile(),
                rx_filter_ctrl5: WIFI_MAC_RX_FILTER_CTRL_BASE.add(5).read_volatile(),
                rx_filter_pattern0: WIFI_MAC_RX_FILTER_PATTERN_BASE.add(0).read_volatile(),
                rx_filter_pattern5: WIFI_MAC_RX_FILTER_PATTERN_BASE.add(5).read_volatile(),
                rx_filter_mask0: WIFI_MAC_RX_FILTER_MASK_BASE.add(0).read_volatile(),
                rx_filter_mask5: WIFI_MAC_RX_FILTER_MASK_BASE.add(5).read_volatile(),
                ctrl_33114: WIFI_MAC_CTRL_33114.read_volatile(),
                ctrl_33118: WIFI_MAC_CTRL_33118.read_volatile(),
                sniffer_ctrl: WIFI_MAC_SNIFFER_CTRL.read_volatile(),
                sniffer_misc0: WIFI_MAC_SNIFFER_MISC0.read_volatile(),
                sniffer_misc1: WIFI_MAC_SNIFFER_MISC1.read_volatile(),
                coex_ctrl: WIFI_COEX_CTRL.read_volatile(),
                coex_pti: WIFI_COEX_PTI.read_volatile(),
                coex_default_pti: WIFI_COEX_DEFAULT_PTI.read_volatile(),
            }
        }
    }

    pub fn debug_tx_regs() -> WifiMmioTxRegs {
        unsafe {
            WifiMmioTxRegs {
                ctrl_33118: WIFI_MAC_CTRL_33118.read_volatile(),
                ctrl_33c78: WIFI_MAC_CTRL_33C78.read_volatile(),
                tx_ctrl_33c10: WIFI_MAC_TX_CTRL_33C10.read_volatile(),
                tx_ctrl_33c14: WIFI_MAC_TX_CTRL_33C14.read_volatile(),
                tx_ctrl_33c18: WIFI_MAC_TX_CTRL_33C18.read_volatile(),
                tx_ctrl_33c54: WIFI_MAC_TX_CTRL_33C54.read_volatile(),
                tx_ctrl_33c88: WIFI_MAC_TX_CTRL_33C88.read_volatile(),
                tx_ctrl_33c94: WIFI_MAC_TX_CTRL_33C94.read_volatile(),
                ctrl_332b8: WIFI_MAC_CTRL_332B8.read_volatile(),
                ctrl_33084: WIFI_MAC_CTRL_33084.read_volatile(),
            }
        }
    }

    pub fn debug_rx_scratch_regs() -> WifiMmioRxScratchRegs {
        unsafe {
            let desc_words = core::ptr::addr_of!(RX_DESCRIPTORS.0).cast::<u32>();
            let mut desc_snapshot = [0u32; RX_DESC_COUNT * 3];
            let mut index = 0;
            while index < desc_snapshot.len() {
                desc_snapshot[index] = desc_words.add(index).read_volatile();
                index += 1;
            }

            let buffer_words = core::ptr::addr_of!(RX_BUFFERS.0).cast::<u32>();
            let mut buffer_snapshot = [0u32; 8];
            index = 0;
            while index < buffer_snapshot.len() {
                buffer_snapshot[index] = buffer_words.add(index).read_volatile();
                index += 1;
            }
            let ctrl_words = core::ptr::addr_of!(WDEV_RX_CONTROL.0).cast::<u32>();
            let mut ctrl_snapshot = [0u32; 4];
            index = 0;
            while index < ctrl_snapshot.len() {
                ctrl_snapshot[index] = ctrl_words.add(index).read_volatile();
                index += 1;
            }
            let rom_wifi_ptrs = [
                ROM_STA_RXCB.read_volatile(),
                ROM_G_IC_PTR.read_volatile(),
                ROM_G_EB_LIST_DESC_PTR.read_volatile(),
                ROM_G_LMAC_CNT_PTR.read_volatile(),
                ROM_WDEV_CTRL_PTR.read_volatile(),
                ROM_PP_WDEV_FUNCS.read_volatile(),
                ROM_LMAC_CONF_MIB_PTR.read_volatile(),
                ROM_P_TX_RX.read_volatile(),
            ];
            WifiMmioRxScratchRegs {
                base: WIFI_MAC_RX_BASE.read_volatile(),
                next: WIFI_MAC_RX_NEXT.read_volatile(),
                last: WIFI_MAC_RX_LAST.read_volatile(),
                aux0: WIFI_MAC_RX_AUX0.read_volatile(),
                aux1: WIFI_MAC_RX_AUX1.read_volatile(),
                ctrl_33c00: WIFI_MAC_CTRL_33C00.read_volatile(),
                reload: WIFI_MAC_RX_RELOAD.read_volatile(),
                interrupt_status: WIFI_MAC_INTERRUPT_STATUS.read_volatile(),
                interrupt_clear: WIFI_MAC_INTERRUPT_CLEAR.read_volatile(),
                dma_state: WIFI_MAC_DMA_STATE.read_volatile(),
                rx_end0: WIFI_MAC_RX_END0.read_volatile(),
                rx_end1: WIFI_MAC_RX_END1.read_volatile(),
                rx_end_state: WIFI_MAC_RX_END_STATE.read_volatile(),
                rx_state0: WIFI_MAC_RX_STATE0.read_volatile(),
                rx_state1: WIFI_MAC_RX_STATE1.read_volatile(),
                rx_info0: WIFI_MAC_RX_INFO0.read_volatile(),
                rx_info1: WIFI_MAC_RX_INFO1.read_volatile(),
                rx_info2: WIFI_MAC_RX_INFO2.read_volatile(),
                rx_info3: WIFI_MAC_RX_INFO3.read_volatile(),
                phy_noise_status: WIFI_PHY_NOISE_STATUS.read_volatile(),
                systimer_value: WIFI_SYSTIMER_VALUE.read_volatile(),
                systimer_aux: WIFI_SYSTIMER_AUX.read_volatile(),
                rom_wifi_ptrs,
                ctrl_words: ctrl_snapshot,
                desc_words: desc_snapshot,
                buffer_words: buffer_snapshot,
            }
        }
    }

    pub fn debug_rate_regs() -> WifiMmioRateRegs {
        unsafe {
            WifiMmioRateRegs {
                rate_33404: WIFI_MAC_RATE_CTRL_33404.read_volatile(),
                rate_33408: WIFI_MAC_RATE_CTRL_33408.read_volatile(),
                rate_3340c: WIFI_MAC_RATE_CTRL_3340C.read_volatile(),
                rate_33410: WIFI_MAC_RATE_CTRL_33410.read_volatile(),
                rate_33414: WIFI_MAC_RATE_CTRL_33414.read_volatile(),
                rate_33418: WIFI_MAC_RATE_CTRL_33418.read_volatile(),
            }
        }
    }

    pub fn debug_crypto_regs() -> WifiMmioCryptoRegs {
        unsafe {
            WifiMmioCryptoRegs {
                crypto_33800: WIFI_MAC_CRYPTO_CTRL_33800.read_volatile(),
                crypto_33804: WIFI_MAC_CRYPTO_CTRL_33804.read_volatile(),
                crypto_33808: WIFI_MAC_CRYPTO_CTRL_33808.read_volatile(),
                crypto_3380c: WIFI_MAC_CRYPTO_CTRL_3380C.read_volatile(),
                crypto_33810: WIFI_MAC_CRYPTO_CTRL_33810.read_volatile(),
                crypto_33840: WIFI_MAC_CRYPTO_CTRL_33840.read_volatile(),
            }
        }
    }

    pub fn debug_antenna_regs() -> WifiMmioAntennaRegs {
        unsafe {
            WifiMmioAntennaRegs {
                ant0: WIFI_MAC_ANT_CTRL_START.read_volatile(),
                ant1: WIFI_MAC_ANT_CTRL_START
                    .sub(WIFI_MAC_ANT_CTRL_STRIDE_WORDS)
                    .read_volatile(),
                ant7: WIFI_MAC_ANT_CTRL_START
                    .sub(WIFI_MAC_ANT_CTRL_STRIDE_WORDS * 7)
                    .read_volatile(),
                ant_aux: WIFI_MAC_ANT_CTRL_332A8.read_volatile(),
            }
        }
    }

    pub fn debug_phy_regs() -> WifiMmioPhyRegs {
        unsafe {
            WifiMmioPhyRegs {
                low_rate_ctrl0: WIFI_PHY_LOW_RATE_CTRL0.read_volatile(),
                low_rate_ctrl1: WIFI_PHY_LOW_RATE_CTRL1.read_volatile(),
                tx_seed: WIFI_PHY_TX_SEED.read_volatile(),
                rx_11b_ctrl0: WIFI_PHY_RX_11B_CTRL0.read_volatile(),
                rx_11b_ctrl1: WIFI_PHY_RX_11B_CTRL1.read_volatile(),
                rx_11b_ctrl2: WIFI_PHY_RX_11B_CTRL2.read_volatile(),
                rx_11b_ctrl3: WIFI_PHY_RX_11B_CTRL3.read_volatile(),
                rx_2440m_ctrl: WIFI_PHY_RX_2440M_CTRL.read_volatile(),
                bb_ctrl_1cc48: WIFI_PHY_BB_CTRL_1CC48.read_volatile(),
                modem_wifi_enable: WIFI_MODEM_WIFI_ENABLE.read_volatile(),
                modem_ctrl_26010: WIFI_MODEM_CTRL_26010.read_volatile(),
                pbus_addr_ctrl: WIFI_PBUS_ADDR_CTRL.read_volatile(),
                pbus_data: WIFI_PBUS_DATA.read_volatile(),
                pbus_bank0: WIFI_PBUS_BANK0.read_volatile(),
                pbus_bank1: WIFI_PBUS_BANK1.read_volatile(),
                pbus_bank2: WIFI_PBUS_BANK2.read_volatile(),
                pbus_bank3: WIFI_PBUS_BANK3.read_volatile(),
                pbus_bank4: WIFI_PBUS_BANK4.read_volatile(),
                pbus_bank5: WIFI_PBUS_BANK5.read_volatile(),
                txrate_power0: WIFI_TXRATE_POWER0.read_volatile(),
                txrate_power15: WIFI_TXRATE_POWER15.read_volatile(),
                i2c_xpd_ctrl0: WIFI_I2C_XPD_CTRL0.read_volatile(),
                i2c_xpd_ctrl1: WIFI_I2C_XPD_CTRL1.read_volatile(),
                rf_ctrl: WIFI_RF_CTRL.read_volatile(),
                txrx_ctrl: WIFI_TXRX_CTRL.read_volatile(),
                freq_pll_cap: WIFI_FREQ_PLL_CAP.read_volatile(),
                freq_hw_ctrl: WIFI_FREQ_HW_CTRL.read_volatile(),
                freq_mem_ctrl: WIFI_FREQ_MEM_CTRL.read_volatile(),
                freq_sw_ctrl: WIFI_FREQ_SW_CTRL.read_volatile(),
                freq_busy: WIFI_FREQ_BUSY.read_volatile(),
                freq_status: WIFI_FREQ_STATUS.read_volatile(),
                freq_mode_ctrl: WIFI_FREQ_MODE_CTRL.read_volatile(),
            }
        }
    }

    pub fn debug_phy_fun_slots() -> WifiMmioPhyFunSlots {
        unsafe {
            let table = phy_get_romfuncs();
            WifiMmioPhyFunSlots {
                table: table as usize as u32,
                slot_008: read_phy_fun_slot(table, 0x008),
                slot_00c: read_phy_fun_slot(table, 0x00c),
                slot_05c: read_phy_fun_slot(table, 0x05c),
                slot_06c: read_phy_fun_slot(table, 0x06c),
                slot_078: read_phy_fun_slot(table, 0x078),
                slot_088: read_phy_fun_slot(table, 0x088),
                slot_0c8: read_phy_fun_slot(table, 0x0c8),
                slot_0d0: read_phy_fun_slot(table, 0x0d0),
                slot_0fc: read_phy_fun_slot(table, 0x0fc),
                slot_100: read_phy_fun_slot(table, 0x100),
                slot_110: read_phy_fun_slot(table, 0x110),
                slot_148: read_phy_fun_slot(table, 0x148),
                slot_160: read_phy_fun_slot(table, 0x160),
                slot_164: read_phy_fun_slot(table, 0x164),
                slot_190: read_phy_fun_slot(table, 0x190),
                slot_198: read_phy_fun_slot(table, 0x198),
                slot_1a8: read_phy_fun_slot(table, 0x1a8),
                slot_1b4: read_phy_fun_slot(table, 0x1b4),
                slot_1d4: read_phy_fun_slot(table, 0x1d4),
                slot_200: read_phy_fun_slot(table, 0x200),
                slot_204: read_phy_fun_slot(table, 0x204),
                slot_208: read_phy_fun_slot(table, 0x208),
                slot_20c: read_phy_fun_slot(table, 0x20c),
                slot_224: read_phy_fun_slot(table, 0x224),
                slot_22c: read_phy_fun_slot(table, 0x22c),
                slot_234: read_phy_fun_slot(table, 0x234),
                slot_254: read_phy_fun_slot(table, 0x254),
                slot_264: read_phy_fun_slot(table, 0x264),
                slot_268: read_phy_fun_slot(table, 0x268),
                slot_288: read_phy_fun_slot(table, 0x288),
                slot_28c: read_phy_fun_slot(table, 0x28c),
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
                5 => {
                    LAST_STATUS.store(501, Ordering::Relaxed);
                    init_hal_tail_slice();
                    LAST_STATUS.store(502, Ordering::Relaxed);
                    true
                }
                6 => {
                    LAST_STATUS.store(601, Ordering::Relaxed);
                    init_rx_buffer_slice();
                    LAST_STATUS.store(602, Ordering::Relaxed);
                    true
                }
                7 => {
                    LAST_STATUS.store(701, Ordering::Relaxed);
                    init_rx_filter_slice();
                    LAST_STATUS.store(702, Ordering::Relaxed);
                    true
                }
                8 => {
                    LAST_STATUS.store(801, Ordering::Relaxed);
                    init_mac_txrx_tail_slice();
                    LAST_STATUS.store(802, Ordering::Relaxed);
                    true
                }
                9 => {
                    LAST_STATUS.store(901, Ordering::Relaxed);
                    init_mac_rate_slice();
                    LAST_STATUS.store(902, Ordering::Relaxed);
                    true
                }
                10 => {
                    LAST_STATUS.store(1001, Ordering::Relaxed);
                    init_crypto_slice();
                    LAST_STATUS.store(1002, Ordering::Relaxed);
                    true
                }
                11 => {
                    LAST_STATUS.store(1101, Ordering::Relaxed);
                    init_antenna_slice();
                    LAST_STATUS.store(1102, Ordering::Relaxed);
                    true
                }
                12 => {
                    LAST_STATUS.store(1201, Ordering::Relaxed);
                    init_phy_low_rate_slice();
                    LAST_STATUS.store(1202, Ordering::Relaxed);
                    true
                }
                13 => {
                    LAST_STATUS.store(1301, Ordering::Relaxed);
                    set_phy_tx_seed(37);
                    LAST_STATUS.store(1302, Ordering::Relaxed);
                    true
                }
                14 => {
                    LAST_STATUS.store(1401, Ordering::Relaxed);
                    init_phy_rx_11b_opt_slice();
                    LAST_STATUS.store(1402, Ordering::Relaxed);
                    true
                }
                15 => {
                    LAST_STATUS.store(1501, Ordering::Relaxed);
                    init_phy_bb_reg_slice();
                    LAST_STATUS.store(1502, Ordering::Relaxed);
                    true
                }
                16 => {
                    LAST_STATUS.store(1601, Ordering::Relaxed);
                    set_phy_wifi_enable(true);
                    LAST_STATUS.store(1602, Ordering::Relaxed);
                    true
                }
                17 => {
                    LAST_STATUS.store(1701, Ordering::Relaxed);
                    init_pbus_mem_slice();
                    LAST_STATUS.store(1702, Ordering::Relaxed);
                    true
                }
                18 => {
                    LAST_STATUS.store(1801, Ordering::Relaxed);
                    write_txrate_power_offset_slice();
                    LAST_STATUS.store(1802, Ordering::Relaxed);
                    true
                }
                19 => {
                    LAST_STATUS.store(1901, Ordering::Relaxed);
                    open_i2c_xpd_slice();
                    LAST_STATUS.store(1902, Ordering::Relaxed);
                    true
                }
                20 => {
                    LAST_STATUS.store(2001, Ordering::Relaxed);
                    rf_init_direct_slice();
                    LAST_STATUS.store(2002, Ordering::Relaxed);
                    true
                }
                21 => {
                    LAST_STATUS.store(2101, Ordering::Relaxed);
                    force_txrx_off_slice(true);
                    LAST_STATUS.store(2102, Ordering::Relaxed);
                    true
                }
                22 => {
                    LAST_STATUS.store(2201, Ordering::Relaxed);
                    set_chan_freq_hw_init_direct_slice(2, 4);
                    LAST_STATUS.store(2202, Ordering::Relaxed);
                    true
                }
                23 => {
                    LAST_STATUS.store(2301, Ordering::Relaxed);
                    set_chan_freq_hw_init_direct_slice(1, 4);
                    LAST_STATUS.store(2302, Ordering::Relaxed);
                    true
                }
                24 => {
                    LAST_STATUS.store(2401, Ordering::Relaxed);
                    set_chan_freq_hw_init_direct_slice(6, 4);
                    LAST_STATUS.store(2402, Ordering::Relaxed);
                    true
                }
                25 => {
                    LAST_STATUS.store(2501, Ordering::Relaxed);
                    set_chan_freq_hw_init_direct_slice(11, 4);
                    LAST_STATUS.store(2502, Ordering::Relaxed);
                    true
                }
                26 => {
                    LAST_STATUS.store(2601, Ordering::Relaxed);
                    set_chan_freq_sw_start_direct_slice(11);
                    enable_mac_direct_slice();
                    release_txrx_force_direct_slice();
                    enable_rx_direct_slice();
                    LAST_STATUS.store(2602, Ordering::Relaxed);
                    true
                }
                27 => {
                    LAST_STATUS.store(2701, Ordering::Relaxed);
                    pll_cap_mem_update_direct_slice(0);
                    LAST_STATUS.store(2702, Ordering::Relaxed);
                    true
                }
                28 => {
                    LAST_STATUS.store(2801, Ordering::Relaxed);
                    phy_en_hw_set_freq_direct_slice();
                    LAST_STATUS.store(2802, Ordering::Relaxed);
                    true
                }
                29 => {
                    LAST_STATUS.store(2901, Ordering::Relaxed);
                    phy_dis_hw_set_freq_direct_slice();
                    LAST_STATUS.store(2902, Ordering::Relaxed);
                    true
                }
                30 => {
                    LAST_STATUS.store(3001, Ordering::Relaxed);
                    freq_i2c_data_write_direct_slice();
                    LAST_STATUS.store(3002, Ordering::Relaxed);
                    true
                }
                31 => {
                    LAST_STATUS.store(3101, Ordering::Relaxed);
                    set_chan_freq_sw_start_direct_slice(171);
                    LAST_STATUS.store(3102, Ordering::Relaxed);
                    true
                }
                32 => {
                    LAST_STATUS.store(3201, Ordering::Relaxed);
                    enable_sniffer_direct_slice();
                    LAST_STATUS.store(3202, Ordering::Relaxed);
                    true
                }
                33 => {
                    LAST_STATUS.store(3301, Ordering::Relaxed);
                    poll_rx_event_direct_slice();
                    LAST_STATUS.store(3302, Ordering::Relaxed);
                    true
                }
                34 => {
                    LAST_STATUS.store(3401, Ordering::Relaxed);
                    set_chan_freq_sw_start_direct_slice(1);
                    enable_mac_direct_slice();
                    release_txrx_force_direct_slice();
                    enable_rx_direct_slice();
                    LAST_STATUS.store(3402, Ordering::Relaxed);
                    true
                }
                35 => {
                    LAST_STATUS.store(3501, Ordering::Relaxed);
                    set_chan_freq_sw_start_direct_slice(6);
                    enable_mac_direct_slice();
                    release_txrx_force_direct_slice();
                    enable_rx_direct_slice();
                    LAST_STATUS.store(3502, Ordering::Relaxed);
                    true
                }
                36 => {
                    LAST_STATUS.store(3601, Ordering::Relaxed);
                    set_chan_freq_sw_start_direct_slice(11);
                    enable_mac_direct_slice();
                    release_txrx_force_direct_slice();
                    enable_rx_direct_slice();
                    LAST_STATUS.store(3602, Ordering::Relaxed);
                    true
                }
                37 => {
                    LAST_STATUS.store(3701, Ordering::Relaxed);
                    enable_rx_direct_slice();
                    LAST_STATUS.store(3702, Ordering::Relaxed);
                    true
                }
                38 => {
                    LAST_STATUS.store(3801, Ordering::Relaxed);
                    release_txrx_force_direct_slice();
                    LAST_STATUS.store(3802, Ordering::Relaxed);
                    true
                }
                39 => {
                    LAST_STATUS.store(3901, Ordering::Relaxed);
                    enable_mac_direct_slice();
                    LAST_STATUS.store(3902, Ordering::Relaxed);
                    true
                }
                40 => {
                    LAST_STATUS.store(4001, Ordering::Relaxed);
                    enable_rftest_rx_policy_direct_slice();
                    LAST_STATUS.store(4002, Ordering::Relaxed);
                    true
                }
                41 => {
                    LAST_STATUS.store(4101, Ordering::Relaxed);
                    enable_rftest_rx_gate_direct_slice();
                    LAST_STATUS.store(4102, Ordering::Relaxed);
                    true
                }
                42 => {
                    LAST_STATUS.store(4201, Ordering::Relaxed);
                    trigger_rftest_rx_buffer_direct_slice();
                    LAST_STATUS.store(4202, Ordering::Relaxed);
                    true
                }
                43 => {
                    LAST_STATUS.store(4301, Ordering::Relaxed);
                    enable_rftest_rx_accept_filter_direct_slice();
                    LAST_STATUS.store(4302, Ordering::Relaxed);
                    true
                }
                44 => {
                    LAST_STATUS.store(4401, Ordering::Relaxed);
                    init_rftest_rx_2440m_opt_direct_slice();
                    LAST_STATUS.store(4402, Ordering::Relaxed);
                    true
                }
                45 => {
                    LAST_STATUS.store(4501, Ordering::Relaxed);
                    init_rx_descriptor_ring_with_eof();
                    LAST_STATUS.store(4502, Ordering::Relaxed);
                    true
                }
                46 => {
                    LAST_STATUS.store(4601, Ordering::Relaxed);
                    init_wdev_rx_buffer_slice();
                    LAST_STATUS.store(4602, Ordering::Relaxed);
                    true
                }
                47 => {
                    LAST_STATUS.store(4701, Ordering::Relaxed);
                    init_rftest_rx_mac_filter_direct_slice();
                    LAST_STATUS.store(4702, Ordering::Relaxed);
                    true
                }
                48 => {
                    LAST_STATUS.store(4801, Ordering::Relaxed);
                    init_rftest_rx_per_filter_direct_slice();
                    LAST_STATUS.store(4802, Ordering::Relaxed);
                    true
                }
                49 => {
                    LAST_STATUS.store(4901, Ordering::Relaxed);
                    init_rftest_rx_pbus_direct_slice(0, 0, 0, 0);
                    LAST_STATUS.store(4902, Ordering::Relaxed);
                    true
                }
                50 => {
                    LAST_STATUS.store(5001, Ordering::Relaxed);
                    init_rftest_rx_pbus_direct_slice(4, 8, 4, 8);
                    LAST_STATUS.store(5002, Ordering::Relaxed);
                    true
                }
                51 => {
                    LAST_STATUS.store(5101, Ordering::Relaxed);
                    let ok = call_phy_rx_start_slot_direct_slice();
                    LAST_STATUS.store(if ok { 5102 } else { -5102 }, Ordering::Relaxed);
                    ok
                }
                52 => {
                    LAST_STATUS.store(5201, Ordering::Relaxed);
                    let ok = call_phy_pbus_debug_slot_direct_slice();
                    LAST_STATUS.store(if ok { 5202 } else { -5202 }, Ordering::Relaxed);
                    ok
                }
                53 => {
                    LAST_STATUS.store(5301, Ordering::Relaxed);
                    install_rom_rx_globals_slice();
                    LAST_STATUS.store(5302, Ordering::Relaxed);
                    true
                }
                54 => {
                    LAST_STATUS.store(5401, Ordering::Relaxed);
                    install_rom_rx_globals_slice();
                    let ok = call_phy_rx_start_slot_direct_slice();
                    LAST_STATUS.store(if ok { 5402 } else { -5402 }, Ordering::Relaxed);
                    ok
                }
                55 => {
                    LAST_STATUS.store(5501, Ordering::Relaxed);
                    install_rom_rx_globals_direct_dma_slice();
                    LAST_STATUS.store(5502, Ordering::Relaxed);
                    true
                }
                56 => {
                    LAST_STATUS.store(5601, Ordering::Relaxed);
                    install_rom_rx_globals_direct_dma_slice();
                    let ok = call_phy_rx_start_slot_direct_slice();
                    LAST_STATUS.store(if ok { 5602 } else { -5602 }, Ordering::Relaxed);
                    ok
                }
                57 => {
                    LAST_STATUS.store(5701, Ordering::Relaxed);
                    let ok = call_phy_slot0(0x160);
                    LAST_STATUS.store(if ok { 5702 } else { -5702 }, Ordering::Relaxed);
                    ok
                }
                58 => {
                    LAST_STATUS.store(5801, Ordering::Relaxed);
                    let ok = call_phy_slot0(0x008);
                    LAST_STATUS.store(if ok { 5802 } else { -5802 }, Ordering::Relaxed);
                    ok
                }
                59 => {
                    LAST_STATUS.store(5901, Ordering::Relaxed);
                    force_txrx_off_slice(true);
                    set_chan_freq_hw_init_direct_slice(6, 0);
                    set_chan_freq_sw_start_direct_slice(6);
                    let ok = call_phy_slot1(0x06c, 0);
                    LAST_STATUS.store(if ok { 5902 } else { -5902 }, Ordering::Relaxed);
                    ok
                }
                60 => {
                    LAST_STATUS.store(6001, Ordering::Relaxed);
                    let ok = call_phy_slot1(0x24c, 1);
                    LAST_STATUS.store(if ok { 6002 } else { -6002 }, Ordering::Relaxed);
                    ok
                }
                61 => {
                    LAST_STATUS.store(6101, Ordering::Relaxed);
                    let ok = call_phy_slot2(0x264, 6, 0);
                    LAST_STATUS.store(if ok { 6102 } else { -6102 }, Ordering::Relaxed);
                    ok
                }
                62 => {
                    LAST_STATUS.store(6201, Ordering::Relaxed);
                    force_txrx_off_slice(false);
                    let ok = call_phy_slot0(0x00c);
                    LAST_STATUS.store(if ok { 6202 } else { -6202 }, Ordering::Relaxed);
                    ok
                }
                63 => {
                    LAST_STATUS.store(6301, Ordering::Relaxed);
                    let ok = call_phy_slot1(0x164, 0);
                    LAST_STATUS.store(if ok { 6302 } else { -6302 }, Ordering::Relaxed);
                    ok
                }
                64 => {
                    LAST_STATUS.store(6401, Ordering::Relaxed);
                    let ok = call_phy_rftest_channel6_slot_path();
                    LAST_STATUS.store(if ok { 6402 } else { -6402 }, Ordering::Relaxed);
                    ok
                }
                _ => false,
            }
        }
    }

    pub fn init_known_good() -> bool {
        for step in [
            0, 3, 4, 5, 6, 7, 32, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        ] {
            if !Self::debug_step(step) {
                return false;
            }
        }
        true
    }
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn read_phy_fun_slot(table: *mut core::ffi::c_void, offset: usize) -> u32 {
    if table.is_null() {
        return 0;
    }
    unsafe { table.cast::<u8>().add(offset).cast::<u32>().read_volatile() }
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn read_phy_fun_slot(_table: *mut core::ffi::c_void, _offset: usize) -> u32 {
    0
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
    enable_mac_direct_slice();
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

unsafe fn init_hal_tail_slice() {
    unsafe {
        WIFI_MAC_CTRL_33C34.write_volatile(0x19a8_79e0);
    }
    update(WIFI_MAC_DMA_CTRL, |v| v | (1 << 28));
    update(WIFI_MAC_RX_GLOBAL, |v| (v & 0xffff_ff00) | 1);
    update(WIFI_MAC_RX_GLOBAL, |v| (v & 0xffff_00ff) | 0x200);
    update(WIFI_MAC_RX_GLOBAL, |v| v | (1 << 20));

    update(WIFI_COEX_CTRL, |v| v | 2);
    update(WIFI_COEX_PTI, |v| v & !0xff);
    update(WIFI_COEX_DEFAULT_PTI, |v| v & !0x0f00);
}

unsafe fn init_rx_buffer_slice() {
    update(WIFI_MAC_RX_CFG0, |v| (v & 0xfff0_0000) | (31 << 15));
    update(WIFI_MAC_RX_CFG1, |v| (v & 0xfff0_0000) | (33 << 14));
    update(WIFI_MAC_RX_CFG2, |v| (v & 0x000f_ffff) | (255 << 22));
    update(WIFI_MAC_RX_CFG3, |v| v & 0xffff_ff00);

    init_rx_descriptor_ring();
}

unsafe fn init_wdev_rx_buffer_slice() {
    update(WIFI_MAC_RX_CFG0, |v| (v & 0xfff0_0000) | (31 << 15));
    update(WIFI_MAC_RX_CFG1, |v| (v & 0xfff0_0000) | (33 << 14));
    update(WIFI_MAC_RX_CFG2, |v| (v & 0x000f_ffff) | (255 << 22));
    update(WIFI_MAC_RX_CFG3, |v| v & 0xffff_ff00);

    init_wdev_rx_descriptor_ring();
}

unsafe fn init_rx_descriptor_ring() {
    init_rx_descriptor_ring_with_control(encode_rx_descriptor_control(RX_BUFFER_USABLE_LEN));
}

unsafe fn init_rx_descriptor_ring_with_eof() {
    init_rx_descriptor_ring_with_control(encode_rx_descriptor_control_with_eof(
        RX_BUFFER_USABLE_LEN,
    ));
}

unsafe fn init_rx_descriptor_ring_with_control(control: u32) {
    let desc_base = core::ptr::addr_of_mut!(RX_DESCRIPTORS.0).cast::<RxDescriptor>();
    let buffer_base = core::ptr::addr_of_mut!(RX_BUFFERS.0).cast::<u8>();

    for index in 0..RX_DESC_COUNT {
        let desc = desc_base.add(index);
        let buffer = buffer_base.add(index * RX_BUFFER_LEN);
        zero_rx_buffer(buffer);

        let next = if index + 1 < RX_DESC_COUNT {
            desc_base.add(index + 1) as usize as u32
        } else {
            0
        };

        (*desc).buffer = buffer as usize as u32;
        (*desc).next = next;
        (*desc).control = control;
        buffer.cast::<u32>().write_volatile(RX_BUFFER_SENTINEL);
        buffer
            .add(RX_BUFFER_USABLE_LEN as usize)
            .cast::<u32>()
            .write_volatile(RX_BUFFER_SENTINEL);
    }

    unsafe {
        let base = desc_base as usize as u32;
        WIFI_MAC_RX_BASE.write_volatile(base);
        WIFI_MAC_RX_NEXT.write_volatile(0);
        WIFI_MAC_RX_LAST.write_volatile(desc_base.add(RX_DESC_COUNT - 1) as usize as u32);
    }
}

unsafe fn init_wdev_rx_descriptor_ring() {
    let ctrl = core::ptr::addr_of_mut!(WDEV_RX_CONTROL.0).cast::<u32>();
    let desc_base = core::ptr::addr_of_mut!(RX_DESCRIPTORS.0).cast::<RxDescriptor>();
    let buffer_base = core::ptr::addr_of_mut!(RX_BUFFERS.0).cast::<u8>();
    let control = encode_rx_descriptor_control(RX_BUFFER_USABLE_LEN);

    for index in 0..14 {
        ctrl.add(index).write_volatile(0);
    }

    for index in 0..RX_DESC_COUNT {
        let desc = desc_base.add(index);
        let buffer = buffer_base.add(index * RX_BUFFER_LEN);
        zero_rx_buffer(buffer);

        let next = if index + 1 < RX_DESC_COUNT {
            desc_base.add(index + 1) as usize as u32
        } else {
            0
        };

        (*desc).control = control;
        (*desc).buffer = buffer as usize as u32;
        (*desc).next = next;
        buffer.cast::<u32>().write_volatile(RX_BUFFER_SENTINEL);
        buffer
            .add(RX_BUFFER_USABLE_LEN as usize)
            .cast::<u32>()
            .write_volatile(RX_BUFFER_SENTINEL);
    }

    ctrl.add(0).write_volatile(desc_base as usize as u32);
    ctrl.add(1)
        .write_volatile(desc_base.add(RX_DESC_COUNT - 1) as usize as u32);
    ctrl.add(2).write_volatile(desc_base as usize as u32);

    let ctrl_base = ctrl as usize as u32;
    WIFI_MAC_RX_BASE.write_volatile(ctrl_base);
    WIFI_MAC_RX_NEXT.write_volatile(0);
    WIFI_MAC_RX_LAST.write_volatile(desc_base.add(RX_DESC_COUNT - 1) as usize as u32);
}

unsafe fn install_rom_rx_globals_slice() {
    init_wdev_rx_descriptor_ring();

    let ctrl = core::ptr::addr_of_mut!(WDEV_RX_CONTROL.0).cast::<u32>() as usize as u32;
    let desc_base =
        core::ptr::addr_of_mut!(RX_DESCRIPTORS.0).cast::<RxDescriptor>() as usize as u32;

    ROM_G_EB_LIST_DESC_PTR.write_volatile(desc_base);
    ROM_WDEV_CTRL_PTR.write_volatile(ctrl);
    ROM_P_TX_RX.write_volatile(ctrl);
}

unsafe fn install_rom_rx_globals_direct_dma_slice() {
    init_rx_descriptor_ring();

    let ctrl = core::ptr::addr_of_mut!(WDEV_RX_CONTROL.0).cast::<u32>();
    let desc_base = core::ptr::addr_of_mut!(RX_DESCRIPTORS.0).cast::<RxDescriptor>();

    ctrl.add(0).write_volatile(desc_base as usize as u32);
    ctrl.add(1)
        .write_volatile(desc_base.add(RX_DESC_COUNT - 1) as usize as u32);
    ctrl.add(2).write_volatile(desc_base as usize as u32);

    ROM_G_EB_LIST_DESC_PTR.write_volatile(desc_base as usize as u32);
    ROM_WDEV_CTRL_PTR.write_volatile(ctrl as usize as u32);
    ROM_P_TX_RX.write_volatile(ctrl as usize as u32);
}

fn encode_rx_descriptor_control(len: u32) -> u32 {
    let len = len & 0x0fff;
    0x8000_0000 | len | (len << 12)
}

fn encode_rx_descriptor_control_with_eof(len: u32) -> u32 {
    encode_rx_descriptor_control(len) | 0x4000_0000
}

unsafe fn zero_rx_buffer(buffer: *mut u8) {
    let mut offset = 0;
    while offset < RX_BUFFER_LEN {
        buffer.add(offset).write_volatile(0);
        offset += 1;
    }
}

unsafe fn init_rx_filter_slice() {
    let ctrl = [
        0x0002_3006,
        0x0002_3006,
        0x0002_3006,
        0x0002_301c,
        0x0002_301c,
        0x0002_3011,
    ];
    let pattern = [
        0x0000_0608,
        0x0000_0808,
        0x0000_8e88,
        0x4400_4300,
        0x4300_4400,
        0x0000_0001,
    ];
    let mask = [
        0x0000_ffff,
        0x0000_ffff,
        0x0000_ffff,
        0xffff_ffff,
        0xffff_ffff,
        0x0000_00ff,
    ];

    for (index, value) in ctrl.iter().enumerate() {
        unsafe {
            WIFI_MAC_RX_FILTER_CTRL_BASE
                .add(index)
                .write_volatile(*value);
        }
    }
    for (index, value) in pattern.iter().enumerate() {
        unsafe {
            WIFI_MAC_RX_FILTER_PATTERN_BASE
                .add(index)
                .write_volatile(*value);
        }
    }
    for (index, value) in mask.iter().enumerate() {
        unsafe {
            WIFI_MAC_RX_FILTER_MASK_BASE
                .add(index)
                .write_volatile(*value);
        }
    }

    update(WIFI_MAC_RX_FILTER_COUNT, |v| v | (63 << 8));
    update(WIFI_MAC_RX_FILTER_COUNT, |v| v | 126);
    update(WIFI_MAC_RX_GLOBAL, |v| v | (1 << 27));
}

unsafe fn enable_sniffer_direct_slice() {
    update(WIFI_MAC_SNIFFER_CTRL, |v| (v & !0x0000_000f) | 0x0000_80e0);
    update(WIFI_MAC_SNIFFER_MISC0, |v| (v & 0xffff_0000) | 0x0000_78f8);
    update(WIFI_MAC_SNIFFER_MISC1, |v| (v & 0xffff_0000) | 0x0000_40ff);
    update(WIFI_MAC_RX_CTRL0, |v| v | 4);
    update(WIFI_MAC_RX_CTRL1, |v| v | 4);
    update(WIFI_MAC_RX_CTRL3, |v| v | 4);
    update(WIFI_MAC_CTRL_33C34, |v| v | 4);
}

unsafe fn enable_mac_direct_slice() {
    update(WIFI_MAC_CTRL_33C00, |v| v & !0x0000_00f0);
}

unsafe fn poll_rx_event_direct_slice() {
    let pending = unsafe { WIFI_MAC_INTERRUPT_STATUS.read_volatile() };
    if pending != 0 {
        unsafe {
            WIFI_MAC_INTERRUPT_CLEAR.write_volatile(pending);
        }
    }
    update(WIFI_MAC_RX_RELOAD, |v| v | 1);
}

unsafe fn enable_rftest_rx_policy_direct_slice() {
    WIFI_MAC_RX_POLICY_BASE.write_volatile(0x0000_7960);
    WIFI_MAC_INTERRUPT_CLEAR.write_volatile(0x0000_000c);
    WIFI_MAC_RX_RELOAD.write_volatile(0x8000_0000);
}

unsafe fn enable_rftest_rx_gate_direct_slice() {
    update(WIFI_MAC_RX_RELOAD, |v| v & !0x2000_0000);
    update(WIFI_MAC_RX_CFG3, |v| v & !0x1000_0000);
    WIFI_MAC_INTERRUPT_CLEAR.write_volatile(0x0000_000c);
    update(WIFI_MAC_RX_RELOAD, |v| v | 0x8000_0000);
}

unsafe fn trigger_rftest_rx_buffer_direct_slice() {
    update(WIFI_MAC_RX_RELOAD, |v| v & !0x2000_0000);
    update(WIFI_MAC_RX_CFG3, |v| v & !0x1000_0000);
    update(WIFI_MAC_RX_RELOAD, |v| v | 0x4000_0000);
}

unsafe fn enable_rftest_rx_accept_filter_direct_slice() {
    let mut index = 0;
    while index < 4 {
        let reg = WIFI_MAC_RX_POLICY_BASE.add(index);
        update(reg, |v| (v | 0x0000_000d) & !0x0000_0800);
        index += 1;
    }
}

unsafe fn init_rftest_rx_mac_filter_direct_slice() {
    WIFI_MAC_RX_ADDR0.write_volatile(0x0134_fe18);
    WIFI_MAC_RX_ADDR1.write_volatile(0x0504_0302);
    WIFI_MAC_RX_POLICY_A.write_volatile(0x0000_000f);
    WIFI_MAC_RX_POLICY_B.write_volatile(0x0000_000f);
    WIFI_MAC_RX_POLICY_C.write_volatile(0x0000_000f);
    WIFI_MAC_RX_ADDR_MASK0.write_volatile(0xffff_ffff);
    WIFI_MAC_RX_ADDR_MASK1.write_volatile(0x0001_ffff);
    update(WIFI_MAC_CTRL_33C78, |v| v | 1);
}

unsafe fn init_rftest_rx_per_filter_direct_slice() {
    WIFI_MAC_RX_ADDR0.write_volatile(0x0304_0506);
    WIFI_MAC_RX_ADDR1.write_volatile(0x0000_0102);
    WIFI_MAC_RX_POLICY_BASE
        .write_volatile((WIFI_MAC_RX_POLICY_BASE.read_volatile() & !0x01ff) | 0x0d);
    let mut index = 1;
    while index < 4 {
        let reg = WIFI_MAC_RX_POLICY_BASE.add(index);
        reg.write_volatile((reg.read_volatile() & !0x01ff) | 0x0f);
        index += 1;
    }
}

unsafe fn init_rftest_rx_2440m_opt_direct_slice() {
    update(WIFI_PHY_RX_2440M_CTRL, |v| (v & 0x00ff_ffff) | 0x4b00_0000);
    update(WIFI_PHY_RX_2440M_CTRL, |v| v | 0x0080_0000);
    update(WIFI_PHY_RX_2440M_CTRL, |v| (v & 0x00ff_ffff) | 0x3200_0000);
    update(WIFI_PHY_RX_2440M_CTRL, |v| v | 0x0080_0000);
    update(WIFI_PHY_RX_2440M_CTRL, |v| v & !0x0080_0000);
}

unsafe fn enable_rx_direct_slice() {
    update(WIFI_MAC_RX_RELOAD, |v| v | 0x8000_0000);
}

unsafe fn release_txrx_force_direct_slice() {
    update(WIFI_TXRX_CTRL, |v| v & 0xffff_f0ff);
}

unsafe fn init_mac_txrx_tail_slice() {
    update(WIFI_MAC_CTRL_33118, |v| (v & 0xf00f_ffff) | (27 << 20));
    update(WIFI_MAC_CTRL_33C78, |v| v | 3);
    update(WIFI_MAC_TX_CTRL_33C10, |v| {
        ((v & 0xffff_f000) | 0xf0) | 0xc000_0000
    });
    update(WIFI_MAC_TX_CTRL_33C14, |v| (v & 0xffff_f000) | 0xf0);
    update(WIFI_MAC_TX_CTRL_33C18, |v| (v & 0xffff_f000) | 0xf0);
    update(WIFI_MAC_TX_CTRL_33C94, |v| (v & !0xf0) | 0x40);
    update(WIFI_MAC_TX_CTRL_33C54, |v| v | 0xffff_0000);
    update(WIFI_MAC_TX_CTRL_33C88, |v| v & 0xf0ff_ffff);
    update(WIFI_MAC_CTRL_332B8, |v| v | 2);
    update(WIFI_MAC_CTRL_33084, |v| v & 0x7fff_ffff);
}

unsafe fn init_mac_rate_slice() {
    unsafe {
        WIFI_MAC_RATE_CTRL_33418.write_volatile(0);
        WIFI_MAC_RATE_CTRL_3340C.write_volatile(0x1919_1919);
        WIFI_MAC_RATE_CTRL_33410.write_volatile(0x0009_0a0b);
        WIFI_MAC_RATE_CTRL_33414.write_volatile(0x0005_0100);
        WIFI_MAC_RATE_CTRL_33404.write_volatile(0x0009_0a0b);
        WIFI_MAC_RATE_CTRL_33408.write_volatile(0x0005_0100);
    }
}

unsafe fn init_crypto_slice() {
    unsafe {
        WIFI_MAC_CRYPTO_CTRL_33800.write_volatile(0x0003_0000);
        WIFI_MAC_CRYPTO_CTRL_33804.write_volatile(0x0003_0000);
        WIFI_MAC_CRYPTO_CTRL_33808.write_volatile(0);
        WIFI_MAC_CRYPTO_CTRL_3380C.write_volatile(0);
        WIFI_MAC_CRYPTO_CTRL_33810.write_volatile(0);
    }
    update(WIFI_MAC_CRYPTO_CTRL_33840, |v| v | 25);
}

unsafe fn init_antenna_slice() {
    for index in 0..WIFI_MAC_ANT_CTRL_COUNT {
        let reg = unsafe { WIFI_MAC_ANT_CTRL_START.sub(WIFI_MAC_ANT_CTRL_STRIDE_WORDS * index) };
        update(reg, |v| ((v & !0x7) & !0x8 | 0x20) & !0x10);
    }
    update(WIFI_MAC_ANT_CTRL_332A8, |v| (v & !0x7) | 0x20);
}

unsafe fn init_phy_low_rate_slice() {
    update(WIFI_PHY_LOW_RATE_CTRL0, |v| (v & !0x400) & !0x800);
    update(WIFI_PHY_LOW_RATE_CTRL1, |v| v & !0x800);
}

unsafe fn set_phy_tx_seed(seed: u32) {
    update(WIFI_PHY_TX_SEED, |v| (v & !0x7f) | (seed & 0x7f));
}

unsafe fn init_phy_rx_11b_opt_slice() {
    update(WIFI_PHY_RX_11B_CTRL0, |v| v | 0x003f_0000);
    update(WIFI_PHY_RX_11B_CTRL0, |v| (v & 0xffff_c0ff) | 0x2100);
    update(WIFI_PHY_RX_11B_CTRL1, |v| (v & 0xffff_03ff) | 0x8400);
    update(WIFI_PHY_RX_11B_CTRL1, |v| (v & !0xf) | 3);
    update(WIFI_PHY_RX_11B_CTRL2, |v| (v & 0xffff_0fff) | 0x9000);
    update(WIFI_PHY_RX_11B_CTRL3, |v| (v & 0xffff_fe00) | 0x1e2);
    update(WIFI_MODEM_CTRL_26010, |v| v | 0x0002_0000);
}

unsafe fn init_phy_bb_reg_slice() {
    unsafe {
        WIFI_PHY_BB_CTRL_1CC48.write_volatile(0x1704_33af);
    }
    update(WIFI_PHY_TX_SEED, |v| v | 0x6000);
}

unsafe fn set_phy_wifi_enable(enable: bool) {
    if enable {
        update(WIFI_MODEM_WIFI_ENABLE, |v| v | 2);
    } else {
        update(WIFI_MODEM_WIFI_ENABLE, |v| v & !2);
    }
}

unsafe fn init_pbus_mem_slice() {
    let pbus_bank0_low = [0x0007_09ff, 0x0017_13ff, 0x00f5_0000, 0x00f6_0000];
    let pbus_bank0_high = [0x0004_01ff, 0x0018_01ff, 0x0014_01ff];
    let pbus_bank1_low = [
        0x0004_03ff,
        0x0014_f9ff,
        0x0018_01ff,
        0x0048_01ff,
        0x00f0_0000,
        0x00f1_0000,
        0x00f2_0000,
        0x00f4_0000,
    ];
    let pbus_bank1_high = [0x0014_f9ff, 0x0044_ffff, 0x00f3_0000];
    let pbus_bank2_low = [0x0014_f9ff, 0x0044_ffff, 0x00f3_0000];
    let pbus_bank2_high = [0x0044_01ff, 0x0054_01ff];
    let pbus_bank3_low = [
        pbus_bank0_low[0],
        0x0017_17ff,
        pbus_bank0_low[2],
        pbus_bank0_low[3],
    ];
    let pbus_bank3_high = [0x0004_01ff, 0x0018_01ff, 0x0014_01ff];
    let pbus_bank4_low = pbus_bank1_low;
    let pbus_bank4_high = [0x0004_01ff, 0x0018_01ff, 0x0014_01ff];
    let pbus_bank5_low = [0x0014_fdff, 0x0044_ffff, 0x00f3_0000];
    let pbus_bank5_high = [0x0044_01ff, 0x0054_01ff];

    write_pbus_window(WIFI_PBUS_BANK0, 0, 0, &pbus_bank0_low);
    write_pbus_window(WIFI_PBUS_BANK0, 4, 16, &pbus_bank0_high);
    write_pbus_window(WIFI_PBUS_BANK1, 0, 0, &pbus_bank1_low);
    write_pbus_window(WIFI_PBUS_BANK1, 8, 16, &pbus_bank1_high);
    write_pbus_window(WIFI_PBUS_BANK2, 0, 0, &pbus_bank2_low);
    write_pbus_window(WIFI_PBUS_BANK2, 3, 16, &pbus_bank2_high);
    write_pbus_window(WIFI_PBUS_BANK3, 0, 0, &pbus_bank3_low);
    write_pbus_window(WIFI_PBUS_BANK3, 4, 16, &pbus_bank3_high);
    write_pbus_window(WIFI_PBUS_BANK4, 0, 0, &pbus_bank4_low);
    write_pbus_window(WIFI_PBUS_BANK4, 8, 16, &pbus_bank4_high);
    write_pbus_window(WIFI_PBUS_BANK5, 0, 0, &pbus_bank5_low);
    write_pbus_window(WIFI_PBUS_BANK5, 3, 16, &pbus_bank5_high);
}

unsafe fn write_pbus_window(bank: *mut u32, start: u32, shift: u32, values: &[u32]) {
    let end = start + values.len() as u32;
    let selector = (((end - 1) << 8) | start) << shift;
    let mask = !(0x0000_ffffu32 << shift);
    update(bank, |v| (v & mask) | selector);

    let mut addr = start;
    for value in values {
        unsafe {
            WIFI_PBUS_DATA.write_volatile(*value);
        }
        update(WIFI_PBUS_ADDR_CTRL, |v| {
            (v & 0xfffc_00ff) | ((addr & 0x3ff) << 8)
        });
        update(WIFI_PBUS_ADDR_CTRL, |v| v & 0xfffc_ffff);
        addr += 1;
    }
}

unsafe fn init_rftest_rx_pbus_direct_slice(rx0: u16, rx1: u16, rx2: u16, rx3: u16) {
    let low = [
        ((rx0 as u32) << 9) | 0x0004_01ff,
        ((rx1 as u32) << 9) | 0x0014_01ff,
        0x00f5_0000,
        0x00f6_0000,
    ];
    let high = [
        ((rx2 as u32) << 9) | 0x0004_01ff,
        ((rx3 as u32) << 9) | 0x0014_01ff,
    ];

    write_pbus_window(WIFI_PBUS_BANK0, 0, 0, &low);
    write_pbus_window(WIFI_PBUS_BANK0, 4, 16, &high);
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn call_phy_rx_start_slot_direct_slice() -> bool {
    type PhyRxStartSlot = unsafe extern "C" fn(u32);

    let table = unsafe { phy_get_romfuncs() };
    if table.is_null() {
        return false;
    }

    let slot = unsafe { table.cast::<u8>().add(0x88).cast::<usize>().read_volatile() };
    if slot == 0 {
        return false;
    }

    let func: PhyRxStartSlot = unsafe { core::mem::transmute(slot) };
    unsafe {
        func(1);
    }
    true
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn call_phy_rx_start_slot_direct_slice() -> bool {
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn call_phy_pbus_debug_slot_direct_slice() -> bool {
    type PbusSlot = unsafe extern "C" fn(u32, u32, u32);

    let table = unsafe { phy_get_romfuncs() };
    if table.is_null() {
        return false;
    }

    let slot = unsafe {
        table
            .cast::<u8>()
            .add(0x1a8)
            .cast::<usize>()
            .read_volatile()
    };
    if slot == 0 {
        return false;
    }

    let func: PbusSlot = unsafe { core::mem::transmute(slot) };
    unsafe {
        func(0, 1, 1);
        func(1, 1, 124);
        func(1, 1, 126);
        func(1, 2, 0);
        func(2, 1, 0x100);
        func(3, 1, 0x100);
        func(2, 2, 0x100);
        func(3, 2, 0x100);
        func(4, 1, 127);
        func(5, 1, 15);
    }
    true
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn call_phy_pbus_debug_slot_direct_slice() -> bool {
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn call_phy_rftest_channel6_slot_path() -> bool {
    type Slot0 = unsafe extern "C" fn();
    type Slot1 = unsafe extern "C" fn(u32);
    type Slot2 = unsafe extern "C" fn(u32, u32);

    let table = unsafe { phy_get_romfuncs() };
    if table.is_null() {
        return false;
    }

    let save = read_phy_fun_slot(table, 0x160) as usize;
    let pre = read_phy_fun_slot(table, 0x008) as usize;
    let mode = read_phy_fun_slot(table, 0x06c) as usize;
    let rx_gain_prepare = read_phy_fun_slot(table, 0x24c) as usize;
    let rx_gain_channel = read_phy_fun_slot(table, 0x264) as usize;
    let post = read_phy_fun_slot(table, 0x00c) as usize;
    let restore = read_phy_fun_slot(table, 0x164) as usize;

    if save == 0
        || pre == 0
        || mode == 0
        || rx_gain_prepare == 0
        || rx_gain_channel == 0
        || post == 0
        || restore == 0
    {
        return false;
    }

    let save: Slot0 = core::mem::transmute(save);
    let pre: Slot0 = core::mem::transmute(pre);
    let mode: Slot1 = core::mem::transmute(mode);
    let rx_gain_prepare: Slot1 = core::mem::transmute(rx_gain_prepare);
    let rx_gain_channel: Slot2 = core::mem::transmute(rx_gain_channel);
    let post: Slot0 = core::mem::transmute(post);
    let restore: Slot1 = core::mem::transmute(restore);

    save();
    pre();
    force_txrx_off_slice(true);
    set_chan_freq_hw_init_direct_slice(6, 0);
    set_chan_freq_sw_start_direct_slice(6);
    mode(0);
    rx_gain_prepare(1);
    rx_gain_channel(6, 0);
    force_txrx_off_slice(false);
    post();
    restore(0);
    true
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn call_phy_rftest_channel6_slot_path() -> bool {
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn call_phy_slot0(offset: usize) -> bool {
    type Slot = unsafe extern "C" fn();

    let table = unsafe { phy_get_romfuncs() };
    let slot = unsafe { read_phy_fun_slot(table, offset) } as usize;
    if slot == 0 {
        return false;
    }

    let func: Slot = unsafe { core::mem::transmute(slot) };
    unsafe { func() };
    true
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn call_phy_slot0(_offset: usize) -> bool {
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn call_phy_slot1(offset: usize, arg0: u32) -> bool {
    type Slot = unsafe extern "C" fn(u32);

    let table = unsafe { phy_get_romfuncs() };
    let slot = unsafe { read_phy_fun_slot(table, offset) } as usize;
    if slot == 0 {
        return false;
    }

    let func: Slot = unsafe { core::mem::transmute(slot) };
    unsafe { func(arg0) };
    true
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn call_phy_slot1(_offset: usize, _arg0: u32) -> bool {
    false
}

#[cfg(all(target_arch = "xtensa", target_os = "none"))]
unsafe fn call_phy_slot2(offset: usize, arg0: u32, arg1: u32) -> bool {
    type Slot = unsafe extern "C" fn(u32, u32);

    let table = unsafe { phy_get_romfuncs() };
    let slot = unsafe { read_phy_fun_slot(table, offset) } as usize;
    if slot == 0 {
        return false;
    }

    let func: Slot = unsafe { core::mem::transmute(slot) };
    unsafe { func(arg0, arg1) };
    true
}

#[cfg(not(all(target_arch = "xtensa", target_os = "none")))]
unsafe fn call_phy_slot2(_offset: usize, _arg0: u32, _arg1: u32) -> bool {
    false
}

unsafe fn write_txrate_power_offset_slice() {
    let offsets = [
        0x0000_0000,
        0x1111_1111,
        0x0000_0000,
        0x1111_1111,
        0x4444_5555,
        0x2222_3333,
        0x4444_5555,
        0x2222_3333,
        0xa666_a666,
        0xb777_b777,
        0xc888_c888,
        0xd999_d999,
        0x7654_3210,
        0xfedc_ba98,
        0x7654_3210,
        0xfedc_ba98,
    ];

    let mut reg = WIFI_TXRATE_POWER0;
    for value in offsets {
        unsafe {
            reg.write_volatile(value);
            reg = reg.add(1);
        }
    }
}

unsafe fn open_i2c_xpd_slice() {
    update(WIFI_I2C_XPD_CTRL0, |v| v | 0xf800_0000);
    update(WIFI_I2C_XPD_CTRL1, |v| v | 0x0000_0080);
}

unsafe fn rf_init_direct_slice() {
    update(WIFI_RF_CTRL, |v| v & !0x0002_0000);
    update(WIFI_RF_CTRL, |v| v | 0x0002_0000);
    update(WIFI_TXRX_CTRL, |v| v & !0x0000_0300);
}

unsafe fn force_txrx_off_slice(force_rx: bool) {
    let first = if force_rx { 0x0000_0800 } else { 0x0000_0200 };
    let second = if force_rx { 0x0000_0a00 } else { 0x0000_0200 };

    update(WIFI_TXRX_CTRL, |v| (v & 0xffff_f0ff) | first);
    delay_approx_us(1);
    update(WIFI_TXRX_CTRL, |v| (v & 0xffff_f0ff) | second);
    delay_approx_us(1);
}

unsafe fn set_chan_freq_hw_init_direct_slice(channel: u8, mode: u8) {
    let channel = (channel as u32) & 0x0f;
    let mode = (mode as u32) & 0x0f;

    freq_i2c_data_write_direct_slice();
    update(WIFI_FREQ_MODE_CTRL, |v| (v & 0x0000_ffff) | 0x0c80_0000);
    update(WIFI_FREQ_HW_CTRL, |v| (v & 0xfff0_ffff) | (channel << 16));
    update(WIFI_FREQ_HW_CTRL, |v| (v & 0xff0f_ffff) | (mode << 20));
    update(WIFI_FREQ_HW_CTRL, |v| v | 0x0100_0000);
    update(WIFI_FREQ_HW_CTRL, |v| v | 0x4000_0000);
    update(WIFI_FREQ_HW_CTRL, |v| v & !0x2000_0000);
}

unsafe fn set_chan_freq_sw_start_direct_slice(channel: u8) {
    let channel = channel & 0x7f;

    update(WIFI_FREQ_HW_CTRL, |v| v & !0x0000_0100);
    update(WIFI_FREQ_SW_CTRL, |v| {
        (v & 0xf00f_ffff) | ((channel as u32) << 20)
    });
    update(WIFI_FREQ_HW_CTRL, |v| {
        (v & 0xffff_ff00) | (((channel as u32) << 1) & 0xff)
    });

    for _ in 0..3 {
        wait_freq_not_busy();
        update(WIFI_FREQ_HW_CTRL, |v| v | 0x0000_0100);
        update(WIFI_FREQ_HW_CTRL, |v| v & !0x0000_0100);
        delay_approx_us(1);
        wait_freq_not_busy();

        let status = unsafe { WIFI_FREQ_STATUS.read_volatile() };
        if ((status >> 17) & 0x7f) == channel as u32 {
            break;
        }
    }
}

unsafe fn pll_cap_mem_update_direct_slice(offset: i16) {
    let mut index = 0u32;
    for _ in 0..85 {
        update(WIFI_FREQ_HW_CTRL, |v| (v & 0xffff_ff00) | (index & 0xff));

        let cap = unsafe { WIFI_FREQ_PLL_CAP.read_volatile() };
        let raw = (((cap >> 12) & 0x1) << 8) | (cap & 0xff);
        let adjusted = raw.wrapping_add(offset as i32 as u32) & 0xffff;
        let adjusted_high = ((adjusted as i16 as i32) >> 8) as u32;
        let encoded = (cap & 0x0000_ef00) | (adjusted & 0xff) | (adjusted_high << 12);

        WIFI_FREQ_MEM_CTRL.write_volatile(encoded);
        update(WIFI_FREQ_HW_CTRL, |v| v | 0x0000_0200);
        update(WIFI_FREQ_HW_CTRL, |v| v & !0x0000_0200);

        index = index.wrapping_add(3);
    }
}

unsafe fn phy_en_hw_set_freq_direct_slice() {
    update(WIFI_FREQ_HW_CTRL, |v| v & !0x0200_0000);
}

unsafe fn phy_dis_hw_set_freq_direct_slice() {
    update(WIFI_FREQ_HW_CTRL, |v| v | 0x0200_0000);
    delay_approx_us(2);
}

unsafe fn freq_i2c_data_write_direct_slice() {
    let enable = [1, 1, 1, 1, 1, 1, 1, 1, 1, 0];
    let host = [99, 98, 98, 99, 99, 99, 99, 99, 98, 103];
    let addr = [0, 1, 2, 0, 3, 5, 4, 0, 11, 3];
    let low_b = [15, 16, 17, 0, 22, 20, 21, 1, 2, 3];
    let data_b = [0, 0, 0, 0, 0, 0, 0, 0x10, 6, 0xf0];
    let low_a = [15, 16, 17, 0, 22, 20, 21, 1, 2, 4];
    let data_a = [0, 0, 0, 0, 0, 0, 0, 0x10, 6, 0xf4];
    let flags = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    freq_i2c_write_set_direct_slice(
        &enable, &host, &addr, &low_b, &data_b, &low_a, &data_a, &flags,
    );
}

unsafe fn freq_i2c_write_set_direct_slice(
    enable: &[u8; 10],
    host: &[u8; 10],
    addr: &[u8; 10],
    low_b: &[u8; 10],
    data_b: &[u8; 10],
    low_a: &[u8; 10],
    data_a: &[u8; 10],
    flags: &[u8; 10],
) {
    const COUNT: usize = 10;

    update(WIFI_FREQ_HW_CTRL, |v| {
        (v & 0xffff_83ff) | ((COUNT as u32 & 0x1f) << 10)
    });

    let mut enable_mask = 0u32;
    for index in 0..COUNT {
        if low_a[index] == 1 {
            enable_mask |= 1u32 << index;
        }
    }
    WIFI_FREQ_I2C_ADDR_CTRL.write_volatile(enable_mask);

    for index in 0..COUNT {
        let reg = match index / 8 {
            0 => WIFI_FREQ_I2C_NIB0,
            1 => WIFI_FREQ_I2C_NIB1,
            _ => WIFI_FREQ_I2C_NIB2,
        };
        set_field(reg, ((index % 8) * 4) as u32, 4, enable[index] as u32);
    }

    for index in 0..COUNT {
        let pair = ((addr[index] as u32) << 8) | host[index] as u32;
        let reg = match index / 2 {
            0 => WIFI_FREQ_I2C_PAIR0,
            1 => WIFI_FREQ_I2C_PAIR1,
            2 => WIFI_FREQ_I2C_PAIR2,
            3 => WIFI_FREQ_I2C_PAIR3,
            4 => WIFI_FREQ_I2C_PAIR4,
            5 => WIFI_FREQ_I2C_PAIR5,
            6 => WIFI_FREQ_I2C_PAIR6,
            7 => WIFI_FREQ_I2C_PAIR7,
            8 => WIFI_FREQ_I2C_PAIR8,
            _ => WIFI_FREQ_I2C_PAIR9,
        };
        set_field(reg, ((index % 2) * 16) as u32, 16, pair);
    }

    for index in 0..COUNT {
        set_field(
            WIFI_FREQ_I2C_HI_A,
            index as u32,
            1,
            (low_a[index] >> 4) as u32,
        );
        set_field(
            WIFI_FREQ_I2C_HI_B,
            index as u32,
            1,
            (low_b[index] >> 4) as u32,
        );

        let low_a_reg = match index / 8 {
            0 => WIFI_FREQ_I2C_LOW_A0,
            1 => WIFI_FREQ_I2C_LOW_A1,
            _ => WIFI_FREQ_I2C_LOW_A2,
        };
        let low_b_reg = match index / 8 {
            0 => WIFI_FREQ_I2C_LOW_B0,
            _ => WIFI_FREQ_I2C_LOW_B1,
        };
        set_field(low_a_reg, ((index % 8) * 4) as u32, 4, low_a[index] as u32);
        set_field(low_b_reg, ((index % 8) * 4) as u32, 4, low_b[index] as u32);
    }

    for index in 0..COUNT {
        set_i2c_byte_field(low_a[index], data_a[index]);
        set_i2c_byte_field(low_b[index], data_b[index]);
        if flags[index] != 0 {
            set_i2c_byte_field(flags[index], flags[index]);
        }
    }
}

unsafe fn set_i2c_byte_field(selector: u8, value: u8) {
    let reg = match selector >> 2 {
        0 => WIFI_FREQ_I2C_BYTE0,
        1 => WIFI_FREQ_I2C_BYTE1,
        2 => WIFI_FREQ_I2C_BYTE2,
        3 => WIFI_FREQ_I2C_BYTE3,
        _ => return,
    };
    set_field(reg, ((selector & 0x03) as u32) * 8, 8, value as u32);
}

fn wait_freq_not_busy() {
    for _ in 0..100_000 {
        let busy = unsafe { WIFI_FREQ_BUSY.read_volatile() as i32 };
        if busy >= 0 {
            break;
        }
        core::hint::spin_loop();
    }
}

fn delay_approx_us(us: u32) {
    for _ in 0..us.saturating_mul(320) {
        core::hint::spin_loop();
    }
}

unsafe fn set_field(reg: *mut u32, shift: u32, width: u32, value: u32) {
    let mask = if width == 32 {
        u32::MAX
    } else {
        ((1u32 << width) - 1) << shift
    };
    update(reg, |v| (v & !mask) | ((value << shift) & mask));
}

unsafe fn update(reg: *mut u32, f: impl FnOnce(u32) -> u32) {
    let value = unsafe { reg.read_volatile() };
    unsafe { reg.write_volatile(f(value)) };
}
