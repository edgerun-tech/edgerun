//! Cryptographic primitives for the edgerun protocol.
//!
//! Uses ECDSA P-256 with SHA-256 for signatures and SHA-256 for content hashing.
//! SHA-256 is universally supported by TPMs, YubiKeys, Android Keystore,
//! iOS Secure Enclave, and hardware accelerators on virtually every platform.
//!
//! All raw crypto primitives come from `edgerun-crypto`; this module provides
//! the protocol-specific layer: domain tags, sign/verify helpers, and
//! identity derivation functions.

use crate::prelude::v1::*;

// ---------------------------------------------------------------------------
// Protocol algorithm constants (matching protobuf enum wire values)
// ---------------------------------------------------------------------------

/// Signature algorithm: ECDSA P-256 with SHA-256.
/// Matches `Signature.Algorithm.SIGNATURE_ALGORITHM_ECDSA_P256_SHA256 = 1`.
pub const SIGNATURE_ALGORITHM_ECDSA_P256: u8 = 1;

/// Digest algorithm: SHA-256.
/// Matches `Digest.Algorithm.DIGEST_ALGORITHM_SHA256 = 1`.
pub const DIGEST_ALGORITHM_SHA256: u8 = 1;

/// Identity kind: node identity.
/// Matches `IdentityKind.IDENTITY_KIND_NODE = 2`.
pub const IDENTITY_KIND_NODE: i32 = 2;

/// SEC1 uncompressed elliptic curve point prefix.
pub const SEC1_UNCOMPRESSED_PREFIX: u8 = 0x04;

/// ECDSA P-256 signature size in bytes (r=32 + s=32).
pub const ECDSA_P256_SIGNATURE_LEN: usize = 64;

/// ECDSA P-256 public key size in raw bytes (x=32 + y=32).
pub const ECDSA_P256_PUBLIC_KEY_LEN: usize = 64;

// ---------------------------------------------------------------------------
// Re-export raw crypto primitives from the single crypto boundary
// ---------------------------------------------------------------------------
pub use edgerun_crypto::digest;
pub use edgerun_crypto::hkdf;
pub use edgerun_crypto::hkdf::Hkdf;
pub use edgerun_crypto::hmac;
pub use edgerun_crypto::p256;
pub use edgerun_crypto::p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
pub use edgerun_crypto::p256::ecdsa::{Signature, SigningKey, VerifyingKey};
pub use edgerun_crypto::rand_core;
pub use edgerun_crypto::sha2::Digest as Sha2Digest;
pub use edgerun_crypto::sha2::{Sha256, Sha384, Sha512};
pub use edgerun_crypto::{hkdf_sha256, hmac_sha256, hmac_sha384, random_p256_signing_key};

// ---------------------------------------------------------------------------
// Convenience wrappers — drop-in replacements for old inline implementations
// ---------------------------------------------------------------------------

