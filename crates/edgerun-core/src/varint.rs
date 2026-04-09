//! Varint encoding/decoding utilities (protobuf-style LEB128).

use std::io::{self, Read};

/// Encodes a u64 as a varint, returning the encoded bytes.
pub fn encode_varint(mut v: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
    out
}

/// Decodes a varint from a `Read` source.
/// Returns `Ok(None)` on EOF (no bytes read), `Err` on partial read.
pub fn decode_varint_from_read<R: Read>(r: &mut R) -> io::Result<Option<u64>> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let mut byte = [0u8; 1];
        match r.read_exact(&mut byte) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                if shift == 0 {
                    return Ok(None);
                } else {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated varint"));
                }
            }
            Err(e) => return Err(e),
        }
        let b = byte[0];
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(Some(result));
        }
        shift += 7;
        if shift >= 64 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "varint too long"));
        }
    }
}
