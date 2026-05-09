//! ESP32-S3 Bluetooth controller blob probe glue.
//!
//! This is intentionally narrower than the BLE HCI scaffold: it only proves that
//! the local ESP-IDF controller archive links and can expose basic metadata.

#![allow(unsafe_op_in_unsafe_fn)]

use alloc::alloc::{Layout, alloc};
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

pub const VERSION_MAX: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleBlobError {
    NullVersion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BleBlobStatus {
    pub version: [u8; VERSION_MAX],
    pub version_len: usize,
    pub osi_rc: i32,
    pub init_rc: i32,
    pub enable_rc: i32,
    pub vhci_rc: i32,
    pub hci_tx: u32,
}

impl BleBlobStatus {
    pub const fn empty() -> Self {
        Self {
            version: [0; VERSION_MAX],
            version_len: 0,
            osi_rc: i32::MIN,
            init_rc: i32::MIN,
            enable_rc: i32::MIN,
            vhci_rc: i32::MIN,
            hci_tx: 0,
        }
    }

    pub fn version_bytes(&self) -> &[u8] {
        &self.version[..self.version_len]
    }
}

unsafe extern "C" {
    fn btdm_controller_get_compile_version() -> *const c_char;
    fn btdm_controller_rom_data_init();
    fn btdm_osi_funcs_register(osi_funcs: *const c_void) -> i32;
    fn btdm_controller_init(config: *mut BtControllerConfig) -> i32;
    fn btdm_controller_enable(mode: i32) -> i32;
    fn API_vhci_host_check_send_available() -> bool;
    fn API_vhci_host_register_callback(callback: *const VhciHostCallback) -> i32;
    fn API_vhci_host_send_packet(data: *mut u8, len: u16);
    fn r_vhci_init();
}

pub fn probe() -> Result<BleBlobStatus, BleBlobError> {
    let mut status = version_probe()?;
    unsafe { btdm_controller_rom_data_init() };
    status.osi_rc = unsafe { btdm_osi_funcs_register(ptr::addr_of!(BT_OSI_FUNCS).cast()) };
    if status.osi_rc == 0 {
        let mut config = BtControllerConfig::ble_default();
        status.init_rc = unsafe { btdm_controller_init(ptr::addr_of_mut!(config)) };
        if status.init_rc == 0 {
            status.enable_rc = unsafe { btdm_controller_enable(1) };
            if status.enable_rc == 0 {
                unsafe { r_vhci_init() };
                status.vhci_rc =
                    unsafe { API_vhci_host_register_callback(ptr::addr_of!(VHCI_CALLBACK)) };
                status.hci_tx = 0;
            }
        }
    }
    Ok(status)
}

pub fn send_advertising() -> u32 {
    send_adv_sequence()
}

fn version_probe() -> Result<BleBlobStatus, BleBlobError> {
    let ptr = unsafe { btdm_controller_get_compile_version() };
    if ptr.is_null() {
        return Err(BleBlobError::NullVersion);
    }

    let mut status = BleBlobStatus::empty();
    let mut i = 0usize;
    while i < VERSION_MAX {
        let byte = unsafe { *ptr.add(i) as u8 };
        if byte == 0 {
            break;
        }
        status.version[i] = sanitize_version_byte(byte);
        i += 1;
    }
    status.version_len = i;
    Ok(status)
}

fn sanitize_version_byte(byte: u8) -> u8 {
    if (0x20..=0x7e).contains(&byte) {
        byte
    } else {
        b'?'
    }
}

#[repr(C)]
struct BtControllerConfig {
    magic: u32,
    version: u32,
    controller_task_stack_size: u16,
    controller_task_prio: u8,
    controller_task_run_cpu: u8,
    bluetooth_mode: u8,
    ble_max_act: u8,
    sleep_mode: u8,
    sleep_clock: u8,
    ble_st_acl_tx_buf_nb: u8,
    ble_hw_cca_check: u8,
    ble_adv_dup_filt_max: u16,
    coex_param_en: bool,
    ce_len_type: u8,
    coex_use_hooks: bool,
    hci_tl_type: u8,
    hci_tl_funcs: *mut c_void,
    txant_dft: u8,
    rxant_dft: u8,
    txpwr_dft: u8,
    cfg_mask: u32,
    scan_duplicate_mode: u8,
    scan_duplicate_type: u8,
    normal_adv_size: u16,
    mesh_adv_size: u16,
    coex_phy_coded_tx_rx_time_limit: u8,
    hw_target_code: u32,
    slave_ce_len_min: u8,
    hw_recorrect_en: u8,
    cca_thresh: u8,
    scan_backoff_upperlimitmax: u16,
    dup_list_refresh_period: u16,
    ble_50_feat_supp: bool,
    ble_cca_mode: u8,
    ble_data_lenth_zero_aux: u8,
    ble_chan_ass_en: u8,
    ble_ping_en: u8,
    ble_llcp_disc_flag: u8,
    run_in_flash: bool,
    dtm_en: bool,
    enc_en: bool,
    qa_test: bool,
    connect_en: bool,
    scan_en: bool,
    ble_aa_check: bool,
    adv_en: bool,
}

#[repr(C)]
struct VhciHostCallback {
    notify_host_send_available: Option<unsafe extern "C" fn()>,
    notify_host_recv: Option<unsafe extern "C" fn(*mut u8, u16) -> i32>,
}

unsafe impl Sync for VhciHostCallback {}

static VHCI_CALLBACK: VhciHostCallback = VhciHostCallback {
    notify_host_send_available: Some(vhci_send_available),
    notify_host_recv: Some(vhci_recv),
};

unsafe extern "C" fn vhci_send_available() {}

unsafe extern "C" fn vhci_recv(_data: *mut u8, _len: u16) -> i32 {
    0
}

fn send_adv_sequence() -> u32 {
    let mut tx = 0u32;
    tx += send_hci_cmd(0x0c03, &[]) as u32;
    tx += send_hci_cmd(
        0x2006,
        &[
            0xa0, 0x00, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00,
            0x00,
        ],
    ) as u32;
    tx += send_hci_cmd(0x2008, &adv_payload()) as u32;
    tx += send_hci_cmd(0x200a, &[0x01]) as u32;
    tx
}

fn adv_payload() -> [u8; 32] {
    let mut payload = [0u8; 32];
    let data = b"\x02\x01\x06\x08\x09edgerun";
    payload[0] = data.len() as u8;
    let mut i = 0usize;
    while i < data.len() {
        payload[i + 1] = data[i];
        i += 1;
    }
    payload
}

fn send_hci_cmd(opcode: u16, params: &[u8]) -> bool {
    if params.len() > 255 || unsafe { !API_vhci_host_check_send_available() } {
        return false;
    }
    let mut packet = [0u8; 260];
    packet[0] = 0x01;
    packet[1] = opcode as u8;
    packet[2] = (opcode >> 8) as u8;
    packet[3] = params.len() as u8;
    packet[4..4 + params.len()].copy_from_slice(params);
    unsafe { API_vhci_host_send_packet(packet.as_mut_ptr(), (params.len() + 4) as u16) };
    true
}

impl BtControllerConfig {
    fn ble_default() -> Self {
        Self {
            magic: 0x5a5a_a5a5,
            version: 0x0250_9280,
            controller_task_stack_size: 4096,
            controller_task_prio: 5,
            controller_task_run_cpu: 0,
            bluetooth_mode: 1,
            ble_max_act: 1,
            sleep_mode: 0,
            sleep_clock: 0,
            ble_st_acl_tx_buf_nb: 0,
            ble_hw_cca_check: 0,
            ble_adv_dup_filt_max: 30,
            coex_param_en: false,
            ce_len_type: 0,
            coex_use_hooks: false,
            hci_tl_type: 1,
            hci_tl_funcs: ptr::null_mut(),
            txant_dft: 0,
            rxant_dft: 0,
            txpwr_dft: 6,
            cfg_mask: 1,
            scan_duplicate_mode: 0,
            scan_duplicate_type: 0,
            normal_adv_size: 100,
            mesh_adv_size: 100,
            coex_phy_coded_tx_rx_time_limit: 0,
            hw_target_code: 0x0201_0000,
            slave_ce_len_min: 5,
            hw_recorrect_en: 0,
            cca_thresh: 75,
            scan_backoff_upperlimitmax: 0,
            dup_list_refresh_period: 0,
            ble_50_feat_supp: false,
            ble_cca_mode: 0,
            ble_data_lenth_zero_aux: 0,
            ble_chan_ass_en: 1,
            ble_ping_en: 1,
            ble_llcp_disc_flag: 0,
            run_in_flash: false,
            dtm_en: false,
            enc_en: false,
            qa_test: false,
            connect_en: false,
            scan_en: false,
            ble_aa_check: false,
            adv_en: true,
        }
    }
}

unsafe impl Sync for BtOsiFuncs {}

#[repr(C)]
struct BtOsiFuncs {
    magic: u32,
    version: u32,
    interrupt_alloc:
        Option<unsafe extern "C" fn(i32, i32, *mut c_void, *mut c_void, *mut *mut c_void) -> i32>,
    interrupt_free: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    interrupt_handler_set_rsv: Option<unsafe extern "C" fn(i32, *mut c_void, *mut c_void)>,
    global_intr_disable: Option<unsafe extern "C" fn()>,
    global_intr_restore: Option<unsafe extern "C" fn()>,
    task_yield: Option<unsafe extern "C" fn()>,
    task_yield_from_isr: Option<unsafe extern "C" fn()>,
    semphr_create: Option<unsafe extern "C" fn(u32, u32) -> *mut c_void>,
    semphr_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    semphr_take_from_isr: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    semphr_give_from_isr: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
    semphr_take: Option<unsafe extern "C" fn(*mut c_void, u32) -> i32>,
    semphr_give: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    mutex_create: Option<unsafe extern "C" fn() -> *mut c_void>,
    mutex_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    mutex_lock: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    mutex_unlock: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    queue_create: Option<unsafe extern "C" fn(u32, u32) -> *mut c_void>,
    queue_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    queue_send: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    queue_send_from_isr: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32>,
    queue_recv: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    queue_recv_from_isr: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32>,
    task_create: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            u32,
            *mut c_void,
            u32,
            *mut c_void,
            u32,
        ) -> i32,
    >,
    task_delete: Option<unsafe extern "C" fn(*mut c_void)>,
    is_in_isr: Option<unsafe extern "C" fn() -> bool>,
    cause_sw_intr_to_core: Option<unsafe extern "C" fn(i32, i32) -> i32>,
    malloc: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    malloc_internal: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void)>,
    read_efuse_mac: Option<unsafe extern "C" fn(*mut u8) -> i32>,
    srand: Option<unsafe extern "C" fn(u32)>,
    rand: Option<unsafe extern "C" fn() -> i32>,
    btdm_lpcycles_2_hus: Option<unsafe extern "C" fn(u32, *mut u32) -> u32>,
    btdm_hus_2_lpcycles: Option<unsafe extern "C" fn(u32) -> u32>,
    btdm_sleep_check_duration: Option<unsafe extern "C" fn(*mut i32) -> bool>,
    btdm_sleep_enter_phase1: Option<unsafe extern "C" fn(u32)>,
    btdm_sleep_enter_phase2: Option<unsafe extern "C" fn()>,
    btdm_sleep_exit_phase1: Option<unsafe extern "C" fn()>,
    btdm_sleep_exit_phase2: Option<unsafe extern "C" fn()>,
    btdm_sleep_exit_phase3: Option<unsafe extern "C" fn()>,
    coex_wifi_sleep_set: Option<unsafe extern "C" fn(bool)>,
    coex_core_ble_conn_dyn_prio_get: Option<unsafe extern "C" fn(*mut bool, *mut bool) -> i32>,
    coex_schm_register_btdm_callback: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    coex_schm_status_bit_set: Option<unsafe extern "C" fn(u32, u32)>,
    coex_schm_status_bit_clear: Option<unsafe extern "C" fn(u32, u32)>,
    coex_schm_interval_get: Option<unsafe extern "C" fn() -> u32>,
    coex_schm_curr_period_get: Option<unsafe extern "C" fn() -> u8>,
    coex_schm_curr_phase_get: Option<unsafe extern "C" fn() -> *mut c_void>,
    interrupt_enable: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    interrupt_disable: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    esp_hw_power_down: Option<unsafe extern "C" fn()>,
    esp_hw_power_up: Option<unsafe extern "C" fn()>,
    ets_backup_dma_copy: Option<unsafe extern "C" fn(u32, u32, u32, bool)>,
    ets_delay_us: Option<unsafe extern "C" fn(u32)>,
    btdm_rom_table_ready: Option<unsafe extern "C" fn()>,
    coex_bt_wakeup_request: Option<unsafe extern "C" fn() -> bool>,
    coex_bt_wakeup_request_end: Option<unsafe extern "C" fn()>,
    get_time_us: Option<unsafe extern "C" fn() -> i64>,
    assert: Option<unsafe extern "C" fn()>,
}

