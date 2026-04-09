//! Cryptographic primitives for the edgerun protocol.
//!
//! Uses ECDSA P-256 with SHA-256 for signatures and SHA-256 for content hashing.
//! SHA-256 is universally supported by TPMs, YubiKeys, Android Keystore,
//! iOS Secure Enclave, and hardware accelerators on virtually every platform.

use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};

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
pub const HASH_DOMAIN_STORED_REPRESENTATION_HEADER: &str = "edgerun:v0:hash:stored-representation-header";
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
// Core primitives
// ---------------------------------------------------------------------------

// ===========================================================================
// Inline SHA-256 (FIPS 180-4)
// ===========================================================================

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        let o = i * 4;
        w[i] = u32::from_be_bytes([block[o], block[o+1], block[o+2], block[o+3]]);
    }
    for i in 16..64 {
        let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
        let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
        w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
    }
    let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h] = *state;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA256_K[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h=g; g=f; f=e; e=d.wrapping_add(t1);
        d=c; c=b; b=a; a=t1.wrapping_add(t2);
    }
    state[0]=state[0].wrapping_add(a); state[1]=state[1].wrapping_add(b);
    state[2]=state[2].wrapping_add(c); state[3]=state[3].wrapping_add(d);
    state[4]=state[4].wrapping_add(e); state[5]=state[5].wrapping_add(f);
    state[6]=state[6].wrapping_add(g); state[7]=state[7].wrapping_add(h);
}

/// Compute SHA-256. Returns 32-byte digest.
pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let len = data.len();
    let mut i = 0;
    while i + 64 <= len {
        let mut block = [0u8; 64];
        block.copy_from_slice(&data[i..i+64]);
        sha256_compress(&mut state, &block);
        i += 64;
    }
    let mut block = [0u8; 64];
    let remain = len - i;
    block[..remain].copy_from_slice(&data[i..]);
    block[remain] = 0x80;
    if remain >= 56 {
        sha256_compress(&mut state, &block);
        block = [0u8; 64];
    }
    let bit_len = (len as u64) * 8;
    block[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sha256_compress(&mut state, &block);
    let mut out = Vec::with_capacity(32);
    for i in 0..8 { out.extend_from_slice(&state[i].to_be_bytes()); }
    out
}

/// Incremental SHA-256 hasher — replaces `sha2::Sha256::new()` + `.update()` + `.finalize()`.
pub struct Sha256Hasher {
    state: [u32; 8],
    buf: Vec<u8>,
    len: usize,
}

impl Sha256Hasher {
    pub fn new() -> Self {
        Self {
            state: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19],
            buf: Vec::with_capacity(128),
            len: 0,
        }
    }
    pub fn extend_from_slice(&mut self, data: &[u8]) {
        self.update(data);
    }
    pub fn update(&mut self, data: impl AsRef<[u8]>) {
        self.buf.extend_from_slice(data.as_ref());
        self.len += data.as_ref().len();
        while self.buf.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&self.buf[..64]);
            sha256_compress(&mut self.state, &block);
            self.buf.drain(..64);
        }
    }
    pub fn finalize(mut self) -> Vec<u8> {
        let len = self.len;
        let remain = self.buf.len();
        self.buf.resize(64, 0);
        self.buf[remain] = 0x80;
        if remain >= 56 {
            sha256_compress(&mut self.state, &self.buf[..64].try_into().unwrap());
            self.buf = vec![0u8; 64];
        }
        let bit_len = (len as u64) * 8;
        self.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
        sha256_compress(&mut self.state, &self.buf[..64].try_into().unwrap());
        let mut out = Vec::with_capacity(32);
        for i in 0..8 { out.extend_from_slice(&self.state[i].to_be_bytes()); }
        out
    }
}


/// HKDF-SHA256 (RFC 5869) — replaces `hkdf::Hkdf<Sha256>`.
/// Extract-then-Expand construction.
pub struct HkdfSha256 {
    prk: Vec<u8>, // pseudorandom key from extract phase
}

impl HkdfSha256 {
    /// Create HKDF from salt and input key material.
    /// Equivalent to `crate::crypto::HkdfSha256::new(salt, ikm)`.
    pub fn new(salt: Option<&[u8]>, ikm: &[u8]) -> Self {
        let salt = salt.unwrap_or(&[]);
        // Extract: PRK = HMAC-SHA256(salt, IKM)
        let prk = hmac_sha256(salt, ikm);
        Self { prk }
    }