/// Compute SHA-256. Returns 32-byte digest.
pub fn sha256(data: &[u8]) -> Vec<u8> {
    use edgerun_crypto::sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Compute SHA-384. Returns 48-byte digest.
pub fn sha384(data: &[u8]) -> Vec<u8> {
    use edgerun_crypto::sha2::Digest;
    let mut hasher = Sha384::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Compute SHA-512. Returns 64-byte digest.
pub fn sha512(data: &[u8]) -> Vec<u8> {
    use edgerun_crypto::sha2::Digest;
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// HKDF-SHA256 wrapper.
pub struct HkdfSha256 {
    inner: Hkdf<Sha256>,
}

impl HkdfSha256 {
    pub fn new(salt: Option<&[u8]>, ikm: &[u8]) -> Self {
        let (_prk, hkdf) = Hkdf::<Sha256>::extract(salt, ikm);
        // We need the full HKDF object, not just the PRK.
        // Use expand_from_info on the extracted hkdf directly.
        Self { inner: hkdf }
    }

    pub fn expand(&self, info: &[u8], length: usize) -> Vec<u8> {
        let mut okm = vec![0u8; length];
        self.inner
            .expand(info, &mut okm)
            .expect("HKDF expand failed");
        okm
    }
}

// ---------------------------------------------------------------------------
// Domain-separated hash tags (spec §17.9)
// ---------------------------------------------------------------------------

/// Domain tag for hashing an event envelope.
pub const HASH_DOMAIN_EVENT_ENVELOPE: &str = "edgerun:v0:hash:event-envelope";
/// Domain tag for hashing a command envelope.
pub const HASH_DOMAIN_COMMAND_ENVELOPE: &str = "edgerun:v0:hash:command-envelope";
/// Domain tag for hashing a delegation record.
pub const HASH_DOMAIN_DELEGATION_RECORD: &str = "edgerun:v0:hash:delegation-record";
/// Domain tag for hashing a revocation record.
pub const HASH_DOMAIN_REVOCATION_RECORD: &str = "edgerun:v0:hash:revocation-record";
/// Domain tag for hashing a snapshot descriptor.
pub const HASH_DOMAIN_SNAPSHOT_DESCRIPTOR: &str = "edgerun:v0:hash:snapshot-descriptor";
/// Domain tag for hashing a query result fragment.
pub const HASH_DOMAIN_QUERY_RESULT_FRAGMENT: &str = "edgerun:v0:hash:query-result-fragment";
/// Domain tag for hashing a route advertisement.
pub const HASH_DOMAIN_ROUTE_ADVERTISEMENT: &str = "edgerun:v0:hash:route-advertisement";
/// Domain tag for hashing a session hello.
pub const HASH_DOMAIN_SESSION_HELLO: &str = "edgerun:v0:hash:session-hello";
/// Domain tag for hashing a session accept.
pub const HASH_DOMAIN_SESSION_ACCEPT: &str = "edgerun:v0:hash:session-accept";
/// Domain tag for hashing a relay envelope.
pub const HASH_DOMAIN_RELAY_ENVELOPE: &str = "edgerun:v0:hash:relay-envelope";
/// Domain tag for hashing an identity record.
pub const HASH_DOMAIN_IDENTITY_RECORD: &str = "edgerun:v0:hash:identity-record";
/// Domain tag for hashing an assurance claim.
pub const HASH_DOMAIN_ASSURANCE_CLAIM: &str = "edgerun:v0:hash:assurance-claim";
/// Domain tag for hashing a logical object descriptor.
pub const HASH_DOMAIN_LOGICAL_OBJECT_DESCRIPTOR: &str = "edgerun:v0:hash:logical-object-descriptor";
/// Domain tag for hashing a stored representation header.
pub const HASH_DOMAIN_STORED_REPRESENTATION_HEADER: &str =
    "edgerun:v0:hash:stored-representation-header";
/// Domain tag for hashing a chunk manifest.
pub const HASH_DOMAIN_CHUNK_MANIFEST: &str = "edgerun:v0:hash:chunk-manifest";

// ---------------------------------------------------------------------------
// Domain-separated signature tags (spec §17.9)
// ---------------------------------------------------------------------------

/// Domain tag for signing an event envelope.
pub const SIG_DOMAIN_EVENT_ENVELOPE: &str = "edgerun:v0:sig:event-envelope";
/// Domain tag for signing a command envelope.
pub const SIG_DOMAIN_COMMAND_ENVELOPE: &str = "edgerun:v0:sig:command-envelope";
/// Domain tag for signing a delegation record.
pub const SIG_DOMAIN_DELEGATION_RECORD: &str = "edgerun:v0:sig:delegation-record";
/// Domain tag for signing a revocation record.
pub const SIG_DOMAIN_REVOCATION_RECORD: &str = "edgerun:v0:sig:revocation-record";
/// Domain tag for signing a snapshot descriptor.
pub const SIG_DOMAIN_SNAPSHOT_DESCRIPTOR: &str = "edgerun:v0:sig:snapshot-descriptor";
/// Domain tag for signing a query result fragment.
pub const SIG_DOMAIN_QUERY_RESULT_FRAGMENT: &str = "edgerun:v0:sig:query-result-fragment";
/// Domain tag for signing a route advertisement.
pub const SIG_DOMAIN_ROUTE_ADVERTISEMENT: &str = "edgerun:v0:sig:route-advertisement";
/// Domain tag for signing a session hello.
pub const SIG_DOMAIN_SESSION_HELLO: &str = "edgerun:v0:sig:session-hello";
/// Domain tag for signing a session accept.
pub const SIG_DOMAIN_SESSION_ACCEPT: &str = "edgerun:v0:sig:session-accept";
/// Domain tag for signing a relay envelope.
pub const SIG_DOMAIN_RELAY_ENVELOPE: &str = "edgerun:v0:sig:relay-envelope";
/// Domain tag for signing an identity record.
pub const SIG_DOMAIN_IDENTITY_RECORD: &str = "edgerun:v0:sig:identity-record";
/// Domain tag for signing an assurance claim.
pub const SIG_DOMAIN_ASSURANCE_CLAIM: &str = "edgerun:v0:sig:assurance-claim";
/// Domain tag for signing a mesh frame.
pub const SIG_DOMAIN_MESH_FRAME: &str = "edgerun:v0:sig:mesh-frame";
/// Domain tag for signing a query request.
pub const SIG_DOMAIN_QUERY_REQUEST: &str = "edgerun:v0:sig:query-request";

// ---------------------------------------------------------------------------
// Object identity derivation tags (spec §17.14–17.16)
// ---------------------------------------------------------------------------

/// Domain tag for deriving a logical object identity.
pub const OBJECT_ID_DOMAIN: &str = "edgerun:v0:object";
/// Domain tag for deriving a stored representation digest.
pub const REPRESENTATION_DOMAIN: &str = "edgerun:v0:representation-bytes";
/// Domain tag for deriving a chunk digest.
pub const CHUNK_DOMAIN: &str = "edgerun:v0:chunk-bytes";
/// Domain tag for deriving an identity identifier.
pub const IDENTITY_ID_DOMAIN: &str = "edgerun:v0:id:identity";

// ---------------------------------------------------------------------------
// Protocol-specific crypto helpers
// ---------------------------------------------------------------------------

/// Domain-separated hash: `SHA-256(domain_tag || 0x00 || payload)`.
pub fn domain_hash(domain_tag: &str, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(domain_tag.len() + 1 + payload.len());
    buf.extend_from_slice(domain_tag.as_bytes());
    buf.push(0);
    buf.extend_from_slice(payload);
    sha256(&buf)
}

/// Verify an ECDSA P-256 signature over a pre-computed SHA-256 digest.
///
/// Takes the raw 64-byte public key (x || y without 0x04 prefix),
/// the 32-byte digest, and the 64-byte signature (r || s).
pub fn verify_ecdsa_p256_raw(
    public_key_bytes: &[u8; 64],
    digest: &[u8; 32],
    signature_bytes: &[u8; 64],
) -> bool {
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(public_key_bytes);
    let vk = match p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let r_bytes: [u8; 32] = signature_bytes[..32].try_into().unwrap();
    let s_bytes: [u8; 32] = signature_bytes[32..].try_into().unwrap();
    let ecdsa_sig = match p256::ecdsa::Signature::from_scalars(r_bytes, s_bytes) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    vk.verify_prehash(digest, &ecdsa_sig).is_ok()
}

/// Build the signature input: `sig_domain_tag || 0x00 || record_hash`.
pub fn signature_input(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    let mut input = Vec::with_capacity(sig_domain_tag.len() + 1 + record_hash.len());
    input.extend_from_slice(sig_domain_tag.as_bytes());
    input.push(0);
    input.extend_from_slice(record_hash);
    input
}

/// Sign the signature input (domain_tag || 0x00 || record_hash) using
/// ECDSA P-256 with SHA-256 via the signing key's prehash signer.
///
/// Returns the raw 64-byte signature (r || s), or an error if signing fails.
pub fn sign_record(
    private_key: &SigningKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, p256::ecdsa::Error> {
    let sig_input = signature_input(sig_domain_tag, record_hash);
    let sig: Signature = private_key.sign_prehash(&sig_input)?;
    Ok(sig.to_bytes().to_vec())
}

/// Verify an ECDSA P-256 signature over the signature input.
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

/// Sign canonical bytes with domain separation.
///
/// Computes `SHA-256(sig_domain_tag || 0x00 || SHA-256(canonical_bytes))` and signs it.
pub fn sign_canonical_record(
    private_key: &SigningKey,
    sig_domain_tag: &str,
    canonical_bytes: &[u8],
) -> Result<Vec<u8>, p256::ecdsa::Error> {
    let record_hash = sha256(canonical_bytes);
    sign_record(private_key, sig_domain_tag, &record_hash)
}

/// Verify a signature over canonical bytes with domain separation.
///
/// Per spec §17.11: sig_input = sig_domain_tag || 0x00 || record_hash_bytes
/// then ECDSA_P256_SHA256_verify(public_key, sig_input, signature).
///
/// This verifies signatures produced by spec-compliant signing (ECDSA P-256 with
/// SHA-256, which signs the message directly). This matches the signing done
/// by `sign_record` in this module.
///
/// For hardware signers that require 32-byte pre-hash (which hash sig_input
/// internally), use verify_canonical_record_hw() instead.
pub fn verify_canonical_record(
    public_key: &VerifyingKey,
    sig_domain_tag: &str,
    canonical_bytes: &[u8],
    signature: &[u8],
) -> bool {
    let record_hash = sha256(canonical_bytes);
    let sig_input = signature_input(sig_domain_tag, &record_hash);
    let Ok(sig) = Signature::from_slice(signature) else {
        return false;
    };
    public_key.verify_prehash(&sig_input, &sig).is_ok()
}

/// Verify a signature produced by hardware signers that require 32-byte pre-hash.
///
/// Hardware signers (TPM, YubiKey, etc.) that implement the `sign_digest` interface
/// internally hash the message to 32 bytes before signing. This verification function
/// expects signatures over SHA-256(sig_input), not sig_input directly.
///
/// Use this for verifying signatures from hardware-backed MeshSigner implementations.
pub fn verify_canonical_record_hw(
    public_key: &VerifyingKey,
    sig_domain_tag: &str,
    canonical_bytes: &[u8],
    signature: &[u8],
) -> bool {
    let record_hash = sha256(canonical_bytes);
    let sig_input = signature_input(sig_domain_tag, &record_hash);
    let sig_input_digest = sha256(&sig_input);
    let Ok(sig) = Signature::from_slice(signature) else {
        return false;
    };
    let digest_array: [u8; 32] = match sig_input_digest.try_into() {
        Ok(d) => d,
        Err(_) => return false,
    };
    public_key.verify_prehash(&digest_array, &sig).is_ok()
}

/// Convert a verifying key to the 64-byte NodeID format (x || y without 0x04).
pub fn verifying_key_to_node_id(vk: &VerifyingKey) -> [u8; 64] {
    let encoded = vk.to_encoded_point(false);
    let bytes = encoded.as_bytes();
    let mut node_id = [0u8; 64];
    node_id.copy_from_slice(&bytes[1..65]);
    node_id
}

/// Convert a 64-byte NodeID (x || y) back to a VerifyingKey.
pub fn node_id_to_verifying_key(node_id: &[u8; 64]) -> Option<VerifyingKey> {
    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(node_id);
    VerifyingKey::from_sec1_bytes(&sec1).ok()
}

/// Derive a logical object identity from canonicalization ID and canonical bytes.
/// Per spec §17.14: `SHA-256("edgerun:v0:object" || 0x00 || canon_id || 0x00 || canonical_bytes)`
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

/// Derive a stored representation digest from raw stored bytes.
/// Per spec §17.15: `SHA-256("edgerun:v0:representation-bytes" || 0x00 || stored_bytes)`
pub fn derive_representation_digest(stored_bytes: &[u8]) -> Vec<u8> {
    domain_hash(REPRESENTATION_DOMAIN, stored_bytes)
}

/// Derive a chunk digest from raw chunk bytes.
/// Per spec §17.16: `SHA-256("edgerun:v0:chunk-bytes" || 0x00 || chunk_bytes)`
pub fn derive_chunk_digest(chunk_bytes: &[u8]) -> Vec<u8> {
    domain_hash(CHUNK_DOMAIN, chunk_bytes)
}

/// Derive an identity identifier from identity record canonical bytes.
/// `SHA-256("edgerun:v0:id:identity" || 0x00 || canonical_bytes)`
pub fn derive_identity_id(canonical_bytes: &[u8]) -> Vec<u8> {
    domain_hash(IDENTITY_ID_DOMAIN, canonical_bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signing_key() -> SigningKey {
        let bytes: [u8; 32] = [7u8; 32];
        SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    #[test]
    fn sha256_known_value() {
        // SHA-256 of empty string
        let digest = sha256(b"");
        assert_eq!(
            digest,
            edgerun_encoding::hex::hex_to_bytes(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            )
            .unwrap()
        );
    }

    #[test]
    fn sha256_deterministic() {
        let a = sha256(b"hello");
        let b = sha256(b"hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn hmac_sha256_known_value() {
        // RFC 4231 Test Case 2: key="Jefe", data="what do ya want for nothing?"
        let key = b"Jefe";
        let data = b"what do ya want for nothing?";
        let mac = hmac_sha256(key, data);
        assert_eq!(
            mac,
            edgerun_encoding::hex::hex_to_bytes(
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
            )
            .unwrap()
        );
    }

    #[test]
    fn hkdf_sha256_expand() {
        let hkdf = HkdfSha256::new(Some(b"salt"), b"input key material");
        let okm = hkdf.expand(b"info", 42);
        assert_eq!(okm.len(), 42);
        // Deterministic
        let hkdf2 = HkdfSha256::new(Some(b"salt"), b"input key material");
        let okm2 = hkdf2.expand(b"info", 42);
        assert_eq!(okm, okm2);
    }

    #[test]
    fn domain_hash_changes_with_domain() {
        let a = domain_hash("edgerun:v0:a", b"payload");
        let b = domain_hash("edgerun:v0:b", b"payload");
        assert_ne!(a, b);
    }

    #[test]
    fn signature_input_is_domain_separated() {
        let record_hash = vec![1; 32];
        let a = signature_input("edgerun:v0:sig:a", &record_hash);
        let b = signature_input("edgerun:v0:sig:b", &record_hash);
        assert_ne!(a, b);
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let record_hash = vec![2; 32];
        let sig = sign_record(&signing, "edgerun:v0:sig:test", &record_hash).unwrap();
        assert_eq!(sig.len(), 64);
        assert!(verify_record(
            &verifying,
            "edgerun:v0:sig:test",
            &record_hash,
            &sig
        ));
    }

    #[test]
    fn verify_rejects_wrong_domain() {
        let signing = test_signing_key();
        let verifying = signing.verifying_key();
        let record_hash = vec![3; 32];
        let sig = sign_record(&signing, "edgerun:v0:sig:test-a", &record_hash).unwrap();
        assert!(!verify_record(
            &verifying,
            "edgerun:v0:sig:test-b",
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
        assert_eq!(
            verifying.to_encoded_point(false),
            vk2.to_encoded_point(false)
        );
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
