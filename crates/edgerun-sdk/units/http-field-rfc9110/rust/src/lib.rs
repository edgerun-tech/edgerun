#![no_std]

use edgerun_protocols::http::is_tchar;

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(9110);

#[edgerun_unit::export]
unsafe fn http_field_line_parse(ptr: i32, len: i32, out_ptr: i32) -> i32 {
    if ptr < 0 || len <= 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let mut colon = None;
    let mut index = 0;
    while index < input.len() {
        if input[index] == b':' {
            colon = Some(index);
            break;
        }
        index += 1;
    }
    let Some(colon) = colon else {
        return 1;
    };
    if colon == 0 {
        return 2;
    }
    let mut name_index = 0;
    while name_index < colon {
        if !is_tchar(input[name_index]) {
            return 2;
        }
        name_index += 1;
    }
    let value_start = colon + 1;
    let mut trimmed_start = value_start;
    while trimmed_start < input.len()
        && (input[trimmed_start] == b' ' || input[trimmed_start] == b'\t')
    {
        trimmed_start += 1;
    }
    let mut trimmed_end = input.len();
    while trimmed_end > trimmed_start
        && (input[trimmed_end - 1] == b' ' || input[trimmed_end - 1] == b'\t')
    {
        trimmed_end -= 1;
    }
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(colon as u32);
    out.add(1).write_unaligned(trimmed_start as u32);
    out.add(2).write_unaligned((trimmed_end - trimmed_start) as u32);
    out.add(3).write_unaligned(colon as u32);
    0
}
