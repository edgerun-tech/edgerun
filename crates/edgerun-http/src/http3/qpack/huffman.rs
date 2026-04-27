//! QPACK Huffman coding — from edgerun-encoding.

pub use edgerun_encoding::Encoder;

use edgerun_hpack::HuffmanDecoder;

pub fn encode(input: &[u8]) -> std::vec::Vec<u8> {
    edgerun_hpack::huffman::encode(input)
}

pub fn decode(input: &[u8]) -> std::result::Result<std::vec::Vec<u8>, ()> {
    let mut decoder = HuffmanDecoder::new();
    decoder.decode(input).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        for input in [b"hello".as_slice(), b"GET".as_slice(), b"".as_slice()] {
            let enc = encode(input);
            if input.is_empty() {
                assert!(enc.is_empty());
                continue;
            }
            let dec = decode(&enc).unwrap();
            assert_eq!(&dec, input);
        }
    }
}
