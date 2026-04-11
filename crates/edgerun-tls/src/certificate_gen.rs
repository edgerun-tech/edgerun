//! X.509 certificate generation using `rcgen` from edgerun-crypto.
//!
//! Generates ECDSA P-256 self-signed certificates with proper DER encoding.
//! No hand-rolled DER assembly — `rcgen` produces RFC 5280-compliant certificates.

use edgerun_crypto::p256::pkcs8::DecodePrivateKey;
use edgerun_crypto::rcgen::{generate_simple_self_signed, CertifiedKey, KeyPair};

/// A self-signed certificate with an associated ECDSA P-256 signing key.
pub struct CertificateAndKey {
    /// DER-encoded X.509 certificate
    pub cert_der: Vec<u8>,
    /// ECDSA P-256 signing key (for CertificateVerify during TLS handshake)
    pub signing_key: edgerun_crypto::p256::ecdsa::SigningKey,
}

/// Generate a self-signed certificate for the given hostname(s).
///
/// Uses `rcgen` to produce a properly DER-encoded X.509 v3 certificate
/// with ECDSA P-256 key, SAN extensions, and 1-year validity.
pub fn generate_self_signed(hostnames: &[&str]) -> CertificateAndKey {
    if hostnames.is_empty() {
        panic!("At least one hostname is required");
    }

    let subject_alt_names: Vec<String> = hostnames.iter().map(|h| h.to_string()).collect();
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(subject_alt_names)
            .expect("rcgen: failed to generate self-signed certificate");

    // Extract DER bytes from rcgen Certificate
    let cert_der = cert.der().to_vec();

    // Convert rcgen KeyPair to p256::ecdsa::SigningKey via PKCS#8
    let signing_key = extract_signing_key(&signing_key);

    CertificateAndKey { cert_der, signing_key }
}

/// Convert an rcgen KeyPair to a p256::ecdsa::SigningKey.
///
/// rcgen's KeyPair stores the private key in PKCS#8 DER format.
/// We deserialize it into the p256 signing key type.
fn extract_signing_key(key_pair: &KeyPair) -> edgerun_crypto::p256::ecdsa::SigningKey {
    let pkcs8_der = key_pair.serialize_der();
    edgerun_crypto::p256::ecdsa::SigningKey::from_pkcs8_der(&pkcs8_der)
        .expect("rcgen ECDSA P-256 key should be valid PKCS#8")
}

