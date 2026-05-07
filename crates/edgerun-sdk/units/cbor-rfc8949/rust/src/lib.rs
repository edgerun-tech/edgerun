#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(8949);

#[edgerun_unit::export]
fn cbor_major_valid(major: i32) -> i32 {
    (0..=7).contains(&major) as i32
}

#[edgerun_unit::export]
unsafe fn cbor_head_parse(ptr: i32, len: i32, out_ptr: i32) -> i32 {
    if ptr < 0 || len <= 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(ptr as *const u8, len as usize);
    let first = input[0];
    let major = (first >> 5) as u32;
    let add = (first & 0x1f) as u32;
    if add == 31 {
        return 3;
    }
    let needed = match add {
        0..=23 => 1,
        24 => 2,
        25 => 3,
        26 => 5,
        27 => 9,
        _ => return 2,
    };
    if input.len() < needed {
        return 1;
    }
    let value = match add {
        0..=23 => add as u64,
        24 => input[1] as u64,
        25 => u16::from_be_bytes([input[1], input[2]]) as u64,
        26 => u32::from_be_bytes([input[1], input[2], input[3], input[4]]) as u64,
        27 => u64::from_be_bytes([
            input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        ]),
        _ => 0,
    };
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(major);
    out.add(1).write_unaligned(add);
    out.add(2).write_unaligned(needed as u32);
    out.add(3).write_unaligned(value as u32);
    out.add(4).write_unaligned((value >> 32) as u32);
    0
}