static BT_OSI_FUNCS: BtOsiFuncs = BtOsiFuncs {
    magic: 0xfade_bead,
    version: 0x0001_000a,
    interrupt_alloc: Some(interrupt_alloc),
    interrupt_free: Some(ok_ptr_ret),
    interrupt_handler_set_rsv: Some(noop_handler_set),
    global_intr_disable: Some(noop),
    global_intr_restore: Some(noop),
    task_yield: Some(noop),
    task_yield_from_isr: Some(noop),
    semphr_create: Some(handle_create),
    semphr_delete: Some(noop_ptr),
    semphr_take_from_isr: Some(ok_ptr2_ret),
    semphr_give_from_isr: Some(ok_ptr2_ret),
    semphr_take: Some(ok_ptr_u32_ret),
    semphr_give: Some(ok_ptr_ret),
    mutex_create: Some(handle_create0),
    mutex_delete: Some(noop_ptr),
    mutex_lock: Some(ok_ptr_ret),
    mutex_unlock: Some(ok_ptr_ret),
    queue_create: Some(handle_create),
    queue_delete: Some(noop_ptr),
    queue_send: Some(ok_ptr_ptr_u32_ret),
    queue_send_from_isr: Some(ok_ptr_ptr_ptr_ret),
    queue_recv: Some(queue_recv),
    queue_recv_from_isr: Some(queue_recv_isr),
    task_create: Some(task_create),
    task_delete: Some(noop_ptr),
    is_in_isr: Some(false_ret),
    cause_sw_intr_to_core: Some(ok_i32_i32_ret),
    malloc: Some(os_malloc),
    malloc_internal: Some(os_malloc),
    free: Some(os_free),
    read_efuse_mac: Some(read_efuse_mac),
    srand: Some(noop_u32),
    rand: Some(rand_ret),
    btdm_lpcycles_2_hus: Some(lpcycles_2_hus),
    btdm_hus_2_lpcycles: Some(identity_u32),
    btdm_sleep_check_duration: Some(false_i32_ptr_ret),
    btdm_sleep_enter_phase1: Some(noop_u32),
    btdm_sleep_enter_phase2: Some(noop),
    btdm_sleep_exit_phase1: Some(noop),
    btdm_sleep_exit_phase2: Some(noop),
    btdm_sleep_exit_phase3: Some(noop),
    coex_wifi_sleep_set: Some(noop_bool),
    coex_core_ble_conn_dyn_prio_get: Some(coex_prio_get),
    coex_schm_register_btdm_callback: Some(ok_ptr_ret),
    coex_schm_status_bit_set: Some(noop_u32_u32),
    coex_schm_status_bit_clear: Some(noop_u32_u32),
    coex_schm_interval_get: Some(zero_u32),
    coex_schm_curr_period_get: Some(zero_u8),
    coex_schm_curr_phase_get: Some(null_ret),
    interrupt_enable: Some(ok_ptr_ret),
    interrupt_disable: Some(ok_ptr_ret),
    esp_hw_power_down: Some(noop),
    esp_hw_power_up: Some(noop),
    ets_backup_dma_copy: Some(noop_u32_u32_u32_bool),
    ets_delay_us: Some(delay_us),
    btdm_rom_table_ready: Some(noop),
    coex_bt_wakeup_request: Some(false_ret),
    coex_bt_wakeup_request_end: Some(noop),
    get_time_us: Some(time_us),
    assert: Some(noop),
};

