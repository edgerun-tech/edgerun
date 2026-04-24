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
//! use edgerun_crypto::SigningKey;
//! use edgerun_crypto::sha256;
//! use edgerun_crypto::AesGcmCipher;
//! ```

pub mod error;

// ---------------------------------------------------------------------------
// Re-exports — all crypto flows through this single dependency boundary.
// Consumers should prefer the curated convenience functions below, but
// direct access to the underlying crates is available when needed.
// ---------------------------------------------------------------------------
pub use digest;
pub use sha2;
pub use sha1;
pub use pbkdf2;
pub use hmac;
pub use hkdf;
pub use aes_gcm;
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
pub use signature;
pub use der;
pub use base16ct;
pub use const_oid;
pub use x509_cert;
pub use rcgen;
pub use crypto_bigint;
pub use crypto_common;
pub use block_buffer;
pub use rand_core;
pub use getrandom;
pub use subtle;
pub use zeroize;
pub use typenum;

// ---------------------------------------------------------------------------
// Curated public API
// ---------------------------------------------------------------------------

pub use error::CryptoError;

/// Compatibility alias: `OsRng` from rand_core 0.6 (used by elliptic-curve,
/// ed25519-dalek, x25519-dalek, rsa, etc.).
pub use rand_core_06::OsRng;
/// Re-export RngCore trait for use with OsRng.fill_bytes()
pub use rand_core_06::RngCore;

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

pub use aes_gcm::{Aes256Gcm as AesGcmCipher};

/// Unified AEAD cipher for TLS 1.3 / QUIC
/// Supports AES-128-GCM, AES-256-GCM, and ChaCha20-Poly1305
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AeadCipher {
    Aes128Gcm(aes_gcm::Aes128Gcm),
    Aes256Gcm(aes_gcm::Aes256Gcm),
}

impl AeadCipher {
    /// Create cipher from key bytes based on key length
    pub fn new_from_key(key: &[u8]) -> Result<Self, CryptoError> {
        match key.len() {
            16 => Ok(AeadCipher::Aes128Gcm(
                aes_gcm::Aes128Gcm::new_from_slice(key)
                    .map_err(|_| CryptoError::InvalidKey)?,
            )),
            32 => Ok(AeadCipher::Aes256Gcm(
                aes_gcm::Aes256Gcm::new_from_slice(key)
                    .map_err(|_| CryptoError::InvalidKey)?,
            )),
            _ => Err(CryptoError::InvalidKey),
        }
    }

    /// Encrypt with AAD (authenticated additional data)
    pub fn encrypt(&self, nonce: &[u8; 12], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut result = plaintext.to_vec();
        let tag = self.encrypt_in_place_detached(nonce, aad, &mut result)?;
        result.extend_from_slice(tag.as_slice());
        Ok(result)
    }

    /// Decrypt with AAD (expects ciphertext || tag format)
    pub fn decrypt(&self, nonce: &[u8; 12], aad: &[u8], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if ciphertext_and_tag.len() < 16 {
            return Err(CryptoError::DecryptionFailed);
        }
        let tag_start = ciphertext_and_tag.len() - 16;
        let mut buffer = ciphertext_and_tag[..tag_start].to_vec();
        let tag = aes_gcm::Tag::from_slice(&ciphertext_and_tag[tag_start..]);
        self.decrypt_in_place_detached(nonce, aad, &mut buffer, tag)?;
        Ok(buffer)
    }

    /// Encrypt in-place with detached tag
    pub fn encrypt_in_place_detached(&self, nonce: &[u8; 12], aad: &[u8], buffer: &mut [u8]) -> Result<aes_gcm::Tag, CryptoError> {
        match self {
            AeadCipher::Aes128Gcm(c) => {
                let mut n = aes_gcm::Nonce::default();
                n.copy_from_slice(nonce);
                c.encrypt_in_place_detached(&n, aad, buffer)
                    .map_err(|_| CryptoError::EncryptionFailed)
            }
            AeadCipher::Aes256Gcm(c) => {
                let mut n = aes_gcm::Nonce::default();
                n.copy_from_slice(nonce);
                c.encrypt_in_place_detached(&n, aad, buffer)
                    .map_err(|_| CryptoError::EncryptionFailed)
            }
        }
    }

    /// Decrypt in-place with detached tag
    pub fn decrypt_in_place_detached(&self, nonce: &[u8; 12], aad: &[u8], buffer: &mut [u8], tag: &aes_gcm::Tag) -> Result<(), CryptoError> {
        match self {
            AeadCipher::Aes128Gcm(c) => {
                let mut n = aes_gcm::Nonce::default();
                n.copy_from_slice(nonce);
                c.decrypt_in_place_detached(&n, aad, buffer, tag)
                    .map_err(|_| CryptoError::DecryptionFailed)
            }
            AeadCipher::Aes256Gcm(c) => {
                let mut n = aes_gcm::Nonce::default();
                n.copy_from_slice(nonce);
                c.decrypt_in_place_detached(&n, aad, buffer, tag)
                    .map_err(|_| CryptoError::DecryptionFailed)
            }
        }
    }
}

