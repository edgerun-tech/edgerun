//! Minimal ESP32-S3 Wi-Fi blob initialization adapter.
//!
//! This is a bring-up shim, not the final runtime contract. It provides the
//! ESP-IDF Wi-Fi blob with enough OS callbacks to move initialization forward
//! while Edgerun's own scheduler-backed adapter is still being built.

use alloc::alloc::{alloc, alloc_zeroed, Layout};
use core::ffi::{c_char, c_int, c_uint, c_ulong, c_void};
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

const ESP_OK: i32 = 0;
const WIFI_INIT_CONFIG_MAGIC: i32 = 0x1f2f3f4f;
const ESP_WIFI_OS_ADAPTER_VERSION: i32 = 0x0000_0008;
const ESP_WIFI_OS_ADAPTER_MAGIC: i32 = 0xdead_beafu32 as i32;
const WIFI_MODE_NULL: i32 = 0;
const WIFI_MODE_AP: i32 = 2;

static INIT_STATE: AtomicU32 = AtomicU32::new(0);
static RAND_STATE: AtomicU32 = AtomicU32::new(0x1234_abcd);
static TIME_US: AtomicU32 = AtomicU32::new(0);

#[repr(C)]
#[derive(Clone, Copy)]
struct WpaCryptoFuncs {
    size: u32,
    version: u32,
    hmac_sha256_vector: Option<unsafe extern "C" fn()>,
    pbkdf2_sha1: Option<unsafe extern "C" fn()>,
    aes_128_encrypt: Option<unsafe extern "C" fn()>,
    aes_128_decrypt: Option<unsafe extern "C" fn()>,
    omac1_aes_128: Option<unsafe extern "C" fn()>,
    ccmp_decrypt: Option<unsafe extern "C" fn()>,
    ccmp_encrypt: Option<unsafe extern "C" fn()>,
    aes_gmac: Option<unsafe extern "C" fn()>,
    sha256_vector: Option<unsafe extern "C" fn()>,
}

#[repr(C)]
struct WifiInitConfig {
    osi_funcs: *mut WifiOsiFuncs,
    wpa_crypto_funcs: WpaCryptoFuncs,
    static_rx_buf_num: c_int,
    dynamic_rx_buf_num: c_int,
    tx_buf_type: c_int,
    static_tx_buf_num: c_int,
    dynamic_tx_buf_num: c_int,
    rx_mgmt_buf_type: c_int,
    rx_mgmt_buf_num: c_int,
    cache_tx_buf_num: c_int,
    csi_enable: c_int,
    ampdu_rx_enable: c_int,
    ampdu_tx_enable: c_int,
    amsdu_tx_enable: c_int,
    nvs_enable: c_int,
    nano_enable: c_int,
    rx_ba_win: c_int,
    wifi_task_core_id: c_int,
    beacon_max_len: c_int,
    mgmt_sbuf_num: c_int,
    feature_caps: u64,
    sta_disconnected_pm: bool,
    espnow_max_encrypt_num: c_int,
    tx_hetb_queue_num: c_int,
    dump_hesigb_enable: bool,
    magic: c_int,
}

