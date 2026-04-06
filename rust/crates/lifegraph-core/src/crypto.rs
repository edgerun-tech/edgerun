//! Cryptographic primitives for the Lifegraph protocol.
//!
//! Uses ECDSA P-256 with SHA-256 for signatures and SHA-256 for content hashing.
//! SHA-256 is universally supported by TPMs, YubiKeys, Android Keystore,
//! iOS Secure Enclave, and hardware accelerators on virtually every platform.

use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Domain-separated hash tags (spec §17.9)
// ---------------------------------------------------------------------------

/// Domain tag for hashing an event envelope.
pub const HASH_DOMAIN_EVENT_ENVELOPE: &str = "lifegraph:v0:hash:event-envelope";
/// Domain tag for hashing a command envelope.
pub const HASH_DOMAIN_COMMAND_ENVELOPE: &str = "lifegraph:v0:hash:command-envelope";
/// Domain tag for hashing a delegation record.
pub const HASH_DOMAIN_DELEGATION_RECORD: &str = "lifegraph:v0:hash:delegation-record";
/// Domain tag for hashing a revocation record.
pub const HASH_DOMAIN_REVOCATION_RECORD: &str = "lifegraph:v0:hash:revocation-record";
/// Domain tag for hashing a snapshot descriptor.
pub const HASH_DOMAIN_SNAPSHOT_DESCRIPTOR: &str = "lifegraph:v0:hash:snapshot-descriptor";
/// Domain tag for hashing a query result fragment.
pub const HASH_DOMAIN_QUERY_RESULT_FRAGMENT: &str = "lifegraph:v0:hash:query-result-fragment";
/// Domain tag for hashing a route advertisement.
pub const HASH_DOMAIN_ROUTE_ADVERTISEMENT: &str = "lifegraph:v0:hash:route-advertisement";
/// Domain tag for hashing a session hello.
pub const HASH_DOMAIN_SESSION_HELLO: &str = "lifegraph:v0:hash:session-hello";
/// Domain tag for hashing a session accept.
pub const HASH_DOMAIN_SESSION_ACCEPT: &str = "lifegraph:v0:hash:session-accept";
/// Domain tag for hashing a relay envelope.
pub const HASH_DOMAIN_RELAY_ENVELOPE: &str = "lifegraph:v0:hash:relay-envelope";
/// Domain tag for hashing an identity record.
pub const HASH_DOMAIN_IDENTITY_RECORD: &str = "lifegraph:v0:hash:identity-record";
/// Domain tag for hashing an assurance claim.
pub const HASH_DOMAIN_ASSURANCE_CLAIM: &str = "lifegraph:v0:hash:assurance-claim";
/// Domain tag for hashing a logical object descriptor.
pub const HASH_DOMAIN_LOGICAL_OBJECT_DESCRIPTOR: &str = "lifegraph:v0:hash:logical-object-descriptor";
/// Domain tag for hashing a stored representation header.
pub const HASH_DOMAIN_STORED_REPRESENTATION_HEADER: &str = "lifegraph:v0:hash:stored-representation-header";
/// Domain tag for hashing a chunk manifest.
pub const HASH_DOMAIN_CHUNK_MANIFEST: &str = "lifegraph:v0:hash:chunk-manifest";

// ---------------------------------------------------------------------------
// Domain-separated signature tags (spec §17.9)
// ---------------------------------------------------------------------------

/// Domain tag for signing an event envelope.
pub const SIG_DOMAIN_EVENT_ENVELOPE: &str = "lifegraph:v0:sig:event-envelope";
/// Domain tag for signing a command envelope.
pub const SIG_DOMAIN_COMMAND_ENVELOPE: &str = "lifegraph:v0:sig:command-envelope";
/// Domain tag for signing a delegation record.
pub const SIG_DOMAIN_DELEGATION_RECORD: &str = "lifegraph:v0:sig:delegation-record";
/// Domain tag for signing a revocation record.
pub const SIG_DOMAIN_REVOCATION_RECORD: &str = "lifegraph:v0:sig:revocation-record";
/// Domain tag for signing a snapshot descriptor.
pub const SIG_DOMAIN_SNAPSHOT_DESCRIPTOR: &str = "lifegraph:v0:sig:snapshot-descriptor";
/// Domain tag for signing a query result fragment.
pub const SIG_DOMAIN_QUERY_RESULT_FRAGMENT: &str = "lifegraph:v0:sig:query-result-fragment";
/// Domain tag for signing a route advertisement.
pub const SIG_DOMAIN_ROUTE_ADVERTISEMENT: &str = "lifegraph:v0:sig:route-advertisement";
/// Domain tag for signing a session hello.
pub const SIG_DOMAIN_SESSION_HELLO: &str = "lifegraph:v0:sig:session-hello";
/// Domain tag for signing a session accept.
pub const SIG_DOMAIN_SESSION_ACCEPT: &str = "lifegraph:v0:sig:session-accept";
/// Domain tag for signing a relay envelope.
pub const SIG_DOMAIN_RELAY_ENVELOPE: &str = "lifegraph:v0:sig:relay-envelope";
/// Domain tag for signing an identity record.
pub const SIG_DOMAIN_IDENTITY_RECORD: &str = "lifegraph:v0:sig:identity-record";
/// Domain tag for signing an assurance claim.
pub const SIG_DOMAIN_ASSURANCE_CLAIM: &str = "lifegraph:v0:sig:assurance-claim";

