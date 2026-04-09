//! Cipher suite definitions and implementations

use std::fmt;

/// TLS cipher suite
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CipherSuite {
    // TLS 1.3 cipher suites
    /// TLS_AES_128_GCM_SHA256
    TLS_AES_128_GCM_SHA256,
    /// TLS_AES_256_GCM_SHA384
    TLS_AES_256_GCM_SHA384,
    /// TLS_CHACHA20_POLY1305_SHA256
    TLS_CHACHA20_POLY1305_SHA256,

    // TLS 1.2 cipher suites
    /// TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
    TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    /// TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
    TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    /// TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
    TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    /// TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
    TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
}

impl CipherSuite {
    /// Get supported cipher suites
    pub fn supported() -> Vec<Self> {
        vec![
            // TLS 1.3 suites (preferred)
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_AES_256_GCM_SHA384,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
            // TLS 1.2 suites (fallback)
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        ]
    }

    /// Convert to wire format
    pub fn to_wire(self) -> u16 {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => 0x1301,
            CipherSuite::TLS_AES_256_GCM_SHA384 => 0x1302,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => 0x1303,
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 => 0xC02B,
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 => 0xC02F,
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 => 0xC02C,
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => 0xC030,
        }
    }

    /// Parse from wire format
    pub fn from_wire(value: u16) -> Result<Self, String> {
        match value {
            0x1301 => Ok(CipherSuite::TLS_AES_128_GCM_SHA256),
            0x1302 => Ok(CipherSuite::TLS_AES_256_GCM_SHA384),
            0x1303 => Ok(CipherSuite::TLS_CHACHA20_POLY1305_SHA256),
            0xC02B => Ok(CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256),
            0xC02F => Ok(CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256),
            0xC02C => Ok(CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384),
            0xC030 => Ok(CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384),
            _ => Err(format!("Unsupported cipher suite: 0x{:04x}", value)),
        }
    }

    /// Check if this is a TLS 1.3 cipher suite
    pub fn is_tls13(&self) -> bool {
        matches!(
            self,
            CipherSuite::TLS_AES_128_GCM_SHA256
                | CipherSuite::TLS_AES_256_GCM_SHA384
                | CipherSuite::TLS_CHACHA20_POLY1305_SHA256
        )
    }

    /// Get the key exchange algorithm
    pub fn key_exchange(&self) -> KeyExchange {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256
            | CipherSuite::TLS_AES_256_GCM_SHA384
            | CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => KeyExchange::None, // TLS 1.3
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => KeyExchange::ECDHE,
        }
    }

    /// Get the authentication algorithm
    pub fn authentication(&self) -> Authentication {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256
            | CipherSuite::TLS_AES_256_GCM_SHA384
            | CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => Authentication::None, // TLS 1.3
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 => {
                Authentication::ECDSA
            }
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => Authentication::RSA,
        }
    }

    /// Get the bulk encryption algorithm
    pub fn encryption(&self) -> Encryption {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 => Encryption::AES_128_GCM,
            CipherSuite::TLS_AES_256_GCM_SHA384
            | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => Encryption::AES_256_GCM,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => Encryption::CHACHA20_POLY1305,
        }
    }

    /// Get the hash algorithm for PRF
    pub fn hash(&self) -> Hash {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            | CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => Hash::SHA256,
            CipherSuite::TLS_AES_256_GCM_SHA384
            | CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            | CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => Hash::SHA384,
        }
    }
}

impl fmt::Display for CipherSuite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CipherSuite::TLS_AES_128_GCM_SHA256 => write!(f, "TLS_AES_128_GCM_SHA256"),
            CipherSuite::TLS_AES_256_GCM_SHA384 => write!(f, "TLS_AES_256_GCM_SHA384"),
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 => {
                write!(f, "TLS_CHACHA20_POLY1305_SHA256")
            }
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 => {
                write!(f, "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256")
            }
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 => {
                write!(f, "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256")
            }
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 => {
                write!(f, "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384")
            }
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => {
                write!(f, "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384")
            }
        }
    }
}

/// Key exchange algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyExchange {
    /// None (TLS 1.3)
    None,
    /// Elliptic Curve Diffie-Hellman Ephemeral
    ECDHE,
}

/// Authentication algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authentication {
    /// None (TLS 1.3)
    None,
    /// RSA
    RSA,
    /// ECDSA
    ECDSA,
}

/// Bulk encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    /// AES-128-GCM
    AES_128_GCM,
    /// AES-256-GCM
    AES_256_GCM,
    /// ChaCha20-Poly1305
    CHACHA20_POLY1305,
}

/// Hash algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hash {
    /// SHA-256
    SHA256,
    /// SHA-384
    SHA384,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cipher_suite_wire() {
        let suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        assert_eq!(suite.to_wire(), 0x1301);
        assert_eq!(
            CipherSuite::from_wire(0x1301).unwrap(),
            CipherSuite::TLS_AES_128_GCM_SHA256
        );
    }

    #[test]
    fn test_cipher_suite_is_tls13() {
        assert!(CipherSuite::TLS_AES_128_GCM_SHA256.is_tls13());
        assert!(CipherSuite::TLS_AES_256_GCM_SHA384.is_tls13());
        assert!(CipherSuite::TLS_CHACHA20_POLY1305_SHA256.is_tls13());
        assert!(!CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256.is_tls13());
    }

    #[test]
    fn test_cipher_suite_supported() {
        let suites = CipherSuite::supported();
        assert!(!suites.is_empty());
        assert_eq!(suites.len(), 7);
    }

    #[test]
    fn test_cipher_suite_properties() {
        let suite = CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384;
        assert_eq!(suite.key_exchange(), KeyExchange::ECDHE);
        assert_eq!(suite.authentication(), Authentication::RSA);
        assert_eq!(suite.encryption(), Encryption::AES_256_GCM);
        assert_eq!(suite.hash(), Hash::SHA384);
    }

    #[test]
    fn test_cipher_suite_display() {
        let suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        assert_eq!(suite.to_string(), "TLS_AES_128_GCM_SHA256");
    }

    #[test]
    fn test_cipher_suite_invalid() {
        assert!(CipherSuite::from_wire(0xFFFF).is_err());
    }
}
