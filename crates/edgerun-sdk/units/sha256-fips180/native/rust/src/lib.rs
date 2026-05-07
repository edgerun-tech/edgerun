const MEMORY_LEN: usize = 65_536;
static mut MEMORY: [u8; MEMORY_LEN] = [0; MEMORY_LEN];

#[no_mangle]
pub extern "C" fn proto_abi_version() -> i32 {
    2
}

#[no_mangle]
pub extern "C" fn proto_standard_id() -> i32 {
    180256
}

#[no_mangle]
pub extern "C" fn edgerun_native_memory_len() -> usize {
    MEMORY_LEN
}

#[no_mangle]
pub unsafe extern "C" fn edgerun_native_memory_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(MEMORY).cast::<u8>()
}

#[no_mangle]
pub unsafe extern "C" fn sha256_digest(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    let Some(input_range) = checked_range(input_ptr, input_len) else {
        return 1;
    };
    let Some(output_range) = checked_range(out_ptr, 32) else {
        return 1;
    };
    let digest = edgerun_crypto::sha256(&MEMORY[input_range]);
    MEMORY[output_range].copy_from_slice(&digest);
    0
}

fn checked_range(ptr: i32, len: i32) -> Option<core::ops::Range<usize>> {
    if ptr < 0 || len < 0 {
        return None;
    }
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    (end <= MEMORY_LEN).then_some(start..end)
}