    /// Expand the PRK to `length` bytes.
    /// Equivalent to `hkdf.expand(info, length)`.
    pub fn expand(&self, info: &[u8], length: usize) -> Vec<u8> {
        let hash_len = 32;
        let n = (length + hash_len - 1) / hash_len; // ceil(length / 32)
        let mut okm = Vec::with_capacity(n * hash_len);
        let mut prev = Vec::new();
        for i in 1..=n {
            let mut h = Sha256Hasher::new();
            if !prev.is_empty() {
                h.update(&prev);
            }
            h.update(info);
            h.update(&[i as u8]);
            prev = h.finalize();
            okm.extend_from_slice(&prev);
        }
        okm.truncate(length);
        okm
    }
}

/// HMAC-SHA256 (RFC 2104).
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let block_size = 64;
    // If key > block size, hash it first
    let key = if key.len() > block_size {
        sha256(key)
    } else {
        key.to_vec()
    };
    // Pad key to block size
    let mut k = vec![0u8; block_size];
    k[..key.len()].copy_from_slice(&key);
    // Inner pad and outer pad
    let mut ipad = k.clone();
    let mut opad = k;
    for i in 0..block_size {
        ipad[i] ^= 0x36;
        opad[i] ^= 0x5c;
    }
    // HMAC = H(opad || H(ipad || message))
    let mut inner = Sha256Hasher::new();
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256Hasher::new();
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    outer.finalize()
}

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
///
/// This is the shared utility used by all signature verification paths
/// in the codebase — replaces duplicated code in validators.rs,
/// command_dispatch.rs, session.rs, stream/lib.rs, and mesh/lib.rs.
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
    let r = p256::FieldBytes::from_slice(&signature_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&signature_bytes[32..]);
    let ecdsa_sig = match p256::ecdsa::Signature::from_scalars(*r, *s) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    vk.verify_prehash(digest, &ecdsa_sig).is_ok()
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
/// Per spec §17.14: `object_id = SHA256("edgerun:v0:object" || 0x00 || canonicalization_id || 0x00 || canonical_bytes)`
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
/// Per spec §17.15: `representation_digest = SHA256("edgerun:v0:representation-bytes" || 0x00 || stored_bytes)`
pub fn derive_representation_digest(stored_bytes: &[u8]) -> Vec<u8> {
    domain_hash(REPRESENTATION_DOMAIN, stored_bytes)
}

/// Derives a chunk digest from raw chunk bytes.
/// Per spec §17.16: `chunk_digest = SHA256("edgerun:v0:chunk-bytes" || 0x00 || chunk_bytes)`
pub fn derive_chunk_digest(chunk_bytes: &[u8]) -> Vec<u8> {
    domain_hash(CHUNK_DOMAIN, chunk_bytes)
}

/// Derives an identity identifier from identity record canonical bytes.
/// Recommended: `identity_id = SHA256("edgerun:v0:id:identity" || 0x00 || canonical_bytes)`
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
        let sig = sign_record(&signing, "edgerun:v0:sig:test", &record_hash);
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
        let sig = sign_record(&signing, "edgerun:v0:sig:test-a", &record_hash);
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

// ===========================================================================
// Inline SHA-384 (FIPS 180-4)
// ===========================================================================

// SHA-384 and SHA-512 share the same round constants (FIPS 180-4)
const SHA512_K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

#[inline]
fn sha384_512_k() -> &'static [u64; 80] {
    &SHA512_K
}

const SHA384_K: [u64; 80] = SHA512_K;

fn sha384_compress(state: &mut [u64; 8], block: &[u8; 128]) {
    let mut w = [0u64; 80];
    for i in 0..16 {
        let o = i * 8;
        w[i] = u64::from_be_bytes([block[o],block[o+1],block[o+2],block[o+3],block[o+4],block[o+5],block[o+6],block[o+7]]);
    }
    for i in 16..80 {
        let s0 = w[i-15].rotate_right(1) ^ w[i-15].rotate_right(8) ^ (w[i-15] >> 7);
        let s1 = w[i-2].rotate_right(19) ^ w[i-2].rotate_right(61) ^ (w[i-2] >> 6);
        w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
    }
    let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h] = *state;
    for i in 0..80 {
        let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA384_K[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h=g; g=f; f=e; e=d.wrapping_add(t1);
        d=c; c=b; b=a; a=t1.wrapping_add(t2);
    }
    state[0]=state[0].wrapping_add(a); state[1]=state[1].wrapping_add(b);
    state[2]=state[2].wrapping_add(c); state[3]=state[3].wrapping_add(d);
    state[4]=state[4].wrapping_add(e); state[5]=state[5].wrapping_add(f);
    state[6]=state[6].wrapping_add(g); state[7]=state[7].wrapping_add(h);
}

