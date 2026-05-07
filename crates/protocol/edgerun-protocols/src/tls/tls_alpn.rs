//! TLS-ALPN-01 ACME challenge support (RFC 8737).
//!
//! The TLS-ALPN-01 challenge validates domain ownership by presenting
//! a certificate containing the ACME challenge value during TLS handshake.
//!
//! Flow:
//! 1. ACME server connects to port 443
//! 2. ClientHello includes `acme-tls/1` ALPN protocol
//! 3. Server presents certificate with challenge value in SAN
//! 4. ACME server validates the certificate contains correct challenge
//!
//! Usage:
//! ```rust
//! use edgerun_protocols::tls::tls_alpn::ACME_TLS_ALPN_PROTOCOL;
//!
//! assert_eq!(ACME_TLS_ALPN_PROTOCOL, b"acme-tls/1");
//! ```

/// ACME TLS-ALPN-01 protocol identifier (RFC 8737)
pub const ACME_TLS_ALPN_PROTOCOL: &[u8] = b"acme-tls/1";