// ---------------------------------------------------------------------------
// Object identity derivation tags (spec §17.14–17.16)
// ---------------------------------------------------------------------------

/// Domain tag for deriving a logical object identity.
pub const OBJECT_ID_DOMAIN: &str = "lifegraph:v0:object";
/// Domain tag for deriving a stored representation digest.
pub const REPRESENTATION_DOMAIN: &str = "lifegraph:v0:representation-bytes";
/// Domain tag for deriving a chunk digest.
pub const CHUNK_DOMAIN: &str = "lifegraph:v0:chunk-bytes";
/// Domain tag for deriving an identity identifier.
pub const IDENTITY_ID_DOMAIN: &str = "lifegraph:v0:id:identity";

// ---------------------------------------------------------------------------
// Core primitives
// ---------------------------------------------------------------------------

pub fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

pub fn domain_hash(domain_tag: &str, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(domain_tag.len() + 1 + payload.len());
    buf.extend_from_slice(domain_tag.as_bytes());
    buf.push(0);
    buf.extend_from_slice(payload);
    sha256(&buf)
}

pub fn signature_input(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    let mut input = Vec::with_capacity(sig_domain_tag.len() + 1 + record_hash.len());
    input.extend_from_slice(sig_domain_tag.as_bytes());
    input.push(0);
    input.extend_from_slice(record_hash);
    input
}

/// Signs the signature input (domain_tag || 0x00 || record_hash) using
/// ECDSA P-256 with SHA-256 via the signing key's prehash signer.
///
/// Returns the raw 64-byte signature (r || s).
pub fn sign_record(private_key: &SigningKey, sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    let sig_input = signature_input(sig_domain_tag, record_hash);
    let sig: Signature = private_key.sign_prehash(&sig_input).unwrap();
    sig.to_bytes().to_vec()
}

/// Verifies an ECDSA P-256 signature over the signature input.
pub fn verify_record(
    public_key: &VerifyingKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
    signature: &[u8],
) -> bool {
    let Ok(sig) = Signature::from_slice(signature) else {
        return false;
    };
    public_key
        .verify_prehash(&signature_input(sig_domain_tag, record_hash), &sig)
        .is_ok()
}

/// Converts a verifying key to the 64-byte NodeID format (x || y without 0x04).
pub fn verifying_key_to_node_id(vk: &VerifyingKey) -> [u8; 64] {
    let encoded = vk.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    let mut node_id = [0u8; 64];
    node_id.copy_from_slice(&bytes[1..65]); // skip 0x04 prefix
    node_id
}

/// Converts a 64-byte NodeID (x || y) back to a VerifyingKey.
pub fn node_id_to_verifying_key(node_id: &[u8; 64]) -> Option<VerifyingKey> {
    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(node_id);
    VerifyingKey::from_sec1_bytes(&sec1).ok()
}

/// Derives a logical object identity from canonicalization ID and canonical bytes.
/// Per spec §17.14: `object_id = SHA256("lifegraph:v0:object" || 0x00 || canonicalization_id || 0x00 || canonical_bytes)`
pub fn derive_object_id(canonicalization_id: &[u8], canonical_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(
        OBJECT_ID_DOMAIN.len() + 1 + canonicalization_id.len() + 1 + canonical_bytes.len(),
    );
    buf.extend_from_slice(OBJECT_ID_DOMAIN.as_bytes());
    buf.push(0);
    buf.extend_from_slice(canonicalization_id);
    buf.push(0);
    buf.extend_from_slice(canonical_bytes);
    sha256(&buf)
}

