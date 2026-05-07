#![no_std]

use edgerun_encoding::base64::{
    base64url_decode_into, base64url_decoded_bound as decoded_bound, base64url_nopad_encode_into,
    base64url_nopad_encoded_len,
};

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(4648);

#[edgerun_unit::export]
fn base64url_encoded_len(input_len: i32) -> i32 {
    if input_len < 0 {
        return -1;
    }
    base64url_nopad_encoded_len(input_len as usize) as i32
}

#[edgerun_unit::export]
fn base64url_decoded_bound(input_len: i32) -> i32 {
    if input_len < 0 {
        return -1;
    }
    decoded_bound(input_len as usize) as i32
}

#[edgerun_unit::export]
unsafe fn base64url_encode(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out_len = base64url_nopad_encoded_len(input.len());
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, out_len);
    match base64url_nopad_encode_into(input, out) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[edgerun_unit::export]
unsafe fn base64url_decode(
    input_ptr: i32,
    input_len: i32,
    out_ptr: i32,
    out_len_ptr: i32,
) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 || out_len_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, decoded_bound(input.len()));
    match base64url_decode_into(input, out) {
        Ok(len) => {
            (out_len_ptr as *mut u32).write_unaligned(len as u32);
            0
        }
        Err("invalid base64url length") => 1,
        Err(_) => 2,
    }
}
