// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod control_plane;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTrustProfile {
    Strict,
    Balanced,
    Monitor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncTrustPolicy {
    pub profile: SyncTrustProfile,
    pub warn_risk: u32,
    pub max_risk: u32,
    pub block_revoked: bool,
    pub configured: bool,
}

impl SyncTrustPolicy {
    pub fn strict(configured: bool) -> Self {
        Self {
            profile: SyncTrustProfile::Strict,
            warn_risk: 40,
            max_risk: 60,
            block_revoked: true,
            configured,
        }
    }

    pub fn balanced(configured: bool) -> Self {
        Self {
            profile: SyncTrustProfile::Balanced,
            warn_risk: 70,
            max_risk: 90,
            block_revoked: true,
            configured,
        }
    }

    pub fn monitor(configured: bool) -> Self {
        Self {
            profile: SyncTrustProfile::Monitor,
            warn_risk: 70,
            max_risk: 100,
            block_revoked: false,
            configured,
        }
    }

    pub fn from_profile_name(profile: &str, configured: bool) -> Option<Self> {
        match profile.trim().to_ascii_lowercase().as_str() {
            "strict" => Some(Self::strict(configured)),
            "balanced" => Some(Self::balanced(configured)),
            "monitor" => Some(Self::monitor(configured)),
            _ => None,
        }
    }
}

impl Default for SyncTrustPolicy {
    fn default() -> Self {
        Self::balanced(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttestationClaim {
    pub measurement: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttestationPolicy {
    pub required: bool,
    pub max_age_secs: u64,
    #[serde(default)]
    pub allowed_measurements: Vec<String>,
}

impl Default for AttestationPolicy {
    fn default() -> Self {
        Self {
            required: false,
            max_age_secs: 300,
            allowed_measurements: Vec::new(),
        }
    }
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

    #[test]
    fn sync_trust_policy_profile_defaults_match_expected() {
        let strict = SyncTrustPolicy::strict(true);
        assert_eq!(strict.profile, SyncTrustProfile::Strict);
        assert_eq!(strict.warn_risk, 40);
        assert_eq!(strict.max_risk, 60);
        assert!(strict.block_revoked);
        assert!(strict.configured);

        let balanced = SyncTrustPolicy::balanced(true);
        assert_eq!(balanced.profile, SyncTrustProfile::Balanced);
        assert_eq!(balanced.warn_risk, 70);
        assert_eq!(balanced.max_risk, 90);
        assert!(balanced.block_revoked);
        assert!(balanced.configured);

        let monitor = SyncTrustPolicy::monitor(true);
        assert_eq!(monitor.profile, SyncTrustProfile::Monitor);
        assert_eq!(monitor.warn_risk, 70);
        assert_eq!(monitor.max_risk, 100);
        assert!(!monitor.block_revoked);
        assert!(monitor.configured);
    }

    #[test]
    fn sync_trust_policy_from_profile_name() {
        assert!(matches!(
            SyncTrustPolicy::from_profile_name("strict", true).map(|p| p.profile),
            Some(SyncTrustProfile::Strict)
        ));
        assert!(matches!(
            SyncTrustPolicy::from_profile_name("BALANCED", true).map(|p| p.profile),
            Some(SyncTrustProfile::Balanced)
        ));
        assert!(matches!(
            SyncTrustPolicy::from_profile_name("monitor", true).map(|p| p.profile),
            Some(SyncTrustProfile::Monitor)
        ));
        assert!(SyncTrustPolicy::from_profile_name("unknown", true).is_none());
    }

    #[test]
    fn attestation_policy_default_is_non_blocking() {
        let p = AttestationPolicy::default();
        assert!(!p.required);
        assert_eq!(p.max_age_secs, 300);
        assert!(p.allowed_measurements.is_empty());
    }
}
