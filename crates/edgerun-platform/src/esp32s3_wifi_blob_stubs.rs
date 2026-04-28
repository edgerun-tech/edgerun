//! Minimal ESP-IDF symbol shims for the quarantined ESP32-S3 Wi-Fi blob path.
//!
//! These are intentionally limited to logging/event symbols that the binary
//! blobs reference but Edgerun does not need for the raw 802.11 experiment.

use core::ffi::{c_char, c_int, c_void};

#[no_mangle]
pub static WIFI_EVENT: EventBase = EventBase(WIFI_EVENT_NAME.as_ptr().cast::<c_char>());

#[used]
static WIFI_EVENT_NAME: [u8; 11] = *b"WIFI_EVENT\0";

#[repr(transparent)]
pub struct EventBase(*const c_char);

unsafe impl Sync for EventBase {}

#[no_mangle]
pub static mut g_log_level: u32 = 0;

#[no_mangle]
pub extern "C" fn phy_printf(_format: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn net80211_printf(_format: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn pp_printf(_format: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn coexist_printf(_format: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn rtc_printf(_format: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn syslog(_priority: c_int, _format: *const c_char) {}

#[no_mangle]
pub extern "C" fn esp_event_post(
    _event_base: *const c_char,
    _event_id: i32,
    _event_data: *mut c_void,
    _event_data_size: usize,
    _ticks_to_wait: u32,
) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn esp_log_level_set(_tag: *const c_char, _level: u32) {}
