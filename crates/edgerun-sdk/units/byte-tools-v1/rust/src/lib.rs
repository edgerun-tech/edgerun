#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
unsafe fn byte_eq(left_ptr: i32, left_len: i32, right_ptr: i32, right_len: i32) -> i32 {
    if left_ptr < 0 || left_len < 0 || right_ptr < 0 || right_len < 0 || left_len != right_len {
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
    (diff == 0) as i32
}

#[edgerun_unit::export]
unsafe fn byte_prefix(input_ptr: i32, input_len: i32, prefix_ptr: i32, prefix_len: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || prefix_ptr < 0 || prefix_len < 0 || prefix_len > input_len {
        return 0;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let prefix = core::slice::from_raw_parts(prefix_ptr as *const u8, prefix_len as usize);
    let mut index = 0;
    while index < prefix.len() {
        if input[index] != prefix[index] {
            return 0;
        }
        index += 1;
    }
    1
}

#[edgerun_unit::export]
unsafe fn byte_find(ptr: i32, len: i32, byte: i32) -> i32 {
    if ptr < 0 || len < 0 || !(0..=255).contains(&byte) {
        return -1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let mut index = 0;
    while index < input.len() {
        if input[index] == byte as u8 {
            return index as i32;
        }
        index += 1;
    }
    -1
}

#[edgerun_unit::export]
unsafe fn byte_ascii_lower(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, input.len());
    let mut index = 0;
    while index < input.len() {
        out[index] = match input[index] {
            b'A'..=b'Z' => input[index] + 32,
            byte => byte,
        };
        index += 1;
    }
    0
}

#[edgerun_unit::export]
unsafe fn byte_ascii_case_eq(left_ptr: i32, left_len: i32, right_ptr: i32, right_len: i32) -> i32 {
    if left_ptr < 0 || left_len < 0 || right_ptr < 0 || right_len < 0 || left_len != right_len {
        return 0;
    }
    let left = core::slice::from_raw_parts(left_ptr as *const u8, left_len as usize);
    let right = core::slice::from_raw_parts(right_ptr as *const u8, right_len as usize);
    let mut diff = 0u8;
    let mut index = 0;
    while index < left.len() {
        diff |= ascii_lower(left[index]) ^ ascii_lower(right[index]);
        index += 1;
    }
    (diff == 0) as i32
}

fn ascii_lower(byte: u8) -> u8 {
    match byte {
        b'A'..=b'Z' => byte + 32,
        _ => byte,
    }
}