unsafe extern "C" fn interrupt_alloc(
    _cpu_id: i32,
    _source: i32,
    _handler: *mut c_void,
    _arg: *mut c_void,
    ret_handle: *mut *mut c_void,
) -> i32 {
    if !ret_handle.is_null() {
        ret_handle.write(handle_create0());
    }
    0
}

unsafe extern "C" fn task_create(
    _task_func: *mut c_void,
    _name: *const c_char,
    _stack_depth: u32,
    _param: *mut c_void,
    _prio: u32,
    task_handle: *mut c_void,
    _core_id: u32,
) -> i32 {
    if !task_handle.is_null() {
        (task_handle as *mut *mut c_void).write(handle_create0());
    }
    1
}

unsafe extern "C" fn queue_recv(queue: *mut c_void, item: *mut c_void, _block_time_ms: u32) -> i32 {
    0
}

unsafe extern "C" fn queue_recv_isr(
    queue: *mut c_void,
    item: *mut c_void,
    _hptw: *mut c_void,
) -> i32 {
    0
}

unsafe extern "C" fn handle_create(max: u32, item: u32) -> *mut c_void {
    let size = (max as usize)
        .saturating_mul(item as usize)
        .max(16)
        .min(4096);
    alloc_handle(size)
}

unsafe extern "C" fn handle_create0() -> *mut c_void {
    alloc_handle(16)
}

