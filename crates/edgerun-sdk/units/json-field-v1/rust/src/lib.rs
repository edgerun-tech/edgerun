#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(8259);

#[edgerun_unit::export]
unsafe fn json_string_field_locate(
    json_ptr: i32,
    json_len: i32,
    key_ptr: i32,
    key_len: i32,
    out_ptr: i32,
) -> i32 {
    if json_ptr < 0 || json_len < 0 || key_ptr < 0 || key_len <= 0 || out_ptr < 0 {
        return -1;
    }
    let json = core::slice::from_raw_parts(json_ptr as *const u8, json_len as usize);
    let key = core::slice::from_raw_parts(key_ptr as *const u8, key_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u32, 2);
    let mut index = skip_ws(json, 0);
    if index >= json.len() || json[index] != b'{' {
        return -2;
    }
    index += 1;
    loop {
        index = skip_ws(json, index);
        if index >= json.len() {
            return -2;
        }
        if json[index] == b'}' {
            return -3;
        }
        if json[index] != b'"' {
            return -2;
        }
        let Some((field_start, field_end, next)) = scan_json_string(json, index) else {
            return -2;
        };
        index = skip_ws(json, next);
        if index >= json.len() || json[index] != b':' {
            return -2;
        }
        index = skip_ws(json, index + 1);
        let matches = json_string_eq_unescaped(json, field_start, field_end, key);
        if matches {
            if index >= json.len() || json[index] != b'"' {
                return -4;
            }
            let Some((value_start, value_end, _next)) = scan_json_string(json, index) else {
                return -2;
            };
            out[0] = value_start as u32;
            out[1] = (value_end - value_start) as u32;
            return 0;
        }
        index = skip_value(json, index);
        if index >= json.len() {
            return -2;
        }
        index = skip_ws(json, index);
        if index < json.len() && json[index] == b',' {
            index += 1;
            continue;
        }
        if index < json.len() && json[index] == b'}' {
            return -3;
        }
        return -2;
    }
}

#[edgerun_unit::export]
unsafe fn json_string_decode(input_ptr: i32, input_len: i32, out_ptr: i32, out_cap: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 || out_cap < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, out_cap as usize);
    copy_json_string_unescaped(input, 0, input.len(), out)
}

#[edgerun_unit::export]
unsafe fn json_bool_field_get(json_ptr: i32, json_len: i32, key_ptr: i32, key_len: i32) -> i32 {
    if json_ptr < 0 || json_len < 0 || key_ptr < 0 || key_len <= 0 {
        return -1;
    }
    let json = core::slice::from_raw_parts(json_ptr as *const u8, json_len as usize);
    let key = core::slice::from_raw_parts(key_ptr as *const u8, key_len as usize);
    let mut index = skip_ws(json, 0);
    if index >= json.len() || json[index] != b'{' {
        return -2;
    }
    index += 1;
    loop {
        index = skip_ws(json, index);
        if index >= json.len() {
            return -2;
        }
        if json[index] == b'}' {
            return -3;
        }
        let Some((field_start, field_end, next)) = scan_json_string(json, index) else {
            return -2;
        };
        index = skip_ws(json, next);
        if index >= json.len() || json[index] != b':' {
            return -2;
        }
        index = skip_ws(json, index + 1);
        if json_string_eq_unescaped(json, field_start, field_end, key) {
            if has_prefix(&json[index..], b"true") {
                return 1;
            }
            if has_prefix(&json[index..], b"false") {
                return 0;
            }
            return -4;
        }
        index = skip_value(json, index);
        index = skip_ws(json, index);
        if index < json.len() && json[index] == b',' {
            index += 1;
            continue;
        }
        if index < json.len() && json[index] == b'}' {
            return -3;
        }
        return -2;
    }
}

fn skip_ws(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\n' | b'\r' | b'\t') {
        index += 1;
    }
    index
}

fn scan_json_string(bytes: &[u8], quote: usize) -> Option<(usize, usize, usize)> {
    if quote >= bytes.len() || bytes[quote] != b'"' {
        return None;
    }
    let mut index = quote + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some((quote + 1, index, index + 1)),
            b'\\' => index += 2,
            0..=0x1f => return None,
            _ => index += 1,
        }
    }
    None
}

fn json_string_eq_unescaped(bytes: &[u8], start: usize, end: usize, expected: &[u8]) -> bool {
    let mut input = start;
    let mut out = 0;
    while input < end {
        let byte = if bytes[input] == b'\\' {
            input += 1;
            if input >= end {
                return false;
            }
            match bytes[input] {
                b'"' | b'\\' | b'/' => bytes[input],
                b'b' => 8,
                b'f' => 12,
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                _ => return false,
            }
        } else {
            bytes[input]
        };
        if out >= expected.len() || expected[out] != byte {
            return false;
        }
        out += 1;
        input += 1;
    }
    out == expected.len()
}

fn copy_json_string_unescaped(bytes: &[u8], start: usize, end: usize, out: &mut [u8]) -> i32 {
    let mut input = start;
    let mut written = 0;
    while input < end {
        let byte = if bytes[input] == b'\\' {
            input += 1;
            if input >= end {
                return -2;
            }
            match bytes[input] {
                b'"' | b'\\' | b'/' => bytes[input],
                b'b' => 8,
                b'f' => 12,
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                _ => return -4,
            }
        } else {
            bytes[input]
        };
        if written >= out.len() {
            return -5;
        }
        out[written] = byte;
        written += 1;
        input += 1;
    }
    written as i32
}

fn skip_value(bytes: &[u8], index: usize) -> usize {
    if index >= bytes.len() {
        return index;
    }
    if bytes[index] == b'"' {
        return scan_json_string(bytes, index)
            .map(|(_, _, next)| next)
            .unwrap_or(bytes.len());
    }
    let mut depth = 0i32;
    let mut cursor = index;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' if depth > 0 => depth -= 1,
            b',' | b'}' if depth == 0 => break,
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

fn has_prefix(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}
