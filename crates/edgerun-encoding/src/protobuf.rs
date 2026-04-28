//! Protobuf canonical encoding utilities.
//!
//! Consolidated from repeated patterns across:
//! - `edgerun-node/src/command_dispatch.rs` (13 direct encode calls for signing)
//! - `edgerun-node/src/daemon.rs`
//! - `edgerun-node/src/query_engine.rs`
//! - `edgerun-node/src/store_task.rs`
//! - `edgerun-node/src/assurance.rs`
//! - `edgerun-secret-service/src/backend.rs`
//! - `edgerun-storage/src/store.rs`
//! - `edgerun-core/src/command.rs`
//! - `edgerun-remote-capability/src/capability_signature.rs`
//!
//! Provides a unified interface for:
//! - Canonical protobuf encoding (prost::Message::encode)
//! - Sign-then-encode pattern (clear signature → encode → hash → sign)
//! - Verification pattern (encode → hash → verify)

use alloc::vec::Vec;
use core::fmt::Debug;

/// Trait for types that can be canonicalized to protobuf bytes.
///
/// This wraps `prost::Message::encode()` in a trait so callers don't need
/// to import prost directly.
pub trait CanonicalEncode {
    /// Encode this message to canonical protobuf bytes.
    ///
    /// Returns the raw protobuf wire format. For signing, use
    /// `canonical_bytes_for_signing` instead to clear the signature field first.
    fn encode_to_vec(&self) -> Vec<u8>;

    /// Decode from canonical protobuf bytes.
    fn decode_from_bytes(bytes: &[u8]) -> Result<Self, CanonicalError>
    where
        Self: Sized;
}

/// Error type for canonical encoding operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    /// Failed to encode to protobuf bytes.
    EncodeFailed,
    /// Failed to decode from protobuf bytes.
    DecodeFailed,
    /// Signature field was not present for clearing.
    NoSignatureField,
}

impl core::fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CanonicalError::EncodeFailed => write!(f, "protobuf encode failed"),
            CanonicalError::DecodeFailed => write!(f, "protobuf decode failed"),
            CanonicalError::NoSignatureField => write!(f, "no signature field to clear"),
        }
    }
}

/// A message that supports canonical bytes for signing.
///
/// This trait is for types that have a `.signature` field (typically `Vec<u8>` or `Option<Vec<u8>>`)
/// that must be cleared before computing the canonical bytes for signing.
pub trait SignableMessage: CanonicalEncode {
    /// Clear the signature field on a clone of this message.
    ///
    /// Returns a copy of the message with the `.signature` field set to empty/default.
    fn clear_signature(&self) -> Self;
}

/// Encode a message to canonical protobuf bytes for signing.
///
/// This is the consolidated pattern from `edgerun-node/src/command_dispatch.rs` and others:
/// 1. Clone the message
/// 2. Clear the `.signature` field
/// 3. Encode to protobuf bytes
///
/// # Examples
/// ```text
/// use edgerun_encoding::protobuf::canonical_bytes_for_signing;
/// let canonical = canonical_bytes_for_signing(&message)?;
/// let digest = edgerun_crypto::sha256(&canonical);
/// // sign digest...
/// ```
pub fn canonical_bytes_for_signing<M: SignableMessage>(
    message: &M,
) -> Result<Vec<u8>, CanonicalError> {
    let cleared = message.clear_signature();
    Ok(cleared.encode_to_vec())
}

/// Compute the SHA-256 digest of canonical protobuf bytes.
///
/// This is a convenience wrapper for the common pattern:
/// `canonical_bytes_for_signing` → `sha256`
///
/// The caller must provide a SHA-256 function, since `edgerun-encoding` has no crypto deps.
///
/// # Examples
/// ```text
/// use edgerun_encoding::protobuf::hash_canonical;
/// let digest = hash_canonical(&message, |bytes| edgerun_crypto::sha256(bytes))?;
/// ```
pub fn hash_canonical<M, F>(message: &M, sha256_fn: F) -> Result<Vec<u8>, CanonicalError>
where
    M: SignableMessage,
    F: FnOnce(&[u8]) -> Vec<u8>,
{
    let canonical = canonical_bytes_for_signing(message)?;
    Ok(sha256_fn(&canonical))
}

/// Encode a message directly to protobuf bytes (without clearing signature).
///
/// Use this for non-signing purposes (storage, transmission, etc.).
///
/// # Examples
/// ```text
/// use edgerun_encoding::protobuf::encode_message;
/// let bytes = encode_message(&message)?;
/// ```
pub fn encode_message<M: CanonicalEncode>(message: &M) -> Result<Vec<u8>, CanonicalError> {
    Ok(message.encode_to_vec())
}

