// SPDX-License-Identifier: Apache-2.0
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn blake3_256(bytes: &[u8]) -> [u8; 32] {
    blake3::hash(bytes).into()
}

/// Compute identity hash for deterministic execution bundles.
/// This must be computed over canonical bundle payload bytes only.
pub fn compute_bundle_hash(bundle_payload_bytes: &[u8]) -> [u8; 32] {
    blake3_256(bundle_payload_bytes)
}

pub fn sign(sk: &SigningKey, message: &[u8]) -> Signature {
    sk.sign(message)
}

pub fn verify(pk: &VerifyingKey, message: &[u8], sig: &Signature) -> bool {
    pk.verify(message, sig).is_ok()
}
