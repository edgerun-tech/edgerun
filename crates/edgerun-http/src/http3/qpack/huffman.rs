//! QPACK Huffman coding — uses edgerun-hpack Huffman.

pub use edgerun_hpack::huffman::encode;

use edgerun_hpack::huffman::HuffmanDecoder;

pub fn decode(input: &[u8]) -> std::result::Result<std::vec::Vec<u8>, ()> {
    let mut decoder = HuffmanDecoder::new();
    decoder.decode(input).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huffman_roundtrip() {
        let inputs = [b"hello", b"GET", b"https://example.com/path", b"".as_slice()];
        for input in inputs {
            let encoded = encode(input);
            if input.is_empty() {
                assert!(encoded.is_empty());
                continue;
            }
            let decoded = decode(&encoded).unwrap();
            assert_eq!(&decoded, input);
        }
    }
}