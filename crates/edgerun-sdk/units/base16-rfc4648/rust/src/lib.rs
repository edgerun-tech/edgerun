#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(4648);

#[edgerun_unit::export]
fn base16_encoded_len(input_len: i32) -> i32 {
    if input_len < 0 {
        return -1;
    }
    input_len.saturating_mul(2)
}

#[edgerun_unit::export]
fn base16_decoded_bound(input_len: i32) -> i32 {
    if input_len < 0 {
        return -1;
    }
    input_len / 2
}

#[edgerun_unit::export]
unsafe fn base16_encode(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, input.len() * 2);
    let mut index = 0;
    while index < input.len() {
        let byte = input[index];
        out[index * 2] = hex_nibble(byte >> 4);
        out[index * 2 + 1] = hex_nibble(byte & 0x0f);
        index += 1;
    }
    0
}

#[edgerun_unit::export]
unsafe fn base16_decode(input_ptr: i32, input_len: i32, out_ptr: i32, out_len_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 || out_len_ptr < 0 {
        return 1;
    }
    if input_len % 2 != 0 {
        return 2;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, input.len() / 2);
    let mut index = 0;
    while index < out.len() {
        let Some(high) = hex_value(input[index * 2]) else {
            return 3;
        };
        let Some(low) = hex_value(input[index * 2 + 1]) else {
            return 3;
        };
        out[index] = (high << 4) | low;
        index += 1;
    }
    (out_len_ptr as *mut u32).write_unaligned(out.len() as u32);
    0
}

fn hex_nibble(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'a' + value - 10,
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
