//! Cryptographic primitives for the Lifegraph protocol.
//!
//! Uses ECDSA P-256 with SHA-256 for signatures and SHA-256 for content hashing.
//! SHA-256 is universally supported by TPMs, YubiKeys, Android Keystore,
//! iOS Secure Enclave, and hardware accelerators on virtually every platform.

use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
use sha2::{Digest, Sha256};

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
}
