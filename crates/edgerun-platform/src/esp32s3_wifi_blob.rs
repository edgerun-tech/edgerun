//! Quarantined ESP32-S3 Wi-Fi blob adapter.
//!
//! This module is not enabled by default. It assumes an external Wi-Fi bring-up
//! path has initialized the Espressif driver enough for promiscuous RX and raw
//! 802.11 TX to work. The rest of Edgerun sees only `Esp32s3WifiRadio`.

use core::cell::UnsafeCell;
use core::ffi::{c_int, c_void};

use edgerun_wifi::ieee80211::OpenApConfig;

use crate::esp32s3_wifi::Esp32s3WifiRadio;

const ESP_OK: i32 = 0;
const WIFI_IF_AP: i32 = 1;
const WIFI_SECOND_CHAN_NONE: i32 = 0;
const WIFI_PKT_MGMT: i32 = 0;
const WIFI_PKT_DATA: i32 = 2;
const WIFI_PROMIS_FILTER_MASK_MGMT: u32 = 1;
const WIFI_PROMIS_FILTER_MASK_DATA: u32 = 1 << 2;

const RAW_80211_MAX: usize = 2352;
const RAW_RX_QUEUE: usize = 8;

// ESP32-S3 wifi_pkt_rx_ctrl_t is 48 bytes in IDF 5.5.x. sig_len is the low 12
// bits of the u32 at offset 44 and includes the FCS when present.
const ESP32S3_PROMISC_RX_CTRL_LEN: usize = 48;
const ESP32S3_PROMISC_SIG_LEN_OFFSET: usize = 44;
const IEEE80211_FCS_LEN: usize = 4;

#[repr(C)]
struct WifiPromiscuousFilter {
    filter_mask: u32,
}

unsafe extern "C" {
    fn esp_wifi_set_channel(primary: u8, second: c_int) -> i32;
    fn esp_wifi_set_promiscuous_rx_cb(cb: Option<extern "C" fn(*mut c_void, c_int)>) -> i32;
    fn esp_wifi_set_promiscuous(en: bool) -> i32;
    fn esp_wifi_set_promiscuous_filter(filter: *const WifiPromiscuousFilter) -> i32;
    fn esp_wifi_80211_tx(ifx: c_int, buffer: *const c_void, len: c_int, en_sys_seq: bool) -> i32;
}

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

impl EspressifPromiscRadio {
    pub const fn new() -> Self {
        Self
    }
}

impl Esp32s3WifiRadio for EspressifPromiscRadio {
    fn start_open_ap(&mut self, config: &OpenApConfig) -> bool {
        let filter = WifiPromiscuousFilter {
            filter_mask: WIFI_PROMIS_FILTER_MASK_MGMT | WIFI_PROMIS_FILTER_MASK_DATA,
        };
        unsafe {
            esp_wifi_set_channel(config.channel, WIFI_SECOND_CHAN_NONE) == ESP_OK
                && esp_wifi_set_promiscuous_filter(&filter) == ESP_OK
                && esp_wifi_set_promiscuous_rx_cb(Some(promisc_rx_cb)) == ESP_OK
                && esp_wifi_set_promiscuous(true) == ESP_OK
        }
    }

    fn send_raw_80211(&mut self, frame: &[u8]) -> bool {
        if frame.len() < 24 || frame.len() > 1500 {
            return false;
        }
        unsafe {
            esp_wifi_80211_tx(
                WIFI_IF_AP,
                frame.as_ptr().cast::<c_void>(),
                frame.len() as c_int,
                true,
            ) == ESP_OK
        }
    }

    fn recv_raw_80211(&mut self, out: &mut [u8]) -> Option<usize> {
        unsafe { RAW_RX.pop(out) }
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
