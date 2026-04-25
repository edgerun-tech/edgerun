//! Frame encoding utilities — length-prefixed binary framing.
//!
//! Provides configurable-length prefix encoding (u64 BE/LE, u16 BE)
//! used by TCP servers, DNS-over-TLS, TFTP, and other length-framed protocols.
//!
//! All operations are `no_std` compatible.

use alloc::vec::Vec;

// ===========================================================================
// 8-byte big-endian length prefix (u64 BE)
// ===========================================================================

/// Encode a payload with an 8-byte big-endian length prefix.
///
/// Wire format: `[len_u64_be][payload...]`
///
/// # Examples
/// ```
/// use edgerun_encoding::frame::{encode_frame, decode_frame_len};
/// let frame = encode_frame(b"hello");
/// assert_eq!(frame.len(), 13);
/// assert_eq!(decode_frame_len(&frame[..8].try_into().unwrap()), 5);
/// ```
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Decode the frame length from the first 8 bytes (big-endian).
///
/// # Examples
/// ```
/// use edgerun_encoding::frame::decode_frame_len;
/// assert_eq!(decode_frame_len(&[0, 0, 0, 0, 0, 0, 0, 5]), 5);
/// ```
pub fn decode_frame_len(header: &[u8; 8]) -> usize {
    u64::from_be_bytes(*header) as usize
}

// ===========================================================================
// 8-byte little-endian length prefix (u64 LE)
// ===========================================================================

/// Encode a payload with an 8-byte little-endian length prefix.
pub fn encode_frame_le(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Decode the frame length from the first 8 bytes (little-endian).
pub fn decode_frame_len_le(header: &[u8; 8]) -> usize {
    u64::from_le_bytes(*header) as usize
}

// ===========================================================================
// 2-byte big-endian length prefix (u16 BE) — DNS-over-TCP, TFTP
// ===========================================================================

/// Encode a payload with a 2-byte big-endian length prefix.
///
/// Used by DNS-over-TCP (RFC 7766) and DNS-over-TLS (RFC 7858).
///
/// # Panics
/// Panics if `payload.len()` exceeds `u16::MAX` (65535 bytes).
pub fn encode_frame_u16_be(payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    assert!(
        len <= u16::MAX as usize,
        "payload too large for u16 length prefix: {len}"
    );
    let mut frame = Vec::with_capacity(2 + len);
    frame.extend_from_slice(&(len as u16).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Decode the frame length from the first 2 bytes (big-endian).
pub fn decode_frame_len_u16_be(header: &[u8; 2]) -> usize {
    u16::from_be_bytes(*header) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_encode_decode_frame_empty() {
        let frame = encode_frame(&[]);
        assert_eq!(frame.len(), 8);
        assert_eq!(decode_frame_len(&frame[..8].try_into().unwrap()), 0);
    }

    #[test]
    fn test_encode_decode_frame_basic() {
        let payload = b"hello world";
        let frame = encode_frame(payload);
        assert_eq!(frame.len(), 8 + 11);
        let len = decode_frame_len(&frame[..8].try_into().unwrap());
        assert_eq!(len, 11);
        assert_eq!(&frame[8..], payload);
    }

    #[test]
    fn test_encode_decode_frame_le() {
        let payload = b"test";
        let frame = encode_frame_le(payload);
        assert_eq!(frame.len(), 12);
        let len = decode_frame_len_le(&frame[..8].try_into().unwrap());
        assert_eq!(len, 4);
        assert_eq!(&frame[8..], payload);
    }

    #[test]
    fn test_encode_decode_frame_u16_be() {
        let payload = b"dns query";
        let frame = encode_frame_u16_be(payload);
        assert_eq!(frame.len(), 11);
        let len = decode_frame_len_u16_be(&frame[..2].try_into().unwrap());
        assert_eq!(len, 9);
        assert_eq!(&frame[2..], payload);
    }

    #[test]
    fn test_frame_u16_be_zero() {
        let frame = encode_frame_u16_be(&[]);
        assert_eq!(frame, vec![0, 0]);
        assert_eq!(decode_frame_len_u16_be(&frame[..2].try_into().unwrap()), 0);
    }

    #[test]
    fn test_frame_u16_be_max() {
        let payload = vec![0u8; u16::MAX as usize];
        let frame = encode_frame_u16_be(&payload);
        assert_eq!(frame.len(), 2 + u16::MAX as usize);
        assert_eq!(
            decode_frame_len_u16_be(&frame[..2].try_into().unwrap()),
            u16::MAX as usize
        );
    }
}
