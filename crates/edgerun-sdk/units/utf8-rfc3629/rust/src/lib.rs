#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(3629);

#[edgerun_unit::export]
fn utf8_scalar_width(first: i32) -> i32 {
    if !(0..=255).contains(&first) {
        return 0;
    }
    match first as u8 {
        0x00..=0x7f => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 0,
    }
}

#[edgerun_unit::export]
unsafe fn utf8_validate(ptr: i32, len: i32) -> i32 {
    if ptr < 0 || len < 0 {
        return 1;
    }
    let bytes = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    match core::str::from_utf8(bytes) {
        Ok(_) => 0,
        Err(err) if err.error_len().is_none() => 1,
        Err(_) => 2,
    }
}