#[repr(C)]
struct WifiOsiFuncs {
    _version: i32,
    _env_is_chip: Option<unsafe extern "C" fn() -> bool>,
    _set_intr: Option<unsafe extern "C" fn(i32, u32, u32, i32)>,
    _clear_intr: Option<unsafe extern "C" fn(u32, u32)>,
    _set_isr: Option<unsafe extern "C" fn(i32, *mut c_void, *mut c_void)>,
    _ints_on: Option<unsafe extern "C" fn(u32)>,
    _ints_off: Option<unsafe extern "C" fn(u32)>,
    _is_from_isr: Option<unsafe extern "C" fn() -> bool>,
    _spin_lock_create: Option<unsafe extern "C" fn() -> *mut c_void>,
    _spin_lock_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    _wifi_int_disable: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
    _wifi_int_restore: Option<unsafe extern "C" fn(*mut c_void, u32)>,
    _task_yield_from_isr: Option<unsafe extern "C" fn()>,
    _semphr_create: Option<unsafe extern "C" fn(u32, u32) -> *mut c_void>,
    _semphr_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    _semphr_take: Option<unsafe extern "C" fn(*mut c_void, u32) -> i32>,
    _semphr_give: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    _wifi_thread_semphr_get: Option<unsafe extern "C" fn() -> *mut c_void>,
    _mutex_create: Option<unsafe extern "C" fn() -> *mut c_void>,
    _recursive_mutex_create: Option<unsafe extern "C" fn() -> *mut c_void>,
    _mutex_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    _mutex_lock: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    _mutex_unlock: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    _queue_create: Option<unsafe extern "C" fn(u32, u32) -> *mut c_void>,
    _queue_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    _queue_send: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    _queue_send_from_isr: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32>,
    _queue_send_to_back: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    _queue_send_to_front: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    _queue_recv: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    _queue_msg_waiting: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
    _event_group_create: Option<unsafe extern "C" fn() -> *mut c_void>,
    _event_group_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    _event_group_set_bits: Option<unsafe extern "C" fn(*mut c_void, u32) -> u32>,
    _event_group_clear_bits: Option<unsafe extern "C" fn(*mut c_void, u32) -> u32>,
    _event_group_wait_bits: Option<unsafe extern "C" fn(*mut c_void, u32, c_int, c_int, u32) -> u32>,
    _task_create_pinned_to_core: Option<unsafe extern "C" fn(*mut c_void, *const c_char, u32, *mut c_void, u32, *mut c_void, u32) -> i32>,
    _task_create: Option<unsafe extern "C" fn(*mut c_void, *const c_char, u32, *mut c_void, u32, *mut c_void) -> i32>,
    _task_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    _task_delay: Option<unsafe extern "C" fn(u32)>,
    _task_ms_to_tick: Option<unsafe extern "C" fn(u32) -> i32>,
    _task_get_current_task: Option<unsafe extern "C" fn() -> *mut c_void>,
    _task_get_max_priority: Option<unsafe extern "C" fn() -> i32>,
    _malloc: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    _free: Option<unsafe extern "C" fn(*mut c_void)>,
    _event_post: Option<unsafe extern "C" fn(*const c_char, i32, *mut c_void, usize, u32) -> i32>,
    _get_free_heap_size: Option<unsafe extern "C" fn() -> u32>,
    _rand: Option<unsafe extern "C" fn() -> u32>,
    _dport_access_stall_other_cpu_start_wrap: Option<unsafe extern "C" fn()>,
    _dport_access_stall_other_cpu_end_wrap: Option<unsafe extern "C" fn()>,
    _wifi_apb80m_request: Option<unsafe extern "C" fn()>,
    _wifi_apb80m_release: Option<unsafe extern "C" fn()>,
    _phy_disable: Option<unsafe extern "C" fn()>,
    _phy_enable: Option<unsafe extern "C" fn()>,
    _phy_update_country_info: Option<unsafe extern "C" fn(*const c_char) -> c_int>,
    _read_mac: Option<unsafe extern "C" fn(*mut u8, c_uint) -> c_int>,
    _timer_arm: Option<unsafe extern "C" fn(*mut c_void, u32, bool)>,
    _timer_disarm: Option<unsafe extern "C" fn(*mut c_void)>,
    _timer_done: Option<unsafe extern "C" fn(*mut c_void)>,
    _timer_setfn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void)>,
    _timer_arm_us: Option<unsafe extern "C" fn(*mut c_void, u32, bool)>,
    _wifi_reset_mac: Option<unsafe extern "C" fn()>,
    _wifi_clock_enable: Option<unsafe extern "C" fn()>,
    _wifi_clock_disable: Option<unsafe extern "C" fn()>,
    _wifi_rtc_enable_iso: Option<unsafe extern "C" fn()>,
    _wifi_rtc_disable_iso: Option<unsafe extern "C" fn()>,
    _esp_timer_get_time: Option<unsafe extern "C" fn() -> i64>,
    _nvs_set_i8: Option<unsafe extern "C" fn(u32, *const c_char, i8) -> c_int>,
    _nvs_get_i8: Option<unsafe extern "C" fn(u32, *const c_char, *mut i8) -> c_int>,
    _nvs_set_u8: Option<unsafe extern "C" fn(u32, *const c_char, u8) -> c_int>,
    _nvs_get_u8: Option<unsafe extern "C" fn(u32, *const c_char, *mut u8) -> c_int>,
    _nvs_set_u16: Option<unsafe extern "C" fn(u32, *const c_char, u16) -> c_int>,
    _nvs_get_u16: Option<unsafe extern "C" fn(u32, *const c_char, *mut u16) -> c_int>,
    _nvs_open: Option<unsafe extern "C" fn(*const c_char, c_uint, *mut u32) -> c_int>,
    _nvs_close: Option<unsafe extern "C" fn(u32)>,
    _nvs_commit: Option<unsafe extern "C" fn(u32) -> c_int>,
    _nvs_set_blob: Option<unsafe extern "C" fn(u32, *const c_char, *const c_void, usize) -> c_int>,
    _nvs_get_blob: Option<unsafe extern "C" fn(u32, *const c_char, *mut c_void, *mut usize) -> c_int>,
    _nvs_erase_key: Option<unsafe extern "C" fn(u32, *const c_char) -> c_int>,
    _get_random: Option<unsafe extern "C" fn(*mut u8, usize) -> c_int>,
    _get_time: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
    _random: Option<unsafe extern "C" fn() -> c_ulong>,
    _slowclk_cal_get: Option<unsafe extern "C" fn() -> u32>,
    _log_write: Option<unsafe extern "C" fn()>,
    _log_writev: Option<unsafe extern "C" fn()>,
    _log_timestamp: Option<unsafe extern "C" fn() -> u32>,
    _malloc_internal: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    _realloc_internal: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    _calloc_internal: Option<unsafe extern "C" fn(usize, usize) -> *mut c_void>,
    _zalloc_internal: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    _wifi_malloc: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    _wifi_realloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    _wifi_calloc: Option<unsafe extern "C" fn(usize, usize) -> *mut c_void>,
    _wifi_zalloc: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    _wifi_create_queue: Option<unsafe extern "C" fn(c_int, c_int) -> *mut c_void>,
    _wifi_delete_queue: Option<unsafe extern "C" fn(*mut c_void)>,
    _coex_init: Option<unsafe extern "C" fn() -> c_int>,
    _coex_deinit: Option<unsafe extern "C" fn()>,
    _coex_enable: Option<unsafe extern "C" fn() -> c_int>,
    _coex_disable: Option<unsafe extern "C" fn()>,
    _coex_status_get: Option<unsafe extern "C" fn() -> u32>,
    _coex_condition_set: Option<unsafe extern "C" fn(u32, bool)>,
    _coex_wifi_request: Option<unsafe extern "C" fn(u32, u32, u32) -> c_int>,
    _coex_wifi_release: Option<unsafe extern "C" fn(u32) -> c_int>,
    _coex_wifi_channel_set: Option<unsafe extern "C" fn(u8, u8) -> c_int>,
    _coex_event_duration_get: Option<unsafe extern "C" fn(u32, *mut u32) -> c_int>,
    _coex_pti_get: Option<unsafe extern "C" fn(u32, *mut u8) -> c_int>,
    _coex_schm_status_bit_clear: Option<unsafe extern "C" fn(u32, u32)>,
    _coex_schm_status_bit_set: Option<unsafe extern "C" fn(u32, u32)>,
    _coex_schm_interval_set: Option<unsafe extern "C" fn(u32) -> c_int>,
    _coex_schm_interval_get: Option<unsafe extern "C" fn() -> u32>,
    _coex_schm_curr_period_get: Option<unsafe extern "C" fn() -> u8>,
    _coex_schm_curr_phase_get: Option<unsafe extern "C" fn() -> *mut c_void>,
    _coex_schm_process_restart: Option<unsafe extern "C" fn() -> c_int>,
    _coex_schm_register_cb: Option<unsafe extern "C" fn(c_int, Option<unsafe extern "C" fn(c_int) -> c_int>) -> c_int>,
    _coex_register_start_cb: Option<unsafe extern "C" fn(Option<unsafe extern "C" fn() -> c_int>) -> c_int>,
    _coex_schm_flexible_period_set: Option<unsafe extern "C" fn(u8) -> c_int>,
    _coex_schm_flexible_period_get: Option<unsafe extern "C" fn() -> u8>,
    _coex_schm_get_phase_by_idx: Option<unsafe extern "C" fn(c_int) -> *mut c_void>,
    _magic: i32,
}

