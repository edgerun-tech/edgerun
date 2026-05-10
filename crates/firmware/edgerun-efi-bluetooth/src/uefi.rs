use core::ffi::c_void;
use core::ptr::null_mut;

pub type EfiStatus = usize;
pub type EfiHandle = *mut c_void;
pub type Char16 = u16;

pub const EFI_SUCCESS: EfiStatus = 0;
pub const EFI_ERROR_BIT: EfiStatus = 1usize << (usize::BITS as usize - 1);
pub const EFI_LOAD_ERROR: EfiStatus = EFI_ERROR_BIT | 1;
pub const EFI_INVALID_PARAMETER: EfiStatus = EFI_ERROR_BIT | 2;
pub const EFI_UNSUPPORTED: EfiStatus = EFI_ERROR_BIT | 3;
pub const EFI_BAD_BUFFER_SIZE: EfiStatus = EFI_ERROR_BIT | 4;
pub const EFI_BUFFER_TOO_SMALL: EfiStatus = EFI_ERROR_BIT | 5;
pub const EFI_NOT_READY: EfiStatus = EFI_ERROR_BIT | 6;
pub const EFI_DEVICE_ERROR: EfiStatus = EFI_ERROR_BIT | 7;
pub const EFI_NOT_FOUND: EfiStatus = EFI_ERROR_BIT | 14;
pub const EFI_TIMEOUT: EfiStatus = EFI_ERROR_BIT | 18;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct EfiSimpleTextOutputProtocol {
    pub reset: usize,
    pub output_string: extern "efiapi" fn(
        this: *mut EfiSimpleTextOutputProtocol,
        string: *const Char16,
    ) -> EfiStatus,
}

#[repr(C)]
pub struct EfiSystemTable {
    pub hdr: EfiTableHeader,
    pub firmware_vendor: *mut Char16,
    pub firmware_revision: u32,
    pub console_in_handle: EfiHandle,
    pub con_in: *mut c_void,
    pub console_out_handle: EfiHandle,
    pub con_out: *mut EfiSimpleTextOutputProtocol,
    pub standard_error_handle: EfiHandle,
    pub std_err: *mut EfiSimpleTextOutputProtocol,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut EfiBootServices,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut c_void,
}

#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,
    pub raise_tpl: usize,
    pub restore_tpl: usize,
    pub allocate_pages: usize,
    pub free_pages: usize,
    pub get_memory_map: usize,
    pub allocate_pool: extern "efiapi" fn(
        pool_type: u32,
        size: usize,
        buffer: *mut *mut c_void,
    ) -> EfiStatus,
    pub free_pool: extern "efiapi" fn(buffer: *mut c_void) -> EfiStatus,
    pub create_event: usize,
    pub set_timer: usize,
    pub wait_for_event: usize,
    pub signal_event: usize,
    pub close_event: usize,
    pub check_event: usize,
    pub install_protocol_interface: usize,
    pub reinstall_protocol_interface: usize,
    pub uninstall_protocol_interface: usize,
    pub handle_protocol: usize,
    pub reserved: usize,
    pub register_protocol_notify: usize,
    pub locate_handle: usize,
    pub locate_device_path: usize,
    pub install_configuration_table: usize,
    pub load_image: usize,
    pub start_image: usize,
    pub exit: usize,
    pub unload_image: usize,
    pub exit_boot_services: usize,
    pub get_next_monotonic_count: usize,
    pub stall: extern "efiapi" fn(microseconds: usize) -> EfiStatus,
    pub set_watchdog_timer: usize,
    pub connect_controller: usize,
    pub disconnect_controller: usize,
    pub open_protocol: extern "efiapi" fn(
        handle: EfiHandle,
        protocol: *const EfiGuid,
        interface: *mut *mut c_void,
        agent_handle: EfiHandle,
        controller_handle: EfiHandle,
        attributes: u32,
    ) -> EfiStatus,
    pub close_protocol: usize,
    pub open_protocol_information: usize,
    pub protocols_per_handle: usize,
    pub locate_handle_buffer: extern "efiapi" fn(
        search_type: EfiLocateSearchType,
        protocol: *const EfiGuid,
        search_key: *mut c_void,
        no_handles: *mut usize,
        buffer: *mut *mut EfiHandle,
    ) -> EfiStatus,
    pub locate_protocol: usize,
    pub install_multiple_protocol_interfaces: usize,
    pub uninstall_multiple_protocol_interfaces: usize,
    pub calculate_crc32: usize,
    pub copy_mem: usize,
    pub set_mem: usize,
    pub create_event_ex: usize,
}

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum EfiLocateSearchType {
    AllHandles = 0,
    ByRegisterNotify = 1,
    ByProtocol = 2,
}

pub const EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL: u32 = 0x0000_0001;

static mut SYSTEM_TABLE: *mut EfiSystemTable = null_mut();
static mut IMAGE_HANDLE: EfiHandle = null_mut();

pub unsafe fn init(image_handle: EfiHandle, system_table: *mut EfiSystemTable) {
    unsafe {
        IMAGE_HANDLE = image_handle;
        SYSTEM_TABLE = system_table;
    }
}

pub unsafe fn image_handle() -> EfiHandle {
    unsafe { IMAGE_HANDLE }
}

pub unsafe fn boot_services() -> *mut EfiBootServices {
    unsafe { (*SYSTEM_TABLE).boot_services }
}

pub unsafe fn puts(s: &str) {
    let mut wide = [0u16; 384];
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && i + 1 < wide.len() {
        wide[i] = bytes[i] as u16;
        i += 1;
    }
    wide[i] = 0;

    unsafe {
        let con_out = (*SYSTEM_TABLE).con_out;
        ((*con_out).output_string)(con_out, wide.as_ptr());
    }
}

pub unsafe fn stall_us(us: usize) {
    unsafe {
        let bs = boot_services();
        ((*bs).stall)(us);
    }
}

pub unsafe fn free_pool(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let bs = boot_services();
            ((*bs).free_pool)(ptr);
        }
    }
}

pub unsafe fn put_status(status: EfiStatus) {
    if status == EFI_SUCCESS {
        unsafe { puts("EFI_SUCCESS") };
        return;
    }
    unsafe {
        puts("0x");
        put_hex_usize(status);
    }
}

pub unsafe fn put_hex_u8(value: u8) {
    unsafe { put_hex_fixed(value as u64, 2) }
}

pub unsafe fn put_hex_u16(value: u16) {
    unsafe { put_hex_fixed(value as u64, 4) }
}

pub unsafe fn put_hex_usize(value: usize) {
    unsafe { put_hex_fixed(value as u64, usize::BITS as usize / 4) }
}

unsafe fn put_hex_fixed(value: u64, nibbles: usize) {
    let mut buf = [0u8; 16];
    let mut i = 0;
    while i < nibbles {
        let shift = (nibbles - 1 - i) * 4;
        let digit = ((value >> shift) & 0x0f) as u8;
        buf[i] = if digit < 10 { b'0' + digit } else { b'a' + digit - 10 };
        i += 1;
    }
    let s = unsafe { core::str::from_utf8_unchecked(&buf[..nibbles]) };
    unsafe { puts(s) };
}
