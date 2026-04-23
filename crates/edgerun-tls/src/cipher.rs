//! TLS cipher suite definitions (TLS 1.3 only)
//!
//! Re-exports `CipherSuite` from `edgerun_crypto` for consistency
//! across TLS and QUIC implementations.

pub use edgerun_crypto::CipherSuite;

/// Named elliptic curve groups for TLS 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedGroup {
    /// secp256r1 (NIST P-256)
    SECP256R1,
    /// secp384r1 (NIST P-384)
    SECP384R1,
    /// x25519 (Curve25519)
    X25519,
}

impl NamedGroup {
    /// IANA wire-format value
    pub fn to_wire(self) -> u16 {
        match self {
            NamedGroup::SECP256R1 => 0x0017,
            NamedGroup::SECP384R1 => 0x0018,
            NamedGroup::X25519 => 0x001D,
        }
    }

    /// Scalar (private key) size in bytes
    pub fn scalar_len(self) -> usize {
        match self {
            NamedGroup::SECP256R1 => 32,
            NamedGroup::SECP384R1 => 48,
            NamedGroup::X25519 => 32,
        }
    }

    /// Public key size (uncompressed SEC1 point)
    pub fn public_key_len(self) -> usize {
        1 + 2 * self.scalar_len()
    }

    /// Client-preferred groups in order
    pub fn client_default() -> Vec<Self> {
        vec![NamedGroup::X25519, NamedGroup::SECP256R1, NamedGroup::SECP384R1]
    }
}