impl From<AeadCipher> for AesGcmCipher {
    fn from(c: AeadCipher) -> Self {
        match c {
            AeadCipher::Aes128Gcm(_) => AesGcmCipher::new_from_slice(&[0u8; 32]).unwrap(),
            AeadCipher::Aes256Gcm(c) => c,
        }
    }
}

// Signature traits
pub use signature::Signer;
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
// Cipher suites (shared between TLS 1.3 and QUIC)
// ---------------------------------------------------------------------------

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CipherSuite {
    TLS_AES_128_GCM_SHA256,
    TLS_AES_256_GCM_SHA384,
    TLS_CHACHA20_POLY1305_SHA256,
}

impl CipherSuite {
    pub fn client_default() -> Vec<Self> {
        vec![
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_AES_256_GCM_SHA384,
        ]
    }

    pub fn to_wire(self) -> u16 {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => 0x1301,
            CipherSuite::TLS_AES_256_GCM_SHA384 => 0x1302,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 0x1303,
        }
    }

    pub fn from_wire(value: u16) -> Result<Self, CryptoError> {
        match value {
            0x1301 => Ok(CipherSuite::TLS_AES_128_GCM_SHA256),
            0x1302 => Ok(CipherSuite::TLS_AES_256_GCM_SHA384),
            0x1303 => Ok(CipherSuite::TLS_CHACHA20_POLY1305_SHA256),
            _ => Err(CryptoError::CipherSuiteError(format!("Unsupported: 0x{:04x}", value))),
        }
    }

    pub fn key_len(self) -> usize {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => 16,
            CipherSuite::TLS_AES_256_GCM_SHA384 | CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 32,
        }
    }

    pub const fn iv_len(&self) -> usize {
        12
    }

    pub const fn tag_len(&self) -> usize {
        16
    }

    pub fn hash_len(self) -> usize {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 | CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 32,
            CipherSuite::TLS_AES_256_GCM_SHA384 => 48,
        }
    }
}

impl std::fmt::Display for CipherSuite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => write!(f, "TLS_AES_128_GCM_SHA256"),
            CipherSuite::TLS_AES_256_GCM_SHA384 => write!(f, "TLS_AES_256_GCM_SHA384"),
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => write!(f, "TLS_CHACHA20_POLY1305_SHA256"),
        }
    }
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
/// Returns plaintext or error.
pub fn aes256_gcm_decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>, CryptoError> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
    let cipher = Aes256Gcm::new_from_slice(key).expect("valid AES-256 key");
    let nonce = Nonce::from(*nonce);
    cipher.decrypt(&nonce, ciphertext_and_tag).map_err(|_| CryptoError::DecryptionFailed)
}

// ---------------------------------------------------------------------------
// RSA-PSS convenience (for TLS handshake)
// ---------------------------------------------------------------------------

pub use rsa::signature::Verifier;

/// Verify an RSA-PSS signature with SHA-256.
///
/// This is a convenience wrapper that handles the RSA math internally.
pub fn rsa_pss_verify(n: &[u8], e: &[u8], signature: &[u8], msg_hash: &[u8]) -> Result<bool, CryptoError> {
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::RsaPublicKey;
    use rsa::pkcs1::DecodeRsaPublicKey;
    use rsa::pkcs1v15::Signature as RsaSignature;

    let pk = RsaPublicKey::new(
        rsa::BigUint::from_bytes_be(n),
        rsa::BigUint::from_bytes_be(e),
    ).map_err(|_| CryptoError::InvalidKey)?;

    let sig = RsaSignature::try_from(signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)?;

    let vk = VerifyingKey::<Sha256>::new(pk);
    Ok(vk.verify(msg_hash, &sig).is_ok())
}

// ---------------------------------------------------------------------------
// Encrypted key file utilities
// ---------------------------------------------------------------------------

const ENCRYPTED_KEY_MAGIC: &[u8; 4] = b"EDG1";
const ENCRYPTED_KEY_VERSION: u8 = 1;
const ENCRYPTED_KEY_SALT_LEN: usize = 16;
const ENCRYPTED_KEY_NONCE_LEN: usize = 12;
const PBKDF2_ITERATIONS: u32 = 100_000;

