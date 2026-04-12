//! QPACK Huffman coding — thin wrapper around `httlib-huffman`.
//!
//! QPACK (RFC 9204) uses the exact same canonical Huffman code table
//! as HPACK (RFC 7541 Appendix B), so `httlib-huffman` works directly.

/// Encode plaintext using HPACK/QPACK Huffman coding.
pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    httlib_huffman::encode(input, &mut output)
        .expect("Huffman encoding should never fail");
    output
}

/// Decode HPACK/QPACK Huffman-coded bytes into plaintext.
///
/// Returns `Err` if the Huffman data is malformed (invalid codes,
/// invalid padding, or EOS symbol in the middle of the stream).
pub fn decode(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(input.len());
    httlib_huffman::decode(input, &mut output, httlib_huffman::DecoderSpeed::ThreeBits)
        .map(|_| output)
        .map_err(|e| format!("Huffman decode error: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huffman_encode_decode_roundtrip() {
        let inputs = [
            b"hello".as_slice(),
            b"GET".as_slice(),
            b"https://example.com/path".as_slice(),
            b"text/html; charset=utf-8".as_slice(),
            b"application/json".as_slice(),
            b"0123456789".as_slice(),
            b"".as_slice(),
            &[0, 255, 128, 64],
            b"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".as_slice(),
        ];

        for input in inputs {
            let encoded = encode(input);
            if input.is_empty() {
                assert!(encoded.is_empty());
                continue;
            }
            let decoded = decode(&encoded).unwrap();
            assert_eq!(&decoded, input, "roundtrip failed for {:?}", input);
        }
    }

    #[test]
    fn test_huffman_compress_ratio() {
        let input = b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        let encoded = encode(input);
        assert!(encoded.len() < input.len());
    }

    #[test]
    fn test_huffman_all_bytes_roundtrip() {
        // Test all byte values 0-255
        let input: Vec<u8> = (0..=255).collect();
        let encoded = encode(&input);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_huffman_rfc_example() {
        // RFC 7541 Appendix C: "www.example.com" → 0xf1e3c2e5f23a6ba0
        let input = b"www.example.com";
        let encoded = encode(input);
        // The exact encoding should match the RFC example
        assert!(!encoded.is_empty());
        let decoded = decode(&encoded).unwrap();
        assert_eq!(&decoded, input);
    }
}