unsafe impl Sync for WifiOsiFuncs {}

unsafe extern "C" {
    fn esp_wifi_init_internal(config: *const WifiInitConfig) -> i32;
    fn esp_wifi_set_mode(mode: c_int) -> i32;
    fn esp_wifi_start() -> i32;
}

pub fn ensure_started_ap() -> i32 {
    match INIT_STATE.load(Ordering::Relaxed) {
        3 => return ESP_OK,
        _ => {}
    }
    unsafe {
        let status = esp_wifi_init_internal(ptr::addr_of!(WIFI_INIT_CONFIG));
        if status != ESP_OK && status != 0x3003 {
            return 10_000 + status;
        }
        INIT_STATE.store(1, Ordering::Relaxed);
        let status = esp_wifi_set_mode(WIFI_MODE_AP);
        if status != ESP_OK {
            return 20_000 + status;
        }
        INIT_STATE.store(2, Ordering::Relaxed);
        let status = esp_wifi_start();
        if status != ESP_OK {
            return 30_000 + status;
        }
        INIT_STATE.store(3, Ordering::Relaxed);
    }
    ESP_OK
}

static mut WIFI_INIT_CONFIG: WifiInitConfig = WifiInitConfig {
    osi_funcs: ptr::addr_of!(WIFI_OSI_FUNCS).cast_mut(),
    wpa_crypto_funcs: WpaCryptoFuncs {
        size: core::mem::size_of::<WpaCryptoFuncs>() as u32,
        version: 1,
        hmac_sha256_vector: None,
        pbkdf2_sha1: None,
        aes_128_encrypt: None,
        aes_128_decrypt: None,
        omac1_aes_128: None,
        ccmp_decrypt: None,
        ccmp_encrypt: None,
        aes_gmac: None,
        sha256_vector: None,
    },
    static_rx_buf_num: 10,
    dynamic_rx_buf_num: 32,
    tx_buf_type: 0,
    static_tx_buf_num: 16,
    dynamic_tx_buf_num: 0,
    rx_mgmt_buf_type: 0,
    rx_mgmt_buf_num: 5,
    cache_tx_buf_num: 32,
    csi_enable: 0,
    ampdu_rx_enable: 1,
    ampdu_tx_enable: 1,
    amsdu_tx_enable: 0,
    nvs_enable: 0,
    nano_enable: 0,
    rx_ba_win: 6,
    wifi_task_core_id: 0,
    beacon_max_len: 752,
    mgmt_sbuf_num: 32,
    feature_caps: 0,
    sta_disconnected_pm: false,
    espnow_max_encrypt_num: 7,
    tx_hetb_queue_num: 3,
    dump_hesigb_enable: false,
    magic: WIFI_INIT_CONFIG_MAGIC,
};

