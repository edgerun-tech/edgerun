#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(180512);

#[edgerun_unit::export]
unsafe fn sha512_digest(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let digest = edgerun_crypto::sha512(input);
    core::ptr::copy_nonoverlapping(digest.as_ptr(), out_ptr as *mut u8, digest.len());
    0
}
