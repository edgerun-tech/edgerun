//! Minimal ESP-IDF symbol shims for the quarantined ESP32-S3 Wi-Fi blob path.
//!
//! These are intentionally limited to logging/event symbols that the binary
//! blobs reference but Edgerun does not need for the raw 802.11 experiment.

use core::ffi::{c_char, c_int, c_uchar, c_void};

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
pub static mut g_misc_nvs: [u8; 1] = [0];

#[no_mangle]
pub static mut g_espnow_user_oui: [u8; 3] = [0x18, 0xfe, 0x34];

#[no_mangle]
pub static regdomain_table: [u8; 1] = [0];

#[no_mangle]
pub static regulatory_data: [u8; 1] = [0];

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

#[no_mangle]
pub extern "C" fn misc_nvs_init() -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn misc_nvs_deinit() {}

#[no_mangle]
pub extern "C" fn puts(_s: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn sprintf(
    dst: *mut c_char,
    _format: *const c_char,
    _args: ...
) -> c_int {
    if !dst.is_null() {
        unsafe { dst.write(0) };
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut i = 0;
    loop {
        let byte = unsafe { src.add(i).read() };
        unsafe { dst.add(i).write(byte) };
        i += 1;
        if byte == 0 {
            break;
        }
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncpy(
    dst: *mut c_char,
    src: *const c_char,
    n: usize,
) -> *mut c_char {
    let mut i = 0;
    let mut done = false;
    while i < n {
        let byte = if done {
            0
        } else {
            let byte = unsafe { src.add(i).read() };
            done = byte == 0;
            byte
        };
        unsafe { dst.add(i).write(byte) };
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncmp(lhs: *const c_char, rhs: *const c_char, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let a = unsafe { lhs.add(i).read() as c_uchar };
        let b = unsafe { rhs.add(i).read() as c_uchar };
        if a != b {
            return a as c_int - b as c_int;
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strnlen(s: *const c_char, max: usize) -> usize {
    let mut len = 0;
    while len < max {
        if unsafe { s.add(len).read() } == 0 {
            break;
        }
        len += 1;
    }
    len
}

#[no_mangle]
pub unsafe extern "C" fn free(_p: *mut c_void) {}

#[no_mangle]
pub unsafe extern "C" fn hexstr2bin(hex: *const c_char, buf: *mut u8, len: usize) -> c_int {
    let mut i = 0;
    while i < len {
        let hi = unsafe { hex_nibble(hex.add(i * 2).read()) };
        let lo = unsafe { hex_nibble(hex.add(i * 2 + 1).read()) };
        if hi < 0 || lo < 0 {
            return -1;
        }
        unsafe { buf.add(i).write(((hi as u8) << 4) | lo as u8) };
        i += 1;
    }
    0
}

fn hex_nibble(byte: c_char) -> c_int {
    let byte = byte as u8;
    match byte {
        b'0'..=b'9' => (byte - b'0') as c_int,
        b'a'..=b'f' => (byte - b'a' + 10) as c_int,
        b'A'..=b'F' => (byte - b'A' + 10) as c_int,
        _ => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let mut i = 0;
    let dst_bytes = dst.cast::<u8>();
    let src_bytes = src.cast::<u8>();
    while i < n {
        unsafe {
            dst_bytes.add(i).write(src_bytes.add(i).read());
        }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memset(dst: *mut c_void, value: c_int, n: usize) -> *mut c_void {
    let mut i = 0;
    let dst_bytes = dst.cast::<u8>();
    while i < n {
        unsafe {
            dst_bytes.add(i).write(value as u8);
        }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(lhs: *const c_void, rhs: *const c_void, n: usize) -> c_int {
    let mut i = 0;
    let lhs_bytes = lhs.cast::<u8>();
    let rhs_bytes = rhs.cast::<u8>();
    while i < n {
        let a = unsafe { lhs_bytes.add(i).read() };
        let b = unsafe { rhs_bytes.add(i).read() };
        if a != b {
            return a as c_int - b as c_int;
        }
        i += 1;
    }
    0
}
