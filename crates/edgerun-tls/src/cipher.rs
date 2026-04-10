//! TLS cipher suite definitions (TLS 1.3 only)

// IANA cipher suite names use ALL_CAPS_SNAKE_CASE with numbers
#![allow(non_camel_case_types)]

/// TLS 1.3 cipher suite
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CipherSuite {
    /// TLS_AES_128_GCM_SHA256
    TLS_AES_128_GCM_SHA256,
    /// TLS_AES_256_GCM_SHA384
    TLS_AES_256_GCM_SHA384,
    /// TLS_CHACHA20_POLY1305_SHA256
    TLS_CHACHA20_POLY1305_SHA256,
}

impl CipherSuite {
    /// Cipher suites offered by the client, in preference order.
    /// Only includes cipher suites with working implementations in RecordCipher.
    pub fn client_default() -> Vec<Self> {
        vec![
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_AES_256_GCM_SHA384,
            // TLS_CHACHA20_POLY1305_SHA256 (0x1303) is NOT offered because
            // RecordCipher only implements AES-GCM. It remains parseable for
            // server selection (from_wire) but will not be negotiated.
        ]
    }

    /// IANA wire-format value
    pub fn to_wire(self) -> u16 {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => 0x1301,
            CipherSuite::TLS_AES_256_GCM_SHA384 => 0x1302,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 0x1303,
        }
    }

    /// Parse from wire format
    pub fn from_wire(value: u16) -> Result<Self, String> {
        match value {
            0x1301 => Ok(CipherSuite::TLS_AES_128_GCM_SHA256),
            0x1302 => Ok(CipherSuite::TLS_AES_256_GCM_SHA384),
            0x1303 => Ok(CipherSuite::TLS_CHACHA20_POLY1305_SHA256),
            _ => Err(format!("Unsupported cipher suite: 0x{:04x}", value)),
        }
    }

    /// AEAD key length in bytes
    pub fn key_len(self) -> usize {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => 16,
            CipherSuite::TLS_AES_256_GCM_SHA384 => 32,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 32,
        }
    }

    /// AEAD nonce/IV length in bytes (always 12 for TLS 1.3)
    pub const fn iv_len(&self) -> usize {
        12
    }

    /// AEAD tag length in bytes (always 16 for GCM/Poly1305 in TLS 1.3)
    pub const fn tag_len(&self) -> usize {
        16
    }

    /// Hash function output length in bytes
    pub fn hash_len(self) -> usize {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 | CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 32,
            CipherSuite::TLS_AES_256_GCM_SHA384 => 48,
        }
    }

    /// Named group used for ECDH
    pub fn named_group(self) -> NamedGroup {
        // TLS 1.3 always uses the key_share group from extensions
        NamedGroup::SECP256R1
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
        vec![NamedGroup::SECP256R1, NamedGroup::X25519, NamedGroup::SECP384R1]
    }
}
