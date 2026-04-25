//! X.509 certificate generation and PEM/DER conversion.
//!
//! Provides `CertificateAndKey` — a self-signed certificate with an
//! associated ECDSA P-256 signing key — plus PEM ↔ DER conversion utilities.

use std::sync::Arc;

use edgerun_crypto::{
    load_cert_and_key_from_pem, p256, p256_signing_key_from_der, p256_signing_key_from_pem,
    p256_signing_key_to_pem, pem_encode, x509_cert_from_pem, CryptoError,
};

/// A certificate with an associated ECDSA P-256 signing key.
///
/// This is the primary type for TLS server identity. The certificate
/// is DER-encoded X.509; the signing key is a P-256 ECDSA key.
#[derive(Clone)]
pub struct CertificateAndKey {
    /// DER-encoded X.509 certificate
    pub cert_der: Vec<u8>,
    /// ECDSA P-256 signing key (for CertificateVerify during TLS handshake)
    pub signing_key: Arc<p256::ecdsa::SigningKey>,
}

impl CertificateAndKey {
    /// Create from raw DER components.
    pub fn from_der(cert_der: Vec<u8>, signing_key: p256::ecdsa::SigningKey) -> Self {
        Self {
            cert_der,
            signing_key: Arc::new(signing_key),
        }
    }

    /// Parse a PEM-encoded certificate and PKCS#8 private key.
    ///
    /// The PEM text should contain exactly one `CERTIFICATE` block and
    /// one `PRIVATE KEY` (or `EC PRIVATE KEY`) block.
    pub fn from_pem(pem_text: &str) -> Result<Self, CryptoError> {
        let (cert_der, signing_key) = load_cert_and_key_from_pem(pem_text)?;
        Ok(Self {
            cert_der,
            signing_key: Arc::new(signing_key),
        })
    }

    /// Parse from DER-encoded certificate and PKCS#8 DER-encoded private key.
    pub fn from_der_pair(cert_der: &[u8], key_der: &[u8]) -> Result<Self, CryptoError> {
        let signing_key = p256_signing_key_from_der(key_der)?;
        Ok(Self {
            cert_der: cert_der.to_vec(),
            signing_key: Arc::new(signing_key),
        })
    }

    /// Serialize the certificate as PEM.
    pub fn cert_pem(&self) -> String {
        pem_encode("CERTIFICATE", &self.cert_der)
    }

    /// Serialize the private key as PEM.
    pub fn key_pem(&self) -> Result<String, CryptoError> {
        p256_signing_key_to_pem(&self.signing_key)
    }

    /// Serialize both certificate and key as a single PEM string.
    pub fn to_pem(&self) -> Result<String, CryptoError> {
        let key_pem = self.key_pem()?;
        Ok(format!("{}\n{}", self.cert_pem(), key_pem))
    }
}

/// Generate a self-signed certificate for the given hostname(s).
///
/// Uses `rcgen` to produce a properly DER-encoded X.509 v3 certificate
/// with ECDSA P-256 key, SAN extensions, and 1-year validity.
pub fn generate_self_signed(hostnames: &[&str]) -> Result<CertificateAndKey, CryptoError> {
    let (cert_der, signing_key) = edgerun_crypto::generate_self_signed(hostnames)?;
    Ok(CertificateAndKey {
        cert_der,
        signing_key: Arc::new(signing_key),
    })
}

/// Generate a self-signed certificate and return it as PEM.
///
/// Returns `(cert_pem, key_pem)`.
pub fn generate_self_signed_pem(hostnames: &[&str]) -> Result<(String, String), CryptoError> {
    edgerun_crypto::generate_self_signed_pem(hostnames)
}

/// Parse a PEM-encoded X.509 certificate, returning DER bytes.
pub fn cert_from_pem(pem_str: &str) -> Result<Vec<u8>, CryptoError> {
    x509_cert_from_pem(pem_str)
}

/// Parse a PEM-encoded PKCS#8 private key.
pub fn signing_key_from_pem(pem_str: &str) -> Result<p256::ecdsa::SigningKey, CryptoError> {
    p256_signing_key_from_pem(pem_str)
}

/// Serialize a P-256 signing key to PEM.
pub fn signing_key_to_pem(key: &p256::ecdsa::SigningKey) -> Result<String, CryptoError> {
    p256_signing_key_to_pem(key)
}