static WIFI_OSI_FUNCS: WifiOsiFuncs = WifiOsiFuncs {
    _version: ESP_WIFI_OS_ADAPTER_VERSION,
    _env_is_chip: Some(env_is_chip),
    _set_intr: Some(set_intr),
    _clear_intr: Some(clear_intr),
    _set_isr: Some(set_isr),
    _ints_on: Some(ints_on),
    _ints_off: Some(ints_off),
    _is_from_isr: Some(is_from_isr),
    _spin_lock_create: Some(dummy_alloc),
    _spin_lock_delete: Some(dummy_delete),
    _wifi_int_disable: Some(wifi_int_disable),
    _wifi_int_restore: Some(wifi_int_restore),
    _task_yield_from_isr: Some(noop0),
    _semphr_create: Some(semphr_create),
    _semphr_delete: Some(dummy_delete),
    _semphr_take: Some(ok_take),
    _semphr_give: Some(ok_ptr),
    _wifi_thread_semphr_get: Some(dummy_alloc),
    _mutex_create: Some(dummy_alloc),
    _recursive_mutex_create: Some(dummy_alloc),
    _mutex_delete: Some(dummy_delete),
    _mutex_lock: Some(ok_ptr),
    _mutex_unlock: Some(ok_ptr),
    _queue_create: Some(queue_create),
    _queue_delete: Some(dummy_delete),
    _queue_send: Some(queue_send),
    _queue_send_from_isr: Some(queue_send_from_isr),
    _queue_send_to_back: Some(queue_send),
    _queue_send_to_front: Some(queue_send),
    _queue_recv: Some(queue_recv),
    _queue_msg_waiting: Some(zero_ptr),
    _event_group_create: Some(dummy_alloc),
    _event_group_delete: Some(dummy_delete),
    _event_group_set_bits: Some(return_bits),
    _event_group_clear_bits: Some(return_zero_bits),
    _event_group_wait_bits: Some(wait_bits),
    _task_create_pinned_to_core: Some(task_create_pinned_to_core),
    _task_create: Some(task_create),
    _task_delete: Some(dummy_delete),
    _task_delay: Some(delay),
    _task_ms_to_tick: Some(ms_to_tick),
    _task_get_current_task: Some(dummy_alloc),
    _task_get_max_priority: Some(max_priority),
    _malloc: Some(os_malloc),
    _free: Some(os_free),
    _event_post: Some(event_post),
    _get_free_heap_size: Some(free_heap),
    _rand: Some(rand),
    _dport_access_stall_other_cpu_start_wrap: Some(noop0),
    _dport_access_stall_other_cpu_end_wrap: Some(noop0),
    _wifi_apb80m_request: Some(noop0),
    _wifi_apb80m_release: Some(noop0),
    _phy_disable: Some(noop0),
    _phy_enable: Some(noop0),
    _phy_update_country_info: Some(ok_country),
    _read_mac: Some(read_mac),
    _timer_arm: Some(timer_arm),
    _timer_disarm: Some(dummy_delete),
    _timer_done: Some(dummy_delete),
    _timer_setfn: Some(timer_setfn),
    _timer_arm_us: Some(timer_arm),
    _wifi_reset_mac: Some(noop0),
    _wifi_clock_enable: Some(noop0),
    _wifi_clock_disable: Some(noop0),
    _wifi_rtc_enable_iso: Some(noop0),
    _wifi_rtc_disable_iso: Some(noop0),
    _esp_timer_get_time: Some(timer_get_time),
    _nvs_set_i8: Some(nvs_set_i8),
    _nvs_get_i8: Some(nvs_get_i8),
    _nvs_set_u8: Some(nvs_set_u8),
    _nvs_get_u8: Some(nvs_get_u8),
    _nvs_set_u16: Some(nvs_set_u16),
    _nvs_get_u16: Some(nvs_get_u16),
    _nvs_open: Some(nvs_open),
    _nvs_close: Some(nvs_close),
    _nvs_commit: Some(nvs_commit),
    _nvs_set_blob: Some(nvs_set_blob),
    _nvs_get_blob: Some(nvs_get_blob),
    _nvs_erase_key: Some(nvs_erase_key),
    _get_random: Some(get_random),
    _get_time: Some(get_time),
    _random: Some(random),
    _slowclk_cal_get: Some(slowclk_cal_get),
    _log_write: None,
    _log_writev: None,
    _log_timestamp: Some(log_timestamp),
    _malloc_internal: Some(os_malloc),
    _realloc_internal: Some(os_realloc),
    _calloc_internal: Some(os_calloc),
    _zalloc_internal: Some(os_zalloc),
    _wifi_malloc: Some(os_malloc),
    _wifi_realloc: Some(os_realloc),
    _wifi_calloc: Some(os_calloc),
    _wifi_zalloc: Some(os_zalloc),
    _wifi_create_queue: Some(wifi_create_queue),
    _wifi_delete_queue: Some(dummy_delete),
    _coex_init: Some(ok0),
    _coex_deinit: Some(noop0),
    _coex_enable: Some(ok0),
    _coex_disable: Some(noop0),
    _coex_status_get: Some(zero0),
    _coex_condition_set: Some(coex_condition_set),
    _coex_wifi_request: Some(coex_wifi_request),
    _coex_wifi_release: Some(coex_wifi_release),
    _coex_wifi_channel_set: Some(coex_wifi_channel_set),
    _coex_event_duration_get: Some(coex_event_duration_get),
    _coex_pti_get: Some(coex_pti_get),
    _coex_schm_status_bit_clear: Some(coex_status_bit),
    _coex_schm_status_bit_set: Some(coex_status_bit),
    _coex_schm_interval_set: Some(coex_interval_set),
    _coex_schm_interval_get: Some(zero0),
    _coex_schm_curr_period_get: Some(zero_u8),
    _coex_schm_curr_phase_get: Some(null0),
    _coex_schm_process_restart: Some(ok0),
    _coex_schm_register_cb: Some(coex_register_cb),
    _coex_register_start_cb: Some(coex_register_start_cb),
    _coex_schm_flexible_period_set: Some(coex_flexible_period_set),
    _coex_schm_flexible_period_get: Some(zero_u8),
    _coex_schm_get_phase_by_idx: Some(coex_get_phase),
    _magic: ESP_WIFI_OS_ADAPTER_MAGIC,
};

