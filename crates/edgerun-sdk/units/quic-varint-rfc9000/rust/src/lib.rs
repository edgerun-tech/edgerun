#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(9000);

#[edgerun_unit::export]
fn quic_varint_encoded_len(value_hi: i32, value_lo: i32) -> i32 {
    if value_hi < 0 || value_lo < 0 {
        return -1;
    }
    let value = ((value_hi as u64) << 32) | value_lo as u32 as u64;
    if value < 64 {
        1
    } else if value < 16_384 {
        2
    } else if value < 1_073_741_824 {
        4
    } else if value < (1u64 << 62) {
        8
    } else {
        -1
    }
}

#[edgerun_unit::export]
unsafe fn quic_varint_encode(value_hi: i32, value_lo: i32, out_ptr: i32, out_len_ptr: i32) -> i32 {
    if value_hi < 0 || value_lo < 0 || out_ptr < 0 || out_len_ptr < 0 {
        return 1;
    }
    let value = ((value_hi as u64) << 32) | value_lo as u32 as u64;
    let len = quic_varint_encoded_len(value_hi, value_lo);
    if len < 0 {
        return 2;
    }
    let out = core::slice::from_raw_parts_mut(out_ptr as *mut u8, len as usize);
    match len {
        1 => out[0] = value as u8,
        2 => {
            out[0] = ((value >> 8) as u8) | 0x40;
            out[1] = value as u8;
        }
        4 => {
            let bytes = (value as u32).to_be_bytes();
            out[0] = bytes[0] | 0x80;
            out[1] = bytes[1];
            out[2] = bytes[2];
            out[3] = bytes[3];
        }
        8 => {
            let bytes = value.to_be_bytes();
            out[0] = bytes[0] | 0xc0;
            out[1..8].copy_from_slice(&bytes[1..8]);
        }
        _ => return 2,
    }
    (out_len_ptr as *mut u32).write_unaligned(len as u32);
    0
}

#[edgerun_unit::export]
unsafe fn quic_varint_decode(input_ptr: i32, input_len: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    if input.is_empty() {
        return 1;
    }
    let len = match input[0] >> 6 {
        0 => 1,
        1 => 2,
        2 => 4,
        _ => 8,
    };
    if input.len() < len {
        return 2;
    }
    let value = match len {
        1 => (input[0] & 0x3f) as u64,
        2 => u16::from_be_bytes([input[0] & 0x3f, input[1]]) as u64,
        4 => u32::from_be_bytes([input[0] & 0x3f, input[1], input[2], input[3]]) as u64,
        8 => {
            let mut bytes = [
                input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7],
            ];
            bytes[0] &= 0x3f;
            u64::from_be_bytes(bytes)
        }
        _ => return 2,
    };
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned((value >> 32) as u32);
    out.add(1).write_unaligned(value as u32);
    out.add(2).write_unaligned(len as u32);
    0
}