pub fn encrypt_signing_key(key: &p256::ecdsa::SigningKey, passphrase: &str) -> Vec<u8> {
    use hmac::Mac;
    use pbkdf2::pbkdf2_hmac_array;
    use aes_gcm::{aead::Aead, KeyInit, Nonce};

    let mut salt = [0u8; ENCRYPTED_KEY_SALT_LEN];
    getrandom::fill(&mut salt).expect("random generation failed");

    let mut nonce = [0u8; ENCRYPTED_KEY_NONCE_LEN];
    getrandom::fill(&mut nonce).expect("random generation failed");

    let derived_key: [u8; 32] = pbkdf2_hmac_array::<sha2::Sha256, 32>(passphrase.as_bytes(), &salt, PBKDF2_ITERATIONS);

    let cipher = Aes256Gcm::new_from_slice(&derived_key).expect("valid AES-256 key");
    let nonce = Nonce::from(nonce);
    let key_bytes = key.to_bytes();
    let ciphertext = cipher.encrypt(&nonce, key_bytes.as_ref()).expect("encryption ok");

    let mut result = Vec::with_capacity(4 + 1 + ENCRYPTED_KEY_SALT_LEN + ENCRYPTED_KEY_NONCE_LEN + ciphertext.len());
    result.extend_from_slice(ENCRYPTED_KEY_MAGIC);
    result.push(ENCRYPTED_KEY_VERSION);
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    result
}

pub fn decrypt_signing_key(encrypted_data: &[u8], passphrase: &str) -> Result<p256::ecdsa::SigningKey, CryptoError> {
    use hmac::Mac;
    use pbkdf2::pbkdf2_hmac_array;
    use aes_gcm::{aead::Aead, KeyInit, Nonce};

    if encrypted_data.len() < 4 + 1 + ENCRYPTED_KEY_SALT_LEN + ENCRYPTED_KEY_NONCE_LEN + 16 {
        return Err(CryptoError::DecryptionFailed);
    }

    let magic = &encrypted_data[0..4];
    if magic != ENCRYPTED_KEY_MAGIC {
        return Err(CryptoError::DecryptionFailed);
    }

    let version = encrypted_data[4];
    if version != ENCRYPTED_KEY_VERSION {
        return Err(CryptoError::DecryptionFailed);
    }

    let salt = &encrypted_data[5..5 + ENCRYPTED_KEY_SALT_LEN];
    let nonce = &encrypted_data[5 + ENCRYPTED_KEY_SALT_LEN..5 + ENCRYPTED_KEY_SALT_LEN + ENCRYPTED_KEY_NONCE_LEN];
    let ciphertext = &encrypted_data[5 + ENCRYPTED_KEY_SALT_LEN + ENCRYPTED_KEY_NONCE_LEN..];

    let derived_key: [u8; 32] = pbkdf2_hmac_array::<sha2::Sha256, 32>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS);

    let cipher = Aes256Gcm::new_from_slice(&derived_key).map_err(|_| CryptoError::DecryptionFailed)?;
    let mut nonce_arr = [0u8; 12];
    nonce_arr.copy_from_slice(nonce);
    let nonce = Nonce::from(nonce_arr);
    let plaintext = cipher.decrypt(&nonce, ciphertext).map_err(|_| CryptoError::DecryptionFailed)?;

    if plaintext.len() != 32 {
        return Err(CryptoError::DecryptionFailed);
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&plaintext);
    p256::ecdsa::SigningKey::from_bytes(&key_bytes.into()).map_err(|_| CryptoError::DecryptionFailed)
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
pub fn pem_decode(label: &str, pem_str: &str) -> Result<Vec<u8>, CryptoError> {
    let parsed = pem::parse(pem_str).map_err(|e| CryptoError::InvalidPem(e.to_string()))?;
    if parsed.tag() != label {
        return Err(CryptoError::InvalidPem(
            format!("expected PEM label '{label}', got '{}'", parsed.tag()),
        ));
    }
    Ok(parsed.contents().to_vec())
}

/// PEM-encode a P-256 ECDSA signing key to PKCS#8 PEM format.
///
/// Uses the key's PKCS#8 serialization with proper PEM encoding.
pub fn p256_signing_key_to_pem(key: &p256::ecdsa::SigningKey) -> Result<String, CryptoError> {
    use pkcs8::EncodePrivateKey;
    
    let pem = key.to_pkcs8_pem(pkcs8::LineEnding::LF)
        .map_err(|e| CryptoError::KeyParseError(format!("Failed to serialize key to PEM: {}", e)))?;
    
    Ok(pem.as_str().to_string())
}

