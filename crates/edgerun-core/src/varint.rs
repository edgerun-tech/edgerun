//! Varint encoding/decoding utilities (protobuf-style LEB128).
//!
//! Delegates to `edgerun_encoding::varint`.

pub use edgerun_encoding::varint::{VarintError, decode_varint_slice, encode_varint};

/// Decodes a varint from a `Read` source (backward-compatible wrapper).
/// Returns `Ok(None)` on EOF (no bytes read), `Err` on partial read.
pub fn decode_varint_from_read<R: std::io::Read>(r: &mut R) -> std::io::Result<Option<u64>> {
    let mut buf = [0u8; 1];
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        match r.read_exact(&mut buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if shift == 0 {
                    return Ok(None);
                } else {
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "truncated varint"));
                }
            }
            Err(e) => return Err(e),
        }
        let b = buf[0];
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(Some(result));
        }
        shift += 7;
        if shift >= 64 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "varint too long"));
        }
    }
}