/// Compute SHA-384. Returns 48-byte digest.
pub fn sha384(data: &[u8]) -> Vec<u8> {
    let mut state: [u64; 8] = [
        0xcbbb9d5dc1059ed8, 0x629a292a367cd507, 0x9159015a3070dd17,
        0x152fecd8f70e5939, 0x67332667ffc00b31, 0x8eb44a8768581511,
        0xdb0c2e0d64f98fa7, 0x47b5481dbefa4fa4,
    ];
    let len = data.len();
    let mut i = 0;
    while i + 128 <= len {
        let mut block = [0u8; 128];
        block.copy_from_slice(&data[i..i+128]);
        sha384_compress(&mut state, &block);
        i += 128;
    }
    let mut block = [0u8; 128];
    let remain = len - i;
    block[..remain].copy_from_slice(&data[i..]);
    block[remain] = 0x80;
    if remain >= 112 {
        sha384_compress(&mut state, &block);
        block = [0u8; 128];
    }
    let bit_len = (len as u64) * 8;
    block[120..128].copy_from_slice(&bit_len.to_be_bytes());
    sha384_compress(&mut state, &block);
    let mut out = Vec::with_capacity(48);
    for i in 0..6 { out.extend_from_slice(&state[i].to_be_bytes()); }
    out
}

// ===========================================================================
// Inline SHA-512 (FIPS 180-4)
// ===========================================================================

// SHA512_K is defined above (shared with SHA-384)

fn sha512_compress(state: &mut [u64; 8], block: &[u8; 128]) {
    let mut w = [0u64; 80];
    for i in 0..16 {
        let o = i * 8;
        w[i] = u64::from_be_bytes([block[o],block[o+1],block[o+2],block[o+3],block[o+4],block[o+5],block[o+6],block[o+7]]);
    }
    for i in 16..80 {
        let s0 = w[i-15].rotate_right(1) ^ w[i-15].rotate_right(8) ^ (w[i-15] >> 7);
        let s1 = w[i-2].rotate_right(19) ^ w[i-2].rotate_right(61) ^ (w[i-2] >> 6);
        w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
    }
    let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h] = *state;
    for i in 0..80 {
        let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
        let ch = (e & f) ^ ((!e) & g);
        let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(SHA512_K[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h=g; g=f; f=e; e=d.wrapping_add(t1);
        d=c; c=b; b=a; a=t1.wrapping_add(t2);
    }
    state[0]=state[0].wrapping_add(a); state[1]=state[1].wrapping_add(b);
    state[2]=state[2].wrapping_add(c); state[3]=state[3].wrapping_add(d);
    state[4]=state[4].wrapping_add(e); state[5]=state[5].wrapping_add(f);
    state[6]=state[6].wrapping_add(g); state[7]=state[7].wrapping_add(h);
}

/// Compute SHA-512. Returns 64-byte digest.
pub fn sha512(data: &[u8]) -> Vec<u8> {
    let mut state: [u64; 8] = [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1, 0x510e527fade682d1, 0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ];
    let len = data.len();
    let mut i = 0;
    while i + 128 <= len {
        let mut block = [0u8; 128];
        block.copy_from_slice(&data[i..i+128]);
        sha512_compress(&mut state, &block);
        i += 128;
    }
    let mut block = [0u8; 128];
    let remain = len - i;
    block[..remain].copy_from_slice(&data[i..]);
    block[remain] = 0x80;
    if remain >= 112 {
        sha512_compress(&mut state, &block);
        block = [0u8; 128];
    }
    let bit_len = (len as u64) * 8;
    block[120..128].copy_from_slice(&bit_len.to_be_bytes());
    sha512_compress(&mut state, &block);
    let mut out = Vec::with_capacity(64);
    for i in 0..8 { out.extend_from_slice(&state[i].to_be_bytes()); }
    out
}

// ===========================================================================
// Random number generation — reads from /dev/urandom, no external crates needed
// ===========================================================================

/// Fill a buffer with cryptographically secure random bytes.
/// Uses /dev/urandom on Unix-like systems. No libc dependency.
pub fn fill_random(buf: &mut [u8]) {
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open("/dev/urandom")
        .expect("failed to open /dev/urandom");
    file.read_exact(buf)
        .expect("failed to read from /dev/urandom");
}

// ===========================================================================
// RNG wrapper for ECDH EphemeralSecret::random
// ===========================================================================

/// A cryptographically secure RNG backed by /dev/urandom.
/// Implements both `CryptoRng` and `RngCore` for use with p256's ECDH API.
pub struct DevUrandomRng;

impl rand_core::CryptoRng for DevUrandomRng {}

impl rand_core::RngCore for DevUrandomRng {
    fn next_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        fill_random(&mut buf);
        u32::from_le_bytes(buf)
    }

    fn next_u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        fill_random(&mut buf);
        u64::from_le_bytes(buf)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        fill_random(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        fill_random(dest);
        Ok(())
    }
}
