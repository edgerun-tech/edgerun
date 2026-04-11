pub fn encode_varint(value: u64, output: &mut Vec<u8>) {
    if value < 64 {
        output.push(value as u8);
    } else if value < 16384 {
        output.push(((value >> 8) as u8) | 0x40);
        output.push(value as u8);
    } else if value < 1073741824 {
        let bytes = (value as u32).to_be_bytes();
        output.push(bytes[0] | 0x80);
        output.push(bytes[1]);
        output.push(bytes[2]);
        output.push(bytes[3]);
    } else {
        let bytes = value.to_be_bytes();
        output.push(bytes[0] | 0xC0);
        output.extend_from_slice(&bytes[1..]);
    }
}

pub fn decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
    if data.is_empty() {
        return Err("Empty varint".to_string());
    }
    let first = data[0];
    let len = match first >> 6 {
        0 => 1,
        1 => 2,
        2 => 4,
        3 => 8,
        _ => return Err("Invalid varint".to_string()),
    };
    if data.len() < len {
        return Err("Incomplete varint".to_string());
    }
    let value = match len {
        1 => (first & 0x3F) as u64,
        2 => u16::from_be_bytes([first & 0x3F, data[1]]) as u64,
        4 => {
            let b = [first & 0x3F, data[1], data[2], data[3]];
            u32::from_be_bytes(b) as u64
        }
        8 => {
            let mut b: [u8; 8] = data[..8].try_into().unwrap();
            b[0] &= 0x3F;
            u64::from_be_bytes(b)
        }
        _ => unreachable!(),
    };
    Ok((value, len))
}