unsafe extern "C" fn env_is_chip() -> bool { true }
unsafe extern "C" fn set_intr(_cpu: i32, _src: u32, _num: u32, _prio: i32) {}
unsafe extern "C" fn clear_intr(_src: u32, _num: u32) {}
unsafe extern "C" fn set_isr(_n: i32, _f: *mut c_void, _arg: *mut c_void) {}
unsafe extern "C" fn ints_on(_mask: u32) {}
unsafe extern "C" fn ints_off(_mask: u32) {}
unsafe extern "C" fn is_from_isr() -> bool { false }
unsafe extern "C" fn noop0() {}
unsafe extern "C" fn ok0() -> c_int { ESP_OK }
unsafe extern "C" fn zero0() -> u32 { 0 }
unsafe extern "C" fn zero_u8() -> u8 { 0 }
unsafe extern "C" fn null0() -> *mut c_void { ptr::null_mut() }
unsafe extern "C" fn ok_ptr(_p: *mut c_void) -> i32 { 1 }
unsafe extern "C" fn ok_take(_p: *mut c_void, _ticks: u32) -> i32 { 1 }
unsafe extern "C" fn zero_ptr(_p: *mut c_void) -> u32 { 0 }
unsafe extern "C" fn dummy_alloc() -> *mut c_void { 1usize as *mut c_void }
unsafe extern "C" fn dummy_delete(_p: *mut c_void) {}
unsafe extern "C" fn wifi_int_disable(_p: *mut c_void) -> u32 { 0 }
unsafe extern "C" fn wifi_int_restore(_p: *mut c_void, _state: u32) {}
unsafe extern "C" fn semphr_create(_max: u32, _init: u32) -> *mut c_void { 1usize as *mut c_void }
unsafe extern "C" fn queue_create(_len: u32, _item_size: u32) -> *mut c_void { 1usize as *mut c_void }
unsafe extern "C" fn wifi_create_queue(len: c_int, item_size: c_int) -> *mut c_void {
    unsafe { queue_create(len as u32, item_size as u32) }
}
unsafe extern "C" fn queue_send(_q: *mut c_void, _item: *mut c_void, _ticks: u32) -> i32 { 1 }
unsafe extern "C" fn queue_send_from_isr(_q: *mut c_void, _item: *mut c_void, _hptw: *mut c_void) -> i32 { 1 }
unsafe extern "C" fn queue_recv(_q: *mut c_void, _item: *mut c_void, _ticks: u32) -> i32 { 0 }
unsafe extern "C" fn return_bits(_event: *mut c_void, bits: u32) -> u32 { bits }
unsafe extern "C" fn return_zero_bits(_event: *mut c_void, _bits: u32) -> u32 { 0 }
unsafe extern "C" fn wait_bits(_event: *mut c_void, bits: u32, _clear: c_int, _all: c_int, _ticks: u32) -> u32 { bits }
unsafe extern "C" fn task_create_pinned_to_core(_task: *mut c_void, _name: *const c_char, _stack: u32, _param: *mut c_void, _prio: u32, handle: *mut c_void, _core: u32) -> i32 {
    if !handle.is_null() {
        unsafe { (handle as *mut *mut c_void).write(1usize as *mut c_void) };
    }
    1
}
unsafe extern "C" fn task_create(task: *mut c_void, name: *const c_char, stack: u32, param: *mut c_void, prio: u32, handle: *mut c_void) -> i32 {
    unsafe { task_create_pinned_to_core(task, name, stack, param, prio, handle, 0) }
}
unsafe extern "C" fn delay(_ticks: u32) {}
unsafe extern "C" fn ms_to_tick(ms: u32) -> i32 { ms as i32 }
unsafe extern "C" fn max_priority() -> i32 { 5 }
unsafe extern "C" fn event_post(_base: *const c_char, _id: i32, _data: *mut c_void, _size: usize, _ticks: u32) -> i32 { ESP_OK }
unsafe extern "C" fn free_heap() -> u32 { 64 * 1024 }
unsafe extern "C" fn rand() -> u32 {
    let mut value = RAND_STATE.load(Ordering::Relaxed);
    value = value.wrapping_mul(1664525).wrapping_add(1013904223);
    RAND_STATE.store(value, Ordering::Relaxed);
    value
}
unsafe extern "C" fn ok_country(_country: *const c_char) -> c_int { ESP_OK }
unsafe extern "C" fn read_mac(mac: *mut u8, type_: c_uint) -> c_int {
    if mac.is_null() {
        return -1;
    }
    let mut bytes: [u8; 6] = [0x8c, 0xbf, 0xea, 0x0d, 0xb5, 0x30];
    bytes[5] = bytes[5].wrapping_add(type_ as u8);
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), mac, bytes.len()) };
    ESP_OK
}
unsafe extern "C" fn timer_arm(_timer: *mut c_void, _timeout: u32, _repeat: bool) {}
unsafe extern "C" fn timer_setfn(_timer: *mut c_void, _func: *mut c_void, _arg: *mut c_void) {}
unsafe extern "C" fn timer_get_time() -> i64 { TIME_US.fetch_add(1000, Ordering::Relaxed) as i64 }
unsafe extern "C" fn nvs_set_i8(_h: u32, _k: *const c_char, _v: i8) -> c_int { ESP_OK }
unsafe extern "C" fn nvs_get_i8(_h: u32, _k: *const c_char, out: *mut i8) -> c_int { if !out.is_null() { unsafe { out.write(0) } } ESP_OK }
unsafe extern "C" fn nvs_set_u8(_h: u32, _k: *const c_char, _v: u8) -> c_int { ESP_OK }
unsafe extern "C" fn nvs_get_u8(_h: u32, _k: *const c_char, out: *mut u8) -> c_int { if !out.is_null() { unsafe { out.write(0) } } ESP_OK }
unsafe extern "C" fn nvs_set_u16(_h: u32, _k: *const c_char, _v: u16) -> c_int { ESP_OK }
unsafe extern "C" fn nvs_get_u16(_h: u32, _k: *const c_char, out: *mut u16) -> c_int { if !out.is_null() { unsafe { out.write(0) } } ESP_OK }
unsafe extern "C" fn nvs_open(_name: *const c_char, _mode: c_uint, out: *mut u32) -> c_int { if !out.is_null() { unsafe { out.write(1) } } ESP_OK }
unsafe extern "C" fn nvs_close(_h: u32) {}
unsafe extern "C" fn nvs_commit(_h: u32) -> c_int { ESP_OK }
unsafe extern "C" fn nvs_set_blob(_h: u32, _k: *const c_char, _v: *const c_void, _len: usize) -> c_int { ESP_OK }
unsafe extern "C" fn nvs_get_blob(_h: u32, _k: *const c_char, _out: *mut c_void, len: *mut usize) -> c_int { if !len.is_null() { unsafe { len.write(0) } } ESP_OK }
unsafe extern "C" fn nvs_erase_key(_h: u32, _k: *const c_char) -> c_int { ESP_OK }
unsafe extern "C" fn get_random(buf: *mut u8, len: usize) -> c_int {
    if buf.is_null() {
        return -1;
    }
    let mut i = 0;
    while i < len {
        unsafe { buf.add(i).write(rand() as u8) };
        i += 1;
    }
    ESP_OK
}
unsafe extern "C" fn get_time(_t: *mut c_void) -> c_int { ESP_OK }
unsafe extern "C" fn random() -> c_ulong { unsafe { rand() as c_ulong } }
unsafe extern "C" fn slowclk_cal_get() -> u32 { 32_768 }
unsafe extern "C" fn log_timestamp() -> u32 { (unsafe { timer_get_time() } / 1000) as u32 }
unsafe extern "C" fn os_malloc(size: usize) -> *mut c_void {
    let Ok(layout) = Layout::from_size_align(size.max(1), 4) else { return ptr::null_mut() };
    unsafe { alloc(layout).cast::<c_void>() }
}
unsafe extern "C" fn os_free(_p: *mut c_void) {}
unsafe extern "C" fn os_realloc(_ptr: *mut c_void, size: usize) -> *mut c_void { unsafe { os_malloc(size) } }
unsafe extern "C" fn os_calloc(n: usize, size: usize) -> *mut c_void {
    let Some(total) = n.checked_mul(size) else { return ptr::null_mut() };
    let Ok(layout) = Layout::from_size_align(total.max(1), 4) else { return ptr::null_mut() };
    unsafe { alloc_zeroed(layout).cast::<c_void>() }
}
unsafe extern "C" fn os_zalloc(size: usize) -> *mut c_void { unsafe { os_calloc(1, size) } }
unsafe extern "C" fn coex_condition_set(_t: u32, _d: bool) {}
unsafe extern "C" fn coex_wifi_request(_e: u32, _l: u32, _d: u32) -> c_int { ESP_OK }
unsafe extern "C" fn coex_wifi_release(_e: u32) -> c_int { ESP_OK }
unsafe extern "C" fn coex_wifi_channel_set(_p: u8, _s: u8) -> c_int { ESP_OK }
unsafe extern "C" fn coex_event_duration_get(_e: u32, out: *mut u32) -> c_int { if !out.is_null() { unsafe { out.write(0) } } ESP_OK }
unsafe extern "C" fn coex_pti_get(_e: u32, out: *mut u8) -> c_int { if !out.is_null() { unsafe { out.write(0) } } ESP_OK }
unsafe extern "C" fn coex_status_bit(_t: u32, _s: u32) {}
unsafe extern "C" fn coex_interval_set(_i: u32) -> c_int { ESP_OK }
unsafe extern "C" fn coex_register_cb(_i: c_int, _cb: Option<unsafe extern "C" fn(c_int) -> c_int>) -> c_int { ESP_OK }
unsafe extern "C" fn coex_register_start_cb(_cb: Option<unsafe extern "C" fn() -> c_int>) -> c_int { ESP_OK }
unsafe extern "C" fn coex_flexible_period_set(_p: u8) -> c_int { ESP_OK }
unsafe extern "C" fn coex_get_phase(_idx: c_int) -> *mut c_void { ptr::null_mut() }
