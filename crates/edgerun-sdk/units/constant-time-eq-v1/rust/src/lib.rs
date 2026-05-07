#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
unsafe fn constant_time_eq(
    left_ptr: i32,
    left_len: i32,
    right_ptr: i32,
    right_len: i32,
    out_ptr: i32,
) -> i32 {
    if left_ptr < 0 || left_len < 0 || right_ptr < 0 || right_len < 0 || out_ptr < 0 {
        return 1;
    }
    let out = out_ptr as *mut u8;
    if left_len != right_len {
        out.write(2);
        return 0;
    }
    let left = core::slice::from_raw_parts(left_ptr as *const u8, left_len as usize);
    let right = core::slice::from_raw_parts(right_ptr as *const u8, right_len as usize);
    let mut diff = 0u8;
    let mut index = 0;
    while index < left.len() {
        diff |= left[index] ^ right[index];
        index += 1;
    }
    out.write((diff != 0) as u8);
    0
}
