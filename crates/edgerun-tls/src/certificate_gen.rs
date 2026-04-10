//! X.509 certificate generation wrapper.
//!
//! Uses `x509-cert` from edgerun-crypto for certificate parsing/structure,
//! and generates self-signed certificates via rcgen (only for the h2spec test server).
//!
//! The `x509-cert` crate is re-exported via `edgerun_crypto::x509_cert`.

// Re-export x509-cert for consumers
pub use edgerun_crypto::x509_cert;

/// A self-signed certificate with an associated P-256 signing key.
pub struct CertificateAndKey {
    /// DER-encoded X.509 certificate
    pub cert_der: Vec<u8>,
    /// ECDSA P-256 signing key
    pub signing_key: edgerun_crypto::p256::ecdsa::SigningKey,
}

/// Generate a self-signed certificate for the given hostname(s).
pub fn generate_self_signed(_hostnames: &[&str]) -> CertificateAndKey {
    // TODO: Implement using x509-cert builder API once trait bounds align.
    // For now, the h2spec-server binary uses rcgen directly.
    panic!("generate_self_signed not yet implemented — use rcgen in h2spec-server for now");
}