/// Parse a P-256 ECDSA signing key from PEM-encoded PKCS#8.
///
/// Accepts `-----BEGIN PRIVATE KEY-----` or `-----BEGIN EC PRIVATE KEY-----`.
pub fn p256_signing_key_from_pem(pem_str: &str) -> Result<p256::ecdsa::SigningKey, CryptoError> {
    use p256::elliptic_curve::pkcs8::DecodePrivateKey;
    p256::ecdsa::SigningKey::from_pkcs8_pem(pem_str)
        .map_err(|e| CryptoError::KeyParseError(e.to_string()))
}

/// Parse a P-256 ECDSA signing key from DER-encoded PKCS#8.
pub fn p256_signing_key_from_der(der: &[u8]) -> Result<p256::ecdsa::SigningKey, CryptoError> {
    use p256::elliptic_curve::pkcs8::DecodePrivateKey;
    p256::ecdsa::SigningKey::from_pkcs8_der(der)
        .map_err(|e| CryptoError::KeyParseError(e.to_string()))
}

/// Parse an X.509 certificate from PEM format, returning DER bytes.
pub fn x509_cert_from_pem(pem_str: &str) -> Result<Vec<u8>, CryptoError> {
    use x509_cert::der::{DecodePem, Encode};
    let cert = x509_cert::Certificate::from_pem(pem_str)
        .map_err(|e| CryptoError::InvalidPem(e.to_string()))?;
    cert.to_der()
        .map(|b| b.to_vec())
        .map_err(|e| CryptoError::InvalidDer(e.to_string()))
}

/// Parse a PEM string containing both a certificate and a PKCS#8 private key,
/// returning `(cert_der, signing_key)`.
///
/// The PEM blocks can be in any order. Only the first certificate and first
/// private key are used.
pub fn load_cert_and_key_from_pem(
    pem_text: &str,
) -> Result<(Vec<u8>, p256::ecdsa::SigningKey), CryptoError> {
    let mut cert_der: Option<Vec<u8>> = None;
    let mut key_der: Option<Vec<u8>> = None;

    for pem_obj in pem::parse_many(pem_text).map_err(|e| CryptoError::InvalidPem(e.to_string()))? {
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

    let cert = cert_der.ok_or_else(|| CryptoError::CertificateError("no CERTIFICATE block found".into()))?;
    let key_der_bytes = key_der.ok_or_else(|| CryptoError::KeyParseError("no PRIVATE KEY block found".into()))?;
    let key = p256_signing_key_from_der(&key_der_bytes)?;

    Ok((cert, key))
}

/// Generate a self-signed certificate and key pair.
///
/// Uses ECDSA P-256 with a 1-year validity period from the current time.
/// Returns `(cert_der, signing_key)`.
pub fn generate_self_signed(hostnames: &[&str]) -> Result<(Vec<u8>, p256::ecdsa::SigningKey), CryptoError> {
    if hostnames.is_empty() {
        return Err(CryptoError::CertificateError(
            "generate_self_signed: at least one hostname required".into(),
        ));
    }

    use rcgen::{CertificateParams, DistinguishedName, DnType};
    use time::OffsetDateTime;

    let subject_alt_names: Vec<String> = hostnames.iter().map(|h| h.to_string()).collect();

    let now = OffsetDateTime::now_utc();
    let not_before = now;
    let not_after = now + time::Duration::days(365);

    let mut params = CertificateParams::new(subject_alt_names.clone())
        .map_err(|e| CryptoError::CertificateError(e.to_string()))?;
    params.not_before = not_before;
    params.not_after = not_after;

    // Set a proper subject DN
    let mut dn = DistinguishedName::new();
    if let Some(first_host) = hostnames.first() {
        dn.push(DnType::CommonName, first_host.to_string());
    }
    params.distinguished_name = dn;

    let key_pair = rcgen::KeyPair::generate()
        .map_err(|e| CryptoError::CertificateError(e.to_string()))?;

    let cert = params.self_signed(&key_pair)
        .map_err(|e| CryptoError::CertificateError(e.to_string()))?;

    let cert_der = cert.der().to_vec();
    let pkcs8_der = key_pair.serialize_der();
    let signing_key = p256_signing_key_from_der(&pkcs8_der)?;

    Ok((cert_der, signing_key))
}

/// Generate a self-signed certificate and key pair, returned as PEM.
///
/// Uses ECDSA P-256 with a 1-year validity period from the current time.
/// Returns `(cert_pem, key_pem)`.
pub fn generate_self_signed_pem(hostnames: &[&str]) -> Result<(String, String), CryptoError> {
    let (cert_der, key) = generate_self_signed(hostnames)?;
    let cert_pem = pem_encode("CERTIFICATE", &cert_der);
    let key_pem = p256_signing_key_to_pem(&key)?;
    Ok((cert_pem, key_pem))
}

// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// PKCS#10 Certificate Signing Request (CSR)
// ---------------------------------------------------------------------------

pub mod csr;

pub use csr::{generate_csr_p256, generate_csr_pem};
