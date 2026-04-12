//! Central crypto crate for the edgerun project.
//!
//! Re-exports all cryptographic primitives used across the workspace
//! through a single dependency boundary.
//!
//! # Usage
//!
//! Instead of depending on `p256`, `sha2`, `aes-gcm`, `hkdf`, etc. directly,
//! consumer crates should depend on `edgerun-crypto` and import through it:
//! ```ignore
//! use edgerun_crypto::p256::ecdsa::SigningKey;
//! use edgerun_crypto::sha2::Sha256;
//! use edgerun_crypto::aes_gcm::Aes256Gcm;
//! ```

// ---------------------------------------------------------------------------
// Hash / digest
// ---------------------------------------------------------------------------
pub use digest;
pub use sha2;
pub use sha1;
pub use pbkdf2;

// ---------------------------------------------------------------------------
// MAC / KDF
// ---------------------------------------------------------------------------
pub use hmac;
pub use hkdf;

// ---------------------------------------------------------------------------
// Symmetric encryption (AEAD)
// ---------------------------------------------------------------------------
pub use aes_gcm;

// ---------------------------------------------------------------------------
// Elliptic curve / asymmetric
// ---------------------------------------------------------------------------
pub use p256;
pub use rsa;
pub use ed448_goldilocks;
pub use ecdsa;
pub use elliptic_curve;
pub use primeorder;
pub use sec1;
pub use ff;
pub use group;
pub use rfc6979;
pub use x25519_dalek;
pub use ed25519_dalek;
pub use curve25519_dalek;
pub use chacha20poly1305;

// ---------------------------------------------------------------------------
// Signature trait abstraction
// ---------------------------------------------------------------------------
pub use signature;

// ---------------------------------------------------------------------------
// Encoding / formats
// ---------------------------------------------------------------------------
pub use der;
pub use base16ct;
pub use const_oid;

// ---------------------------------------------------------------------------
// X.509 certificate generation / parsing
// ---------------------------------------------------------------------------
pub use x509_cert;
pub use rcgen;

// ---------------------------------------------------------------------------
// Core crypto primitives
// ---------------------------------------------------------------------------
pub use crypto_bigint;
pub use crypto_common;
pub use block_buffer;
pub use rand_core;
pub use getrandom;

/// Compatibility alias: `OsRng` from rand_core 0.6 (used by elliptic-curve,
/// ed25519-dalek, x25519-dalek, rsa, etc.).
pub use rand_core_06::OsRng;
/// Re-export RngCore trait for use with OsRng.fill_bytes()
pub use rand_core_06::RngCore;
pub use subtle;
pub use zeroize;
pub use typenum;

// ---------------------------------------------------------------------------
// Convenience re-exports for most-used types
// ---------------------------------------------------------------------------

// P-256 ECDSA
pub use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
pub use p256::ecdh::EphemeralSecret;
pub use p256::{PublicKey, EncodedPoint, FieldBytes};
pub use p256::elliptic_curve::sec1::ToEncodedPoint;

// RSA
pub use rsa::RsaPublicKey;
pub use rsa::pkcs1::DecodeRsaPublicKey;

// ED25519 (RFC 8080 DNSSEC algorithm 15)
pub use ed25519_dalek::{SigningKey as Ed25519SigningKey, VerifyingKey as Ed25519VerifyingKey, Signature as Ed25519Signature};

// ED448 (RFC 8080 DNSSEC algorithm 16)
pub use ed448_goldilocks::Signature as Ed448Signature;
pub use ed448_goldilocks::VerifyingKey as Ed448VerifyingKey;

// Hash
pub use sha2::{Digest, Sha256, Sha384, Sha512};

// MAC / KDF
pub use hmac::{Hmac, Mac};
pub use hkdf::Hkdf;

// AEAD
pub use aes_gcm::{
    aead::{Aead, AeadInPlace, KeyInit},
    Aes128Gcm, Aes256Gcm, Key, Nonce,
};

// Signature traits
pub use signature::{Signer, Verifier};
pub use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};

