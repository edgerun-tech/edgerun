#![no_std]

edgerun_unit::no_alloc!();

const INPUT_LEN: usize = 1024;
const OUTPUT_LEN: usize = 32;

#[unsafe(no_mangle)]
pub extern "C" fn proto_abi_version() -> i32 {
    edgerun_unit::ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn proto_standard_id() -> i32 {
    31024
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake3_1024(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || out_ptr < 0 || input_len as usize != INPUT_LEN {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, INPUT_LEN);
    let digest = blake3::hash(input);
    core::ptr::copy_nonoverlapping(digest.as_bytes().as_ptr(), out_ptr as *mut u8, OUTPUT_LEN);
    0
}
