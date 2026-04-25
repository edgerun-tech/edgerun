// ---------------------------------------------------------------------------
// PKCS#10 Certificate Signing Request (CSR)
// ---------------------------------------------------------------------------

use crate::CryptoError;
use p256::ecdsa::SigningKey;

/// Generate a PKCS#10 CSR for the given domains using P-256 ECDSA.
///
/// Returns DER-encoded CSR bytes.
pub fn generate_csr_p256(
    domains: &[String],
    signing_key: &SigningKey,
) -> Result<Vec<u8>, CryptoError> {
    let pem = generate_csr_pem(domains, signing_key)?;
    Ok(pem.as_bytes().to_vec())
}

/// Generate a PKCS#10 CSR for the given domains, returned as PEM.
///
/// Returns PEM-encoded CSR string.
pub fn generate_csr_pem(
    domains: &[String],
    signing_key: &SigningKey,
) -> Result<String, CryptoError> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, RcgenError};

    if domains.is_empty() {
        return Err(CryptoError::CertificateError(
            "generate_csr: at least one domain required".into(),
        ));
    }

    // Convert signing key to PEM format
    let key_pem = crate::p256_signing_key_to_pem(signing_key)
        .map_err(|e| CryptoError::CertificateError(e.to_string()))?;

    // Create KeyPair from PEM
    let key_pair = KeyPair::from_pem(&key_pem)
        .map_err(|e| CryptoError::CertificateError(format!("Failed to load key: {}", e)))?;

    // Create certificate params (used for CSR generation)
    let params = CertificateParams::new(domains.to_vec())
        .map_err(|e| CryptoError::CertificateError(e.to_string()))?;

    // Generate CSR
    let csr = params
        .serialize_request(&key_pair)
        .map_err(|e: RcgenError| CryptoError::CertificateError(e.to_string()))?;

    // Get PEM encoding
    let pem_str = csr
        .pem()
        .map_err(|e| CryptoError::CertificateError(e.to_string()))?;
    Ok(pem_str)
}