/// Decode a message from protobuf bytes.
///
/// # Examples
/// ```text
/// use edgerun_encoding::protobuf::decode_message;
/// let message: MyMessage = decode_message(&bytes)?;
/// ```
pub fn decode_message<M: CanonicalEncode>(bytes: &[u8]) -> Result<M, CanonicalError> {
    M::decode_from_bytes(bytes)
}

/// Encode with length-prefix framing.
///
/// Prefixes the encoded message with a varint length.
/// This is the standard pattern for length-delimited protobuf streams.
///
/// # Examples
/// ```text
/// use edgerun_encoding::protobuf::encode_length_prefixed;
/// let framed = encode_length_prefixed(&message)?;
/// ```
pub fn encode_length_prefixed<M: CanonicalEncode>(message: &M) -> Result<Vec<u8>, CanonicalError> {
    let encoded = message.encode_to_vec();
    let len_varint = encode_varint(encoded.len() as u64);
    let mut result = Vec::with_capacity(len_varint.len() + encoded.len());
    result.extend_from_slice(&len_varint);
    result.extend_from_slice(&encoded);
    Ok(result)
}

/// Decode a length-prefixed message.
///
/// Reads a varint length prefix, then decodes that many bytes as the message.
///
/// # Examples
/// ```text
/// use edgerun_encoding::protobuf::decode_length_prefixed;
/// let (message, bytes_consumed) = decode_length_prefixed::<MyMessage>(&bytes)?;
/// ```
pub fn decode_length_prefixed<M: CanonicalEncode>(
    bytes: &[u8],
) -> Result<(M, usize), CanonicalError> {
    let (length, varint_len) = decode_varint(bytes).ok_or(CanonicalError::DecodeFailed)?;
    let message_len = length as usize;

    if varint_len + message_len > bytes.len() {
        return Err(CanonicalError::DecodeFailed);
    }

    let message_bytes = &bytes[varint_len..varint_len + message_len];
    let message = M::decode_from_bytes(message_bytes)?;

    Ok((message, varint_len + message_len))
}

// ─── Varint encoding (copied from edgerun-core/varint.rs) ─────────────────────

/// Encode a u64 as a protobuf varint (LEB128-style).
fn encode_varint(v: u64) -> Vec<u8> {
    let mut result = Vec::new();
    let mut value = v;

    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            result.push(byte);
            break;
        }
        result.push(byte | 0x80);
    }

    result
}

/// Decode a varint from bytes.
/// Returns `(value, bytes_consumed)` or `None` on error.
fn decode_varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut consumed = 0;

    for &byte in bytes {
        consumed += 1;
        let value = (byte & 0x7F) as u64;

        if shift >= 64 {
            return None; // overflow
        }

        // For the 10th byte (shift=63), only 1 bit is valid
        if shift == 63 && value > 1 {
            return None; // overflow
        }

        result |= value << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            return Some((result, consumed));
        }
    }

    None // incomplete varint
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_encode_varint() {
        assert_eq!(encode_varint(0), vec![0x00]);
        assert_eq!(encode_varint(1), vec![0x01]);
        assert_eq!(encode_varint(127), vec![0x7F]);
        assert_eq!(encode_varint(128), vec![0x80, 0x01]);
        assert_eq!(encode_varint(300), vec![0xAC, 0x02]);
        assert_eq!(
            encode_varint(u64::MAX),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]
        );
    }

    #[test]
    fn test_decode_varint() {
        assert_eq!(decode_varint(&[0x00]), Some((0, 1)));
        assert_eq!(decode_varint(&[0x01]), Some((1, 1)));
        assert_eq!(decode_varint(&[0x7F]), Some((127, 1)));
        assert_eq!(decode_varint(&[0x80, 0x01]), Some((128, 2)));
        assert_eq!(decode_varint(&[0xAC, 0x02]), Some((300, 2)));
    }

    #[test]
    fn test_decode_varint_incomplete() {
        assert_eq!(decode_varint(&[0x80]), None);
        assert_eq!(decode_varint(&[0x80, 0x80, 0x80]), None);
    }

    #[test]
    fn test_varint_roundtrip() {
        let test_values: Vec<u64> = vec![
            0,
            1,
            127,
            128,
            255,
            256,
            300,
            u16::MAX as u64,
            u32::MAX as u64,
            u64::MAX,
        ];
        for value in test_values {
            let encoded = encode_varint(value);
            let (decoded, _) = decode_varint(&encoded).unwrap();
            assert_eq!(value, decoded);
        }
    }
}
