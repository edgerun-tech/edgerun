#![no_std]

edgerun_unit::no_alloc!();

const STORAGE_LEN: usize = 1024;

static mut STORAGE: [u8; STORAGE_LEN] = [0; STORAGE_LEN];
static mut IS_STORED: u8 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn proto_abi_version() -> i32 {
    edgerun_unit::ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn proto_standard_id() -> i32 {
    11024
}

#[unsafe(no_mangle)]
pub extern "C" fn storage_len() -> i32 {
    STORAGE_LEN as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn store(input_ptr: i32, input_len: i32) -> i32 {
    if input_ptr < 0 || input_len as usize != STORAGE_LEN {
        return 1;
    }
    core::ptr::copy_nonoverlapping(
        input_ptr as *const u8,
        core::ptr::addr_of_mut!(STORAGE).cast::<u8>(),
        STORAGE_LEN,
    );
    IS_STORED = 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load(out_ptr: i32, out_len: i32) -> i32 {
    if out_ptr < 0 || out_len as usize != STORAGE_LEN || IS_STORED == 0 {
        return 1;
    }
    core::ptr::copy_nonoverlapping(
        core::ptr::addr_of!(STORAGE).cast::<u8>(),
        out_ptr as *mut u8,
        STORAGE_LEN,
    );
    0
}
