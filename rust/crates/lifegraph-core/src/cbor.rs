#[derive(Clone, Debug)]
pub enum CValue {
    Null,
    Bool(bool),
    Int(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<CValue>),
}

fn encode_length(major: u8, value: u64, out: &mut Vec<u8>) {
    match value {
        0..=23 => out.push((major << 5) | value as u8),
        24..=0xff => out.extend_from_slice(&[(major << 5) | 24, value as u8]),
        0x100..=0xffff => {
            out.extend_from_slice(&[(major << 5) | 25, 0, 0]);
            let len = out.len();
            out[len - 2..len].copy_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.extend_from_slice(&[(major << 5) | 26, 0, 0, 0, 0]);
            let len = out.len();
            out[len - 4..len].copy_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.extend_from_slice(&[(major << 5) | 27, 0, 0, 0, 0, 0, 0, 0, 0]);
            let len = out.len();
            out[len - 8..len].copy_from_slice(&value.to_be_bytes());
        }
    }
}

pub fn cbor_dumps(value: &CValue) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    encode(value, &mut out)?;
    Ok(out)
}

fn encode(value: &CValue, out: &mut Vec<u8>) -> Result<(), String> {
    match value {
        CValue::Null => out.push(0xf6),
        CValue::Bool(false) => out.push(0xf4),
        CValue::Bool(true) => out.push(0xf5),
        CValue::Int(v) if *v >= 0 => encode_length(0, *v as u64, out),
        CValue::Int(v) => encode_length(1, (-1 - *v) as u64, out),
        CValue::Bytes(v) => {
            encode_length(2, v.len() as u64, out);
            out.extend_from_slice(v);
        }
        CValue::Text(s) => {
            encode_length(3, s.as_bytes().len() as u64, out);
            out.extend_from_slice(s.as_bytes());
        }
        CValue::Array(items) => {
            encode_length(4, items.len() as u64, out);
            for item in items {
                encode(item, out)?;
            }
        }
    }
    Ok(())
}