fn alloc_handle(size: usize) -> *mut c_void {
    let layout = match Layout::from_size_align(size.max(1), 4) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };
    unsafe { alloc(layout).cast() }
}

unsafe extern "C" fn os_malloc(size: usize) -> *mut c_void {
    alloc_handle(size)
}

unsafe extern "C" fn os_free(_ptr: *mut c_void) {}

unsafe extern "C" fn read_efuse_mac(mac: *mut u8) -> i32 {
    if mac.is_null() {
        return -1;
    }
    let bytes = [0x8c, 0xbf, 0xea, 0x0d, 0xb5, 0x30];
    ptr::copy_nonoverlapping(bytes.as_ptr(), mac, bytes.len());
    0
}

unsafe extern "C" fn lpcycles_2_hus(cycles: u32, error_corr: *mut u32) -> u32 {
    if !error_corr.is_null() {
        error_corr.write(0);
    }
    cycles
}

unsafe extern "C" fn coex_prio_get(low: *mut bool, high: *mut bool) -> i32 {
    if !low.is_null() {
        low.write(false);
    }
    if !high.is_null() {
        high.write(false);
    }
    0
}

unsafe extern "C" fn noop() {}
unsafe extern "C" fn noop_ptr(_ptr: *mut c_void) {}
unsafe extern "C" fn noop_u32(_value: u32) {}
unsafe extern "C" fn noop_bool(_value: bool) {}
unsafe extern "C" fn noop_u32_u32(_a: u32, _b: u32) {}
unsafe extern "C" fn noop_u32_u32_u32_bool(_a: u32, _b: u32, _c: u32, _d: bool) {}
unsafe extern "C" fn noop_handler_set(_interrupt_no: i32, _fn: *mut c_void, _arg: *mut c_void) {}
unsafe extern "C" fn delay_us(_us: u32) {}
unsafe extern "C" fn ok_ptr_ret(_ptr: *mut c_void) -> i32 {
    1
}
unsafe extern "C" fn ok_ptr2_ret(_ptr: *mut c_void, _other: *mut c_void) -> i32 {
    1
}
unsafe extern "C" fn ok_ptr_u32_ret(_ptr: *mut c_void, _value: u32) -> i32 {
    1
}
unsafe extern "C" fn ok_ptr_ptr_u32_ret(_ptr: *mut c_void, _item: *mut c_void, _value: u32) -> i32 {
    1
}
unsafe extern "C" fn ok_ptr_ptr_ptr_ret(
    _ptr: *mut c_void,
    _item: *mut c_void,
    _other: *mut c_void,
) -> i32 {
    1
}
unsafe extern "C" fn ok_i32_i32_ret(_a: i32, _b: i32) -> i32 {
    0
}
unsafe extern "C" fn false_ret() -> bool {
    false
}
unsafe extern "C" fn false_i32_ptr_ret(_ptr: *mut i32) -> bool {
    false
}
unsafe extern "C" fn rand_ret() -> i32 {
    4
}
unsafe extern "C" fn identity_u32(value: u32) -> u32 {
    value
}
unsafe extern "C" fn zero_u32() -> u32 {
    0
}
unsafe extern "C" fn zero_u8() -> u8 {
    0
}
unsafe extern "C" fn null_ret() -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn time_us() -> i64 {
    0
}

#[no_mangle]
pub extern "C" fn phy_printf(_format: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn __popcountsi2(value: u32) -> i32 {
    value.count_ones() as i32
}
