#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1035);

#[edgerun_unit::export]
fn dns_header_length() -> i32 {
    12
}

#[edgerun_unit::export]
unsafe fn dns_header_parse(message_ptr: i32, message_len: i32, out_ptr: i32) -> i32 {
    if message_ptr < 0 || message_len < 0 || out_ptr < 0 {
        return 1;
    }
    let message = core::slice::from_raw_parts(message_ptr as *const u8, message_len as usize);
    if message.len() < 12 {
        return 1;
    }
    let out = out_ptr as *mut u32;
    let mut index = 0;
    while index < 6 {
        let offset = index * 2;
        out.add(index)
            .write_unaligned(u16::from_be_bytes([message[offset], message[offset + 1]]) as u32);
        index += 1;
    }
    0
}

#[edgerun_unit::export]
fn dns_is_query(flags: i32) -> i32 {
    if flags < 0 {
        return 0;
    }
    ((flags as u32 & 0x8000) == 0) as i32
}
