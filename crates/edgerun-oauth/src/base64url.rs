//! Base64-URL encoding/decoding (RFC 4648 §5).
//!
//! Uses the URL-safe alphabet: `A-Za-z0-9-_`.

/// Encode bytes to Base64-URL without padding.
pub fn base64url_nopad_encode(input: &[u8]) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity((input.len() + 2) / 3 * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;

        let triple = (b0 << 16) | (b1 << 8) | b2;

        output.push(alphabet[((triple >> 18) & 0x3F) as usize] as char);
        output.push(alphabet[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            output.push(alphabet[((triple >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(alphabet[(triple & 0x3F) as usize] as char);
        }
    }

    output
}

/// Encode bytes to Base64-URL with standard padding.
pub fn base64url_encode(input: &[u8]) -> String {
    let mut s = base64url_nopad_encode(input);
    while s.len() % 4 != 0 {
        s.push('=');
    }
    s
}

/// Decode Base64-URL (with or without padding).
pub fn base64url_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    let stripped = input.trim_end_matches('=');
    let chars: Vec<u8> = stripped.bytes().collect();
    let len = chars.len();

    if len == 0 {
        return Ok(output);
    }

    for chunk in chars.chunks(4) {
        let vals: Result<Vec<u32>, _> = chunk
            .iter()
            .map(|&c| {
                alphabet
                    .iter()
                    .position(|&a| a == c)
                    .map(|i| i as u32)
                    .ok_or("invalid base64url character")
            })
            .collect();
        let vals = vals?;

        let mut triple = 0u32;
        for (i, &v) in vals.iter().enumerate() {
            triple |= v << (18 - i * 6);
        }

        output.push((triple >> 16) as u8);
        if chunk.len() > 2 {
            output.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk.len() > 3 {
            output.push((triple & 0xFF) as u8);
        }
    }

    // Trim any zero bytes added by incomplete final chunk
    output.truncate(output.len() - (4 - len % 4) % 4);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let inputs: &[&[u8]] = &[b"", b"f", b"fo", b"foo", b"foob", b"foobar", &[0, 1, 2, 255, 254, 253]];
        for &input in inputs {
            let encoded = base64url_nopad_encode(input);
            let decoded = base64url_decode(&encoded).unwrap();
            assert_eq!(decoded, input);
        }
    }

    #[test]
    fn test_padding() {
        assert_eq!(base64url_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64url_nopad_encode(b"hello"), "aGVsbG8");
    }
}
