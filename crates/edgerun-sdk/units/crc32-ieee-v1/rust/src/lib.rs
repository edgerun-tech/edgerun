#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(8023);

#[edgerun_unit::export]
unsafe fn crc32_ieee(input_ptr: i32, input_len: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 {
        return -1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    crc32(input) as i32
}

#[edgerun_unit::export]
unsafe fn crc32_ieee_write(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    (out_ptr as *mut u32).write_unaligned(crc32(input));
    0
}

fn crc32(input: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    let mut index = 0;
    while index < input.len() {
        crc ^= input[index] as u32;
        let mut bit = 0;
        while bit < 8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            bit += 1;
        }
        index += 1;
    }
    !crc
}
