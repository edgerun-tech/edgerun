#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(10001);

#[edgerun_unit::export]
unsafe fn form_urlencoded_locate(
    input_ptr: i32,
    input_len: i32,
    key_ptr: i32,
    key_len: i32,
    out_ptr: i32,
) -> i32 {
    if input_ptr < 0 || input_len < 0 || key_ptr < 0 || key_len <= 0 || out_ptr < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let key = core::slice::from_raw_parts(key_ptr as *const u8, key_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u32, 2);
    let mut start = 0usize;
    while start <= input.len() {
        let end = find_byte(input, start, b'&').unwrap_or(input.len());
        let eq = find_byte(input, start, b'=').filter(|pos| *pos <= end);
        let key_end = eq.unwrap_or(end);
        if encoded_eq(&input[start..key_end], key) {
            let value_start = eq.map(|pos| pos + 1).unwrap_or(end);
            out[0] = value_start as u32;
            out[1] = (end - value_start) as u32;
            return 0;
        }
        if end == input.len() {
            break;
        }
        start = end + 1;
    }
    -3
}

#[edgerun_unit::export]
unsafe fn form_urlencoded_decode(
    input_ptr: i32,
    input_len: i32,
    out_ptr: i32,
    out_cap: i32,
) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 || out_cap < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, out_cap as usize);
    decode_component(input, out)
}

fn find_byte(bytes: &[u8], mut start: usize, needle: u8) -> Option<usize> {
    while start < bytes.len() {
        if bytes[start] == needle {
            return Some(start);
        }
        start += 1;
    }
    None
}

fn encoded_eq(input: &[u8], expected: &[u8]) -> bool {
    let mut i = 0usize;
    let mut o = 0usize;
    while i < input.len() {
        let Some(byte) = decoded_byte(input, &mut i) else {
            return false;
        };
        if o >= expected.len() || expected[o] != byte {
            return false;
        }
        o += 1;
    }
    o == expected.len()
}

fn decode_component(input: &[u8], out: &mut [u8]) -> i32 {
    let mut i = 0usize;
    let mut o = 0usize;
    while i < input.len() {
        let Some(byte) = decoded_byte(input, &mut i) else {
            return -2;
        };
        if o >= out.len() {
            return -5;
        }
        out[o] = byte;
        o += 1;
    }
    o as i32
}

fn decoded_byte(input: &[u8], index: &mut usize) -> Option<u8> {
    let byte = input[*index];
    *index += 1;
    match byte {
        b'+' => Some(b' '),
        b'%' => {
            if *index + 1 >= input.len() {
                return None;
            }
            let high = hex(input[*index])?;
            let low = hex(input[*index + 1])?;
            *index += 2;
            Some((high << 4) | low)
        }
        _ => Some(byte),
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
