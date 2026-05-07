#![no_std]

use edgerun_http::is_tchar;

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(9110);

#[edgerun_unit::export]
fn http_tchar_valid(char: i32) -> i32 {
    if !(0..=255).contains(&char) {
        return 0;
    }
    is_tchar(char as u8) as i32
}

#[edgerun_unit::export]
unsafe fn http_token_validate(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len <= 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let mut index = 0;
    while index < input.len() {
        if !is_tchar(input[index]) {
            return 2;
        }
        index += 1;
    }
    0
}
