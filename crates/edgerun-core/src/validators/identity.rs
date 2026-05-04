//! IdentityRecord structural and signature validation (spec §14.4, §17).
//!
//! An IdentityRecord binds a logical identity to a public key.
//! The signature proves that the holder of the private key issued the record.
//! This is a bootstrap trust primitive: the record's own public key
//! verifies its signature (self-signed).

use crate::prelude::v1::*;

use super::helpers::*;
use crate::crypto::{
    verify_canonical_record, ECDSA_P256_PUBLIC_KEY_LEN, ECDSA_P256_SIGNATURE_LEN,
    SIG_DOMAIN_IDENTITY_RECORD,
};
use crate::protocol::{canonical_bytes, ProtocolRecord};
use crate::result::{reject, ReasonCode, ValidationResult};
use crate::value::Value;
use std::collections::BTreeMap;

/// Validates the structural integrity of an IdentityRecord.
///
/// Checks:
/// - All required fields are present and non-empty
/// - identity_kind is a valid non-unspecified IdentityKind enum value
/// - key_algorithm is KEY_ALGORITHM_ECDSA_P256 (1)
/// - public_key has correct length for the algorithm
/// - signature has correct length
/// - Signature verifies against the record's own public_key (self-signed)
pub fn validate_identity_record(record: &crate::protocol::IdentityRecord) -> ValidationResult {
    use crate::protocol::{IdentityKind, ObjectKind};
    use crate::protocol::{IdentityRecord, KeyAlgorithm};

    // --- Required field checks ---
    if record.record_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            mapping([("reason", ystr("unsupported_record_version"))]),
            empty_map(),
        );
    }

    if record.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_identity_id"))]),
            empty_map(),
        );
    }

    if IdentityKind::from(record.identity_kind).is_none_or(|kind| kind == IdentityKind::Unspecified)
    {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("invalid_identity_kind"))]),
            empty_map(),
        );
    }

    // key_algorithm must be ECDSA_P256 (1)
    if record.key_algorithm != KeyAlgorithm::EcdsaP256 as i32 {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("unsupported_key_algorithm"))]),
            empty_map(),
        );
    }

    // public_key must be correct length for P-256 (64 bytes, compressed without 0x04 prefix)
    if record.public_key.len() != ECDSA_P256_PUBLIC_KEY_LEN {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("bad_public_key_length"))]),
            empty_map(),
        );
    }

    if record.created_at.is_none() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_created_at"))]),
            empty_map(),
        );
    }
    if record
        .created_at
        .as_ref()
        .is_some_and(|created_at| !(0..1_000_000_000).contains(&created_at.nanos))
    {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("invalid_created_at"))]),
            empty_map(),
        );
    }

    if let Some(identity) = &record.supersedes_identity {
        if identity.identity_id.is_empty()
            || identity.identity_kind.is_some_and(|identity_kind| {
                IdentityKind::from(identity_kind)
                    .is_none_or(|kind| kind == IdentityKind::Unspecified)
            })
        {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr("supersedes_identity_missing_identity_id"))]),
                empty_map(),
            );
        }
    }

    if record.assurance_claim_objects.iter().any(|object| {
        object.object_id.is_empty()
            || object.object_kind.is_some_and(|object_kind| {
                ObjectKind::from(object_kind).is_none_or(|kind| kind == ObjectKind::Unspecified)
            })
    }) {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("assurance_claim_object_missing_object_id"))]),
            empty_map(),
        );
    }

    if let Some(object) = &record.metadata_object {
        if object.object_id.is_empty()
            || object.object_kind.is_some_and(|object_kind| {
                ObjectKind::from(object_kind).is_none_or(|kind| kind == ObjectKind::Unspecified)
            })
        {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr("metadata_object_missing_object_id"))]),
                empty_map(),
            );
        }
    }

    // signature must be present and correct length
    let Some(ref sig) = record.signature else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_signature"))]),
            empty_map(),
        );
    };

    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("unsupported_signature_algorithm"))]),
            empty_map(),
        );
    }

    if sig.value.len() != ECDSA_P256_SIGNATURE_LEN {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("bad_signature_length"))]),
            empty_map(),
        );
    }

    // --- Signature verification ---
    // Reconstruct the public key from the record's public_key field.
    // The record is self-signed: the public key in the record verifies
    // the signature on the record itself.
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04; // Uncompressed point format
    vk_sec1[1..].copy_from_slice(&record.public_key);

    let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => {
            return reject(
                ReasonCode::CryptoInvalid,
                mapping([("reason", ystr("invalid_public_key"))]),
                empty_map(),
            );
        }
    };

    // Verify using domain-separated canonical path (spec §17)
    let canonical = canonical_bytes(&ProtocolRecord::IdentityRecord(record.clone()), true);

    if !verify_canonical_record(&vk, SIG_DOMAIN_IDENTITY_RECORD, &canonical, &sig.value) {
        return reject(
            ReasonCode::CryptoInvalid,
            mapping([("reason", ystr("signature_verification_failed"))]),
            empty_map(),
        );
    }

    // --- Accept ---
    accept(
        mapping([
            ("validation_level", ystr("identity_verified")),
            (
                "identity_id",
                ystr(crate::util::bytes_to_hex(&record.identity_id)),
            ),
        ]),
        empty_map(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::IdentityRecord;
    use crate::protocol::{IdentityKind, IdentityRef, ObjectKind, ObjectRef, Signature};

    fn make_test_keypair() -> (
        edgerun_crypto::p256::ecdsa::SigningKey,
        edgerun_crypto::p256::ecdsa::VerifyingKey,
        Vec<u8>,
    ) {
        use edgerun_crypto::p256::ecdsa::signature::SignerMut;
        let sk = edgerun_crypto::p256::ecdsa::SigningKey::random(&mut edgerun_crypto::OsRng);
        let vk = *sk.verifying_key();
        // Public key without 0x04 prefix (64 bytes)
        let sec1 = vk.to_encoded_point(false);
        let pk = sec1.as_bytes()[1..].to_vec();
        (sk, vk, pk)
    }

    fn sign_record(
        sk: &edgerun_crypto::p256::ecdsa::SigningKey,
        record: &IdentityRecord,
    ) -> IdentityRecord {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
        let canonical = canonical_bytes(&ProtocolRecord::IdentityRecord(record.clone()), true);
        let record_hash =
            crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_IDENTITY_RECORD, &canonical);
        let sig_input = crate::crypto::signature_input(SIG_DOMAIN_IDENTITY_RECORD, &record_hash);
        // Sign sig_input directly (per spec §17.11)
        let sig: edgerun_crypto::p256::ecdsa::Signature = sk.sign_prehash(&sig_input).unwrap();
        let sig_bytes = sig.to_bytes();

        let mut signed = record.clone();
        signed.signature = Some(Signature {
            algorithm: 1,
            value: sig_bytes.to_vec(),
        });
        signed
    }

    #[test]
    fn test_valid_identity_record() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1, // ECDSA_P256
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn test_reject_empty_identity_id() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_unsupported_record_version() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 2,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(
            result.reason_code,
            Some(crate::result::ReasonCode::VersionUnsupported)
        );
    }

    #[test]
    fn test_reject_unspecified_identity_kind() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: 0, // UNSPECIFIED
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_unknown_identity_kind() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: 999_999,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_unspecified_key_algorithm() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 0, // UNSPECIFIED
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_bad_public_key_length() {
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: vec![0u8; 32], // wrong length
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_missing_created_at() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: None,
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(
            result.reason_code,
            Some(crate::result::ReasonCode::StructuralInvalid)
        );
    }

    #[test]
    fn test_reject_invalid_created_at_timestamp() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 1_000_000_000,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn test_reject_missing_signature() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_bad_signature_algorithm() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: Some(Signature {
                algorithm: 999,
                value: vec![0u8; 64],
            }),
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(
            result.reason_code,
            Some(crate::result::ReasonCode::StructuralInvalid)
        );
    }

    #[test]
    fn test_reject_bad_signature_length() {
        let (_sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: Some(Signature {
                algorithm: 1,
                value: vec![0u8; 32], // wrong length
            }),
        };
        let result = validate_identity_record(&record);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_invalid_signature() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk.clone(),
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let mut signed = sign_record(&sk, &record);
        // Tamper with signature
        if let Some(ref mut sig) = signed.signature {
            sig.value[0] ^= 0xFF;
        }
        let result = validate_identity_record(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_reject_signature_from_different_key() {
        let (sk1, _vk1, pk1) = make_test_keypair();
        let (_sk2, _vk2, pk2) = make_test_keypair();

        // Sign with key 1 but put key 2 in the record
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk2, // different key
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk1, &record);
        let result = validate_identity_record(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
    }

    #[test]
    fn test_accept_with_node_identity_kind() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);
        let result = validate_identity_record(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn test_accept_with_other_identity_kind() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Other as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);
        let result = validate_identity_record(&signed);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn test_reject_empty_supersedes_identity() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: Some(IdentityRef {
                identity_id: vec![],
                identity_kind: Some(IdentityKind::Node as i32),
                key_hint: None,
            }),
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn test_reject_invalid_supersedes_identity_kind() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: Some(IdentityRef {
                identity_id: vec![6, 7, 8],
                identity_kind: Some(0),
                key_hint: None,
            }),
            assurance_claim_objects: vec![],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn test_reject_empty_assurance_claim_object() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![ObjectRef {
                object_id: vec![],
                object_kind: None,
            }],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn test_reject_invalid_assurance_claim_object_kind() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![ObjectRef {
                object_id: vec![9],
                object_kind: Some(ObjectKind::Unspecified as i32),
            }],
            metadata_object: None,
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn test_reject_empty_metadata_object() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: Some(ObjectRef {
                object_id: vec![],
                object_kind: None,
            }),
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn test_reject_unknown_metadata_object_kind() {
        let (sk, _vk, pk) = make_test_keypair();
        let record = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2, 3, 4, 5],
            identity_kind: IdentityKind::Node as i32,
            key_algorithm: 1,
            public_key: pk,
            created_at: Some(crate::protocol::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![],
            metadata_object: Some(ObjectRef {
                object_id: vec![10],
                object_kind: Some(999_999),
            }),
            signature: None,
        };
        let signed = sign_record(&sk, &record);

        let result = validate_identity_record(&signed);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }
}
