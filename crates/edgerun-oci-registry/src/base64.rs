//! Minimal base64 implementation (standard alphabet, no padding stripping).

const ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    let chunks = data.chunks_exact(3);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = chunk[1] as u32;
        let b2 = chunk[2] as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        out.push(ALPHABET[(triple & 0x3F) as usize] as char);
    }

    match remainder.len() {
        1 => {
            let b0 = remainder[0] as u32;
            out.push(ALPHABET[((b0 >> 2) & 0x3F) as usize] as char);
            out.push(ALPHABET[((b0 << 4) & 0x3F) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let b0 = remainder[0] as u32;
            let b1 = remainder[1] as u32;
            let triple = (b0 << 8) | b1;
            out.push(ALPHABET[((triple >> 10) & 0x3F) as usize] as char);
            out.push(ALPHABET[((triple >> 4) & 0x3F) as usize] as char);
            out.push(ALPHABET[((triple << 2) & 0x3F) as usize] as char);
            out.push('=');
        }
        _ => {}
    }

    out
}

fn decode_char(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a' + 26) as u32),
        b'0'..=b'9' => Some((c - b'0' + 52) as u32),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub fn decode(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input.as_bytes();
    let padding = bytes.iter().rev().take_while(|&&b| b == b'=').count();
    let data_len = (bytes.len() / 4) * 3 + match padding {
        0 => 0,
        1 => 2,
        2 => 1,
        _ => return Err("invalid padding".into()),
    };

    let mut out = Vec::with_capacity(data_len);
    let chunks = bytes.chunks_exact(4);

    for chunk in chunks {
        let vals: Result<Vec<u32>, String> = chunk
            .iter()
            .filter(|&&b| b != b'=')
            .map(|&b| decode_char(b).ok_or_else(|| "invalid base64 char".to_string()))
            .collect();
        let vals = vals?;

        match vals.len() {
            4 => {
                let triple = (vals[0] << 18) | (vals[1] << 12) | (vals[2] << 6) | vals[3];
                out.push((triple >> 16) as u8);
                out.push(((triple >> 8) & 0xFF) as u8);
                out.push((triple & 0xFF) as u8);
            }
            3 => {
                let triple = (vals[0] << 10) | (vals[1] << 4) | (vals[2] >> 2);
                out.push((triple >> 8) as u8);
                out.push((triple & 0xFF) as u8);
            }
            2 => {
                let val = (vals[0] << 2) | (vals[1] >> 4);
                out.push(val as u8);
            }
            _ => return Err("invalid base64 length".into()),
        }
    }

    Ok(out)
}