// DER
pub use der::Tag;

// ---------------------------------------------------------------------------
// Convenience hash functions
// ---------------------------------------------------------------------------

/// SHA-256 hash of `data`, returning 32 bytes.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// SHA-384 hash of `data`, returning 48 bytes.
pub fn sha384(data: &[u8]) -> [u8; 48] {
    use sha2::Digest;
    let mut hasher = sha2::Sha384::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// SHA-512 hash of `data`, returning 64 bytes.
pub fn sha512(data: &[u8]) -> [u8; 64] {
    use sha2::Digest;
    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    hasher.finalize().into()
}

// ---------------------------------------------------------------------------
// Convenience MAC / KDF functions
// ---------------------------------------------------------------------------

/// HMAC-SHA-256 of `msg` with `key`.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    use hmac::Mac;
    let mut mac = <hmac::Hmac<sha2::Sha256> as digest::KeyInit>::new_from_slice(key)
        .expect("HMAC key length ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// HMAC-SHA-384 of `msg` with `key`.
pub fn hmac_sha384(key: &[u8], msg: &[u8]) -> Vec<u8> {
    use hmac::Mac;
    let mut mac = <hmac::Hmac<sha2::Sha384> as digest::KeyInit>::new_from_slice(key)
        .expect("HMAC key length ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// HKDF-SHA-256 derive: expand keying material from salt + ikm.
///
/// Returns `okm_len` bytes of output keying material.
pub fn hkdf_sha256(salt: Option<&[u8]>, ikm: &[u8], info: &[u8], okm_len: usize) -> Vec<u8> {
    use hkdf::Hkdf;
    let hk = Hkdf::<sha2::Sha256>::new(salt, ikm);
    let mut okm = vec![0u8; okm_len];
    hk.expand(info, &mut okm).expect("HKDF expand ok");
    okm
}

// ---------------------------------------------------------------------------
// Convenience key generation
// ---------------------------------------------------------------------------

/// Generate a random P-256 ECDSA signing key.
pub fn random_p256_signing_key() -> p256::ecdsa::SigningKey {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("random generation failed");
    p256::ecdsa::SigningKey::from_bytes(&bytes.into()).unwrap()
}

// ---------------------------------------------------------------------------
// Convenience AEAD helpers
// ---------------------------------------------------------------------------

/// AES-256-GCM encrypt `plaintext` with `key` and random nonce.
/// Returns `(nonce, ciphertext_and_tag)`.
pub fn aes256_gcm_encrypt(key: &[u8; 32], plaintext: &[u8]) -> ([u8; 12], Vec<u8>) {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    let cipher = Aes256Gcm::new_from_slice(key).expect("valid AES-256 key");
    let mut nonce_bytes = [0u8; 12];
    getrandom::fill(&mut nonce_bytes).expect("random generation failed");
    let nonce = Nonce::from(nonce_bytes);
    let ct = cipher.encrypt(&nonce, plaintext).expect("encryption ok");
    (nonce_bytes, ct)
}

/// AES-256-GCM decrypt `ciphertext_and_tag` with `key` and `nonce`.
/// Returns plaintext or error string.
pub fn aes256_gcm_decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>, String> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    let cipher = Aes256Gcm::new_from_slice(key).expect("valid AES-256 key");
    let nonce = Nonce::from(*nonce);
    cipher.decrypt(&nonce, ciphertext_and_tag).map_err(|e| format!("decryption failed: {:?}", e))
}

// ---------------------------------------------------------------------------
// PEM / DER encoding utilities
// ---------------------------------------------------------------------------

/// PEM-encode arbitrary DER bytes with the given label.
///
/// Returns the PEM string including `-----BEGIN <label>-----` / `-----END <label>-----`.
pub fn pem_encode(label: &str, der: &[u8]) -> String {
    pem::encode(&pem::Pem::new(label, der.to_vec()))
}

/// Parse a PEM block by label, returning the raw DER bytes.
pub fn pem_decode(label: &str, pem_str: &str) -> Result<Vec<u8>, String> {
    let parsed = pem::parse(pem_str).map_err(|e| format!("invalid PEM: {e}"))?;
    if parsed.tag() != label {
        return Err(format!("expected PEM label '{label}', got '{}'", parsed.tag()));
    }
    Ok(parsed.contents().to_vec())
}

/// PEM-encode a P-256 ECDSA signing key to PKCS#8 PEM format.
///
/// Returns the PEM string including `-----BEGIN PRIVATE KEY-----` / `-----END PRIVATE KEY-----`.
pub fn p256_signing_key_to_pem(key: &p256::ecdsa::SigningKey) -> String {
    // Construct PKCS#8 DER wrapper for a P-256 EC key.
    // The PKCS#8 structure is:
    //   version (INTEGER 0)
    //   algorithm (SEQUENCE: id-ecPublicKey OID + namedCurve P-256 OID)
    //   private_key (OCTET STRING containing RFC 5915 ECPrivateKey)
    //
    // id-ecPublicKey = 1.2.840.10045.2.1  →  06 07 2a 86 48 ce 3d 02 01
    // prime256v1     = 1.2.840.10045.3.1.7 →  06 08 2a 86 48 ce 3d 03 01 07
    //
    // RFC 5915 ECPrivateKey:
    //   version (INTEGER 1)
    //   privateKey (OCTET STRING 32 bytes)
    //   parameters [0] EXPLICIT OID prime256v1
    //   publicKey  [1] EXPLICIT BIT STRING (uncompressed point, 65 bytes)

    let scalar_bytes = key.to_bytes();
    let verifying_point = key.verifying_key().to_encoded_point(false); // uncompressed
    let public_key_bytes = verifying_point.as_bytes(); // 65 bytes (0x04 + 32 + 32)

    // Build ECPrivateKey (RFC 5915)
    let mut ec_private = Vec::with_capacity(2 + 34 + 13 + 69);
    ec_private.push(0x02); ec_private.push(0x01); ec_private.push(0x01); // version = 1
    ec_private.push(0x04); ec_private.push(0x20); // OCTET STRING, len 32
    ec_private.extend_from_slice(&scalar_bytes);
    // [0] EXPLICIT OID prime256v1
    ec_private.push(0xa0); ec_private.push(0x0a);
    ec_private.push(0x06); ec_private.push(0x08);
    ec_private.extend_from_slice(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]);
    // [1] EXPLICIT BIT STRING (uncompressed point)
    ec_private.push(0xa1);
    ec_private.push(0x44); // 68 = 0x44 (65 bytes + 1 unused bits byte)
    ec_private.push(0x03); // BIT STRING
    ec_private.push(0x42); // len 66 (65 bytes data + 1 byte unused bits = 0)
    ec_private.push(0x00); // 0 unused bits
    ec_private.extend_from_slice(public_key_bytes);

    // Build AlgorithmIdentifier
    let alg_id: &[u8] = &[
        0x30, 0x13, // SEQUENCE, len 19
        0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, // id-ecPublicKey
        0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, // prime256v1
    ];

    // Build PrivateKeyInfo
    let mut pkcs8 = Vec::with_capacity(4 + alg_id.len() + ec_private.len() + 2);
    pkcs8.push(0x02); pkcs8.push(0x01); pkcs8.push(0x00); // version = 0
    pkcs8.extend_from_slice(alg_id);
    pkcs8.push(0x04); // OCTET STRING wrapping ECPrivateKey
    pkcs8.push(ec_private.len() as u8);
    pkcs8.extend_from_slice(&ec_private);

    pem_encode("PRIVATE KEY", &pkcs8)
}

/// Parse a P-256 ECDSA signing key from PEM-encoded PKCS#8.
///
/// Accepts `-----BEGIN PRIVATE KEY-----` or `-----BEGIN EC PRIVATE KEY-----`.
pub fn p256_signing_key_from_pem(pem_str: &str) -> Result<p256::ecdsa::SigningKey, String> {
    use p256::elliptic_curve::pkcs8::DecodePrivateKey;
    p256::ecdsa::SigningKey::from_pkcs8_pem(pem_str)
        .map_err(|e| format!("failed to parse P-256 PEM key: {e}"))
}

/// Parse a P-256 ECDSA signing key from DER-encoded PKCS#8.
pub fn p256_signing_key_from_der(der: &[u8]) -> Result<p256::ecdsa::SigningKey, String> {
    use p256::elliptic_curve::pkcs8::DecodePrivateKey;
    p256::ecdsa::SigningKey::from_pkcs8_der(der)
        .map_err(|e| format!("failed to parse P-256 DER key: {e}"))
}

/// Parse an X.509 certificate from PEM format, returning DER bytes.
pub fn x509_cert_from_pem(pem_str: &str) -> Result<Vec<u8>, String> {
    use x509_cert::der::{DecodePem, Encode};
    let cert = x509_cert::Certificate::from_pem(pem_str)
        .map_err(|e| format!("failed to parse PEM certificate: {e}"))?;
    cert.to_der()
        .map(|b| b.to_vec())
        .map_err(|e| format!("failed to re-encode cert to DER: {e}"))
}

/// Parse a PEM string containing both a certificate and a PKCS#8 private key,
/// returning `(cert_der, signing_key)`.
///
/// The PEM blocks can be in any order. Only the first certificate and first
/// private key are used.
pub fn load_cert_and_key_from_pem(
    pem_text: &str,
) -> Result<(Vec<u8>, p256::ecdsa::SigningKey), String> {
    let mut cert_der: Option<Vec<u8>> = None;
    let mut key_der: Option<Vec<u8>> = None;

    for pem_obj in pem::parse_many(pem_text).map_err(|e| format!("invalid PEM: {e}"))? {
        match pem_obj.tag() {
            "CERTIFICATE" => {
                cert_der = Some(pem_obj.contents().to_vec());
            }
            "PRIVATE KEY" | "EC PRIVATE KEY" => {
                key_der = Some(pem_obj.contents().to_vec());
            }
            _ => {}
        }
    }

    let cert = cert_der.ok_or("no CERTIFICATE block found")?;
    let key_der_bytes = key_der.ok_or("no PRIVATE KEY block found")?;
    let key = p256_signing_key_from_der(&key_der_bytes)?;

    Ok((cert, key))
}

/// Generate a self-signed certificate and key pair.
///
/// Uses ECDSA P-256 with a 1-year validity period.
/// Returns `(cert_der, signing_key)`.
pub fn generate_self_signed(hostnames: &[&str]) -> (Vec<u8>, p256::ecdsa::SigningKey) {
    if hostnames.is_empty() {
        panic!("generate_self_signed: at least one hostname required");
    }

    use rcgen::{generate_simple_self_signed, CertifiedKey, KeyPair};

    let subject_alt_names: Vec<String> = hostnames.iter().map(|h| h.to_string()).collect();
    let CertifiedKey { cert, signing_key: key_pair } =
        generate_simple_self_signed(subject_alt_names)
            .expect("rcgen: failed to generate self-signed certificate");

    let cert_der = cert.der().to_vec();
    let pkcs8_der = key_pair.serialize_der();
    let signing_key = p256_signing_key_from_der(&pkcs8_der)
        .expect("rcgen ECDSA P-256 key should be valid PKCS#8");

    (cert_der, signing_key)
}

/// Generate a self-signed certificate and key pair, returned as PEM.
///
/// Uses ECDSA P-256 with a 1-year validity period.
/// Returns `(cert_pem, key_pem)`.
pub fn generate_self_signed_pem(hostnames: &[&str]) -> (String, String) {
    let (cert_der, key) = generate_self_signed(hostnames);
    let cert_pem = pem_encode("CERTIFICATE", &cert_der);
    let key_pem = p256_signing_key_to_pem(&key);
    (cert_pem, key_pem)
}

// ---------------------------------------------------------------------------
// Unified AEAD cipher enum
// ---------------------------------------------------------------------------

/// Unified AEAD cipher supporting AES-128-GCM and AES-256-GCM.
///
/// Use instead of duplicating the `Aes128Gcm | Aes256Gcm` enum pattern across
/// crates (edgerun-tls, edgerun-http).
pub enum AesGcmCipher {
    Aes128Gcm(Aes128Gcm),
    Aes256Gcm(Aes256Gcm),
}

impl AesGcmCipher {
    /// Construct from a key. The key length determines the cipher: 16 bytes → AES-128, 32 bytes → AES-256.
    pub fn new_from_slice(key: &[u8]) -> Result<Self, String> {
        match key.len() {
            16 => {
                let k: [u8; 16] = key.try_into().map_err(|_| "invalid key slice")?;
                Ok(AesGcmCipher::Aes128Gcm(Aes128Gcm::new(&k.into())))
            }
            32 => {
                let k: [u8; 32] = key.try_into().map_err(|_| "invalid key slice")?;
                Ok(AesGcmCipher::Aes256Gcm(Aes256Gcm::new(&k.into())))
            }
            _ => Err(format!("unsupported key length: {} (expected 16 or 32)", key.len())),
        }
    }

    /// Encrypt plaintext with the given nonce. Returns ciphertext + tag.
    pub fn encrypt(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        use aes_gcm::aead::Aead;
        let nonce = Nonce::from(*nonce);
        match self {
            AesGcmCipher::Aes128Gcm(c) => c.encrypt(&nonce, plaintext).map_err(|e| format!("encrypt: {:?}", e)),
            AesGcmCipher::Aes256Gcm(c) => c.encrypt(&nonce, plaintext).map_err(|e| format!("encrypt: {:?}", e)),
        }
    }

    /// Decrypt ciphertext with the given nonce. Returns plaintext.
    pub fn decrypt(&self, nonce: &[u8; 12], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>, String> {
        use aes_gcm::aead::Aead;
        let nonce = Nonce::from(*nonce);
        match self {
            AesGcmCipher::Aes128Gcm(c) => c.decrypt(&nonce, ciphertext_and_tag).map_err(|e| format!("decrypt: {:?}", e)),
            AesGcmCipher::Aes256Gcm(c) => c.decrypt(&nonce, ciphertext_and_tag).map_err(|e| format!("decrypt: {:?}", e)),
        }
    }

    /// Encrypt in-place with detached tag. `buffer` is extended with ciphertext.
    /// Returns the tag. `aad` is additional authenticated data.
    pub fn encrypt_in_place_detached(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut Vec<u8>,
    ) -> Result<aes_gcm::Tag, String> {
        use aes_gcm::aead::AeadInPlace;
        use aes_gcm::Nonce as NonceInner;
        let nonce = NonceInner::from(*nonce);
        match self {
            AesGcmCipher::Aes128Gcm(c) => c.encrypt_in_place_detached(&nonce, aad, buffer)
                .map_err(|e| format!("encrypt: {:?}", e)),
            AesGcmCipher::Aes256Gcm(c) => c.encrypt_in_place_detached(&nonce, aad, buffer)
                .map_err(|e| format!("encrypt: {:?}", e)),
        }
    }

    /// Decrypt in-place with detached tag. Returns Ok(()) on success.
    pub fn decrypt_in_place_detached(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut Vec<u8>,
        tag: &aes_gcm::Tag,
    ) -> Result<(), String> {
        use aes_gcm::aead::AeadInPlace;
        use aes_gcm::Nonce as NonceInner;
        let nonce = NonceInner::from(*nonce);
        match self {
            AesGcmCipher::Aes128Gcm(c) => c.decrypt_in_place_detached(&nonce, aad, buffer, tag)
                .map_err(|e| format!("decrypt: {:?}", e)),
            AesGcmCipher::Aes256Gcm(c) => c.decrypt_in_place_detached(&nonce, aad, buffer, tag)
                .map_err(|e| format!("decrypt: {:?}", e)),
        }
    }
}

#[cfg(test)]
mod tests;
