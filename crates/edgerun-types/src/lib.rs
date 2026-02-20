use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const BUNDLE_HASH_LEN: usize = 32;
pub const OUTPUT_HASH_LEN: usize = 32;
pub const RUNTIME_ID_LEN: usize = 32;
pub const BUNDLE_ABI_MIN_SUPPORTED: u8 = 1;
pub const BUNDLE_ABI_CURRENT: u8 = 2;
pub const COMMITTEE_SIZE_MVP: u8 = 3;
pub const QUORUM_MVP: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_memory_bytes: u32,
    pub max_instructions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub job_id: [u8; 32],
    pub bundle_hash: [u8; BUNDLE_HASH_LEN],
    pub runtime_id: [u8; RUNTIME_ID_LEN],
    pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundlePayload {
    pub v: u8,
    pub runtime_id: [u8; RUNTIME_ID_LEN],
    pub wasm: Vec<u8>,
    pub input: Vec<u8>,
    pub limits: Limits,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<BundleMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Error)]
pub enum BundleCodecError {
    #[error("unsupported bundle version {0}")]
    UnsupportedVersion(u8),
    #[error("bundle decode failed: {0}")]
    Decode(String),
    #[error("bundle encode failed: {0}")]
    Encode(String),
    #[error("bundle encoding is not canonical")]
    NonCanonicalEncoding,
}

pub fn encode_bundle_payload_canonical(
    payload: &BundlePayload,
) -> Result<Vec<u8>, BundleCodecError> {
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(payload, &mut bytes)
        .map_err(|e| BundleCodecError::Encode(e.to_string()))?;
    Ok(bytes)
}

pub fn decode_bundle_payload_canonical(bytes: &[u8]) -> Result<BundlePayload, BundleCodecError> {
    let payload: BundlePayload =
        ciborium::de::from_reader(bytes).map_err(|e| BundleCodecError::Decode(e.to_string()))?;

    if !(BUNDLE_ABI_MIN_SUPPORTED..=BUNDLE_ABI_CURRENT).contains(&payload.v) {
        return Err(BundleCodecError::UnsupportedVersion(payload.v));
    }

    let reencoded = encode_bundle_payload_canonical(&payload)?;
    if reencoded != bytes {
        return Err(BundleCodecError::NonCanonicalEncoding);
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::value::Value;

    fn sample_payload() -> BundlePayload {
        BundlePayload {
            v: BUNDLE_ABI_MIN_SUPPORTED,
            runtime_id: [7_u8; 32],
            wasm: vec![0x00, 0x61, 0x73, 0x6d],
            input: vec![1, 2, 3],
            limits: Limits {
                max_memory_bytes: 1024,
                max_instructions: 2048,
            },
            meta: None,
        }
    }

    #[test]
    fn canonical_round_trip_is_byte_stable() {
        let payload = sample_payload();
        let encoded = encode_bundle_payload_canonical(&payload).expect("encode");
        let decoded = decode_bundle_payload_canonical(&encoded).expect("decode");
        assert_eq!(decoded, payload);
        let reencoded = encode_bundle_payload_canonical(&decoded).expect("re-encode");
        assert_eq!(encoded, reencoded);
    }

    #[test]
    fn reject_non_canonical_key_order() {
        let limits = Value::Map(vec![
            (
                Value::Text("max_instructions".to_string()),
                Value::Integer(2048_u64.into()),
            ),
            (
                Value::Text("max_memory_bytes".to_string()),
                Value::Integer(1024_u64.into()),
            ),
        ]);

        let non_canonical = Value::Map(vec![
            (
                Value::Text("wasm".to_string()),
                Value::Bytes(vec![0x00, 0x61, 0x73, 0x6d]),
            ),
            (Value::Text("v".to_string()), Value::Integer(1_u64.into())),
            (
                Value::Text("runtime_id".to_string()),
                Value::Bytes(vec![7_u8; 32]),
            ),
            (
                Value::Text("input".to_string()),
                Value::Bytes(vec![1, 2, 3]),
            ),
            (Value::Text("limits".to_string()), limits),
        ]);

        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&non_canonical, &mut bytes).expect("encode test bytes");

        let err = decode_bundle_payload_canonical(&bytes).expect_err("must reject non-canonical");
        assert!(matches!(err, BundleCodecError::NonCanonicalEncoding));
    }

    #[test]
    fn accepts_supported_abi_range() {
        let mut payload = sample_payload();
        payload.v = BUNDLE_ABI_CURRENT;
        let encoded = encode_bundle_payload_canonical(&payload).expect("encode");
        let decoded = decode_bundle_payload_canonical(&encoded).expect("decode");
        assert_eq!(decoded.v, BUNDLE_ABI_CURRENT);
    }

    #[test]
    fn supports_optional_meta_fields() {
        let mut payload = sample_payload();
        payload.meta = Some(BundleMeta {
            content_type: Some("application/octet-stream".to_string()),
            note: Some("whitepaper-meta".to_string()),
        });

        let encoded = encode_bundle_payload_canonical(&payload).expect("encode");
        let decoded = decode_bundle_payload_canonical(&encoded).expect("decode");
        assert_eq!(decoded.meta, payload.meta);
    }

    #[test]
    fn rejects_unsupported_abi_versions() {
        let mut payload = sample_payload();
        payload.v = BUNDLE_ABI_MIN_SUPPORTED.saturating_sub(1);
        let encoded = encode_bundle_payload_canonical(&payload).expect("encode");
        let err = decode_bundle_payload_canonical(&encoded).expect_err("must reject");
        assert!(matches!(err, BundleCodecError::UnsupportedVersion(0)));

        payload.v = BUNDLE_ABI_CURRENT.saturating_add(1);
        let encoded = encode_bundle_payload_canonical(&payload).expect("encode");
        let err = decode_bundle_payload_canonical(&encoded).expect_err("must reject");
        assert!(
            matches!(err, BundleCodecError::UnsupportedVersion(v) if v == BUNDLE_ABI_CURRENT + 1)
        );
    }
}
