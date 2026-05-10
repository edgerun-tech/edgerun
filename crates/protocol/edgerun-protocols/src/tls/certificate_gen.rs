//! X.509 certificate generation and PEM/DER conversion.
//!
//! Provides `CertificateAndKey` — a self-signed certificate with an
//! associated ECDSA P-256 signing key — plus PEM ↔ DER conversion utilities.

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::error::CryptoError;
use edgerun_crypto::{
    p256, p256_signing_key_from_der, p256_signing_key_from_pem, p256_signing_key_to_der,
    p256_signing_key_to_pem, pem_encode, x509_cert_from_pem,
};
use edgerun_encoding::base64::standard_decode;

/// A certificate with an associated ECDSA P-256 signing key.
///
/// This is the primary type for TLS server identity. The certificate
/// is DER-encoded X.509; the signing key is a P-256 ECDSA key.
#[derive(Clone)]
pub struct CertificateAndKey {
    /// DER-encoded X.509 certificate
    pub cert_der: Vec<u8>,
    /// DER-encoded certificate chain, leaf first.
    pub cert_chain_der: Vec<Vec<u8>>,
    /// ECDSA P-256 signing key (for CertificateVerify during TLS handshake)
    pub signing_key: Arc<p256::ecdsa::SigningKey>,
}

impl CertificateAndKey {
    /// Create from raw DER components.
    pub fn from_der(cert_der: Vec<u8>, signing_key: p256::ecdsa::SigningKey) -> Self {
        Self {
            cert_chain_der: vec![cert_der.clone()],
            cert_der,
            signing_key: Arc::new(signing_key),
        }
    }

    /// Parse a PEM-encoded certificate and PKCS#8 private key.
    ///
    /// The PEM text should contain exactly one `CERTIFICATE` block and
    /// one `PRIVATE KEY` (or `EC PRIVATE KEY`) block.
    pub fn from_pem(pem_text: &str) -> Result<Self, CryptoError> {
        let cert_der = x509_cert_from_pem(pem_text).ok_or(CryptoError::InvalidKey)?;
        let mut cert_chain_der = certs_from_pem(pem_text);
        if cert_chain_der.is_empty() {
            cert_chain_der.push(cert_der.clone());
        }
        let signing_key = p256_signing_key_from_pem(pem_text).ok_or(CryptoError::InvalidKey)?;
        Ok(Self {
            cert_der,
            cert_chain_der,
            signing_key: Arc::new(signing_key),
        })
    }

    /// Parse from DER-encoded certificate and PKCS#8 DER-encoded private key.
    pub fn from_der_pair(cert_der: &[u8], key_der: &[u8]) -> Result<Self, CryptoError> {
        let signing_key = p256_signing_key_from_der(key_der).ok_or(CryptoError::InvalidKey)?;
        Ok(Self {
            cert_der: cert_der.to_vec(),
            cert_chain_der: vec![cert_der.to_vec()],
            signing_key: Arc::new(signing_key),
        })
    }

    /// Serialize the certificate as PEM.
    pub fn cert_pem(&self) -> String {
        pem_encode(&self.cert_der)
    }

    /// Serialize the private key as PEM.
    pub fn key_pem(&self) -> Result<String, CryptoError> {
        Ok(p256_signing_key_to_pem(&self.signing_key))
    }

    /// Serialize the private key as PKCS#8 DER.
    pub fn key_der(&self) -> Vec<u8> {
        p256_signing_key_to_der(&self.signing_key)
    }

    /// Serialize both certificate and key as a single PEM string.
    pub fn to_pem(&self) -> Result<String, CryptoError> {
        let key_pem = self.key_pem()?;
        Ok(format!("{}\n{}", self.cert_pem(), key_pem))
    }
}

fn certs_from_pem(pem_text: &str) -> Vec<Vec<u8>> {
    let mut certs = Vec::new();
    let mut rest = pem_text;
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    while let Some(start) = rest.find(BEGIN) {
        let after_begin = &rest[start + BEGIN.len()..];
        let Some(end) = after_begin.find(END) else {
            break;
        };
        let b64 = after_begin[..end]
            .lines()
            .map(str::trim)
            .collect::<String>();
        if let Ok(der) = standard_decode(&b64) {
            certs.push(der);
        }
        rest = &after_begin[end + END.len()..];
    }
    certs
}

/// Generate a self-signed certificate for the given hostname(s).
///
/// Uses `rcgen` to produce a properly DER-encoded X.509 v3 certificate
/// with ECDSA P-256 key, SAN extensions, and 1-year validity.
pub fn generate_self_signed(hostnames: &[&str]) -> Result<CertificateAndKey, CryptoError> {
    let signing_key = edgerun_crypto::random_p256_signing_key();
    let cert_der = edgerun_crypto::generate_self_signed_for_names(&signing_key, hostnames);
    Ok(CertificateAndKey {
        cert_chain_der: vec![cert_der.clone()],
        cert_der,
        signing_key: Arc::new(signing_key),
    })
}

/// Generate a self-signed certificate and return it as PEM.
///
/// Returns `(cert_pem, key_pem)`.
pub fn generate_self_signed_pem(hostnames: &[&str]) -> Result<(String, String), CryptoError> {
    let cert = generate_self_signed(hostnames)?;
    Ok((cert.cert_pem(), cert.key_pem()?))
}

/// Generate a P-256 key and PKCS#10 CSR for the given hostnames.
pub fn generate_csr(hostnames: &[&str]) -> Result<(Vec<u8>, p256::ecdsa::SigningKey), CryptoError> {
    let signing_key = edgerun_crypto::random_p256_signing_key();
    let csr_der = edgerun_crypto::generate_csr_for_names(&signing_key, hostnames);
    Ok((csr_der, signing_key))
}

/// Parse a PEM-encoded X.509 certificate, returning DER bytes.
pub fn cert_from_pem(pem_str: &str) -> Result<Vec<u8>, CryptoError> {
    x509_cert_from_pem(pem_str).ok_or(CryptoError::InvalidKey)
}

/// Parse a PEM-encoded PKCS#8 private key.
pub fn signing_key_from_pem(pem_str: &str) -> Result<p256::ecdsa::SigningKey, CryptoError> {
    p256_signing_key_from_pem(pem_str).ok_or(CryptoError::InvalidKey)
}

/// Serialize a P-256 signing key to PEM.
pub fn signing_key_to_pem(key: &p256::ecdsa::SigningKey) -> Result<String, CryptoError> {
    Ok(p256_signing_key_to_pem(key))
}