/// Derives a stored representation digest from raw stored bytes.
/// Per spec §17.15: `representation_digest = SHA256("lifegraph:v0:representation-bytes" || 0x00 || stored_bytes)`
pub fn derive_representation_digest(stored_bytes: &[u8]) -> Vec<u8> {
    domain_hash(REPRESENTATION_DOMAIN, stored_bytes)
}

/// Derives a chunk digest from raw chunk bytes.
/// Per spec §17.16: `chunk_digest = SHA256("lifegraph:v0:chunk-bytes" || 0x00 || chunk_bytes)`
pub fn derive_chunk_digest(chunk_bytes: &[u8]) -> Vec<u8> {
    domain_hash(CHUNK_DOMAIN, chunk_bytes)
}

/// Derives an identity identifier from identity record canonical bytes.
/// Recommended: `identity_id = SHA256("lifegraph:v0:id:identity" || 0x00 || canonical_bytes)`
pub fn derive_identity_id(canonical_bytes: &[u8]) -> Vec<u8> {
    domain_hash(IDENTITY_ID_DOMAIN, canonical_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signing_key() -> SigningKey {
        // Deterministic key for testing
        let bytes: [u8; 32] = [7u8; 32];
        SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    #[test]
    fn domain_hash_changes_with_domain() {
        let a = domain_hash("lifegraph:v0:a", b"payload");
        let b = domain_hash("lifegraph:v0:b", b"payload");
        assert_ne!(a, b);
    }

    #[test]
    fn signature_input_is_domain_separated() {
        let record_hash = vec![1; 32];
        let a = signature_input("lifegraph:v0:sig:a", &record_hash);
        let b = signature_input("lifegraph:v0:sig:b", &record_hash);
        assert_ne!(a, b);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let record_hash = vec![2; 32];
        let sig = sign_record(&signing, "lifegraph:v0:sig:test", &record_hash);
        assert_eq!(sig.len(), 64);
        assert!(verify_record(
            &verifying,
            "lifegraph:v0:sig:test",
            &record_hash,
            &sig
        ));
    }

    #[test]
    fn verify_rejects_wrong_domain() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let record_hash = vec![3; 32];
        let sig = sign_record(&signing, "lifegraph:v0:sig:test-a", &record_hash);
        assert!(!verify_record(
            &verifying,
            "lifegraph:v0:sig:test-b",
            &record_hash,
            &sig
        ));
    }

    #[test]
    fn node_id_roundtrip() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let node_id = verifying_key_to_node_id(&verifying);
        let vk2 = node_id_to_verifying_key(&node_id).unwrap();
        assert_eq!(verifying.to_encoded_point(false), vk2.to_encoded_point(false));
    }

    #[test]
    fn derive_object_id_is_deterministic() {
        let canon_id = b"canonical-v0";
        let canon_bytes = b"some canonical data";
        let a = derive_object_id(canon_id, canon_bytes);
        let b = derive_object_id(canon_id, canon_bytes);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn derive_object_id_changes_with_canon_id() {
        let data = b"same data";
        let a = derive_object_id(b"canonical-v0", data);
        let b = derive_object_id(b"canonical-v1", data);
        assert_ne!(a, b);
    }

    #[test]
    fn derive_object_id_changes_with_canonical_bytes() {
        let canon_id = b"canonical-v0";
        let a = derive_object_id(canon_id, b"data-a");
        let b = derive_object_id(canon_id, b"data-b");
        assert_ne!(a, b);
    }

    #[test]
    fn derive_representation_digest_is_deterministic() {
        let data = b"stored representation bytes";
        let a = derive_representation_digest(data);
        let b = derive_representation_digest(data);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn derive_representation_digest_differs_from_raw_sha256() {
        let data = b"some data";
        let domain_hashed = derive_representation_digest(data);
        let raw = sha256(data);
        assert_ne!(domain_hashed, raw);
    }

    #[test]
    fn derive_chunk_digest_is_deterministic() {
        let chunk = b"chunk bytes here";
        let a = derive_chunk_digest(chunk);
        let b = derive_chunk_digest(chunk);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn derive_identity_id_is_deterministic() {
        let canon = b"identity record canonical";
        let a = derive_identity_id(canon);
        let b = derive_identity_id(canon);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }
}
