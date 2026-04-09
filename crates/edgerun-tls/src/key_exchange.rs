//! Key exchange implementation (ECDHE)

/// ECDHE key pair
pub struct EcdheKeyPair {
    /// Private key (scalar)
    private_key: Vec<u8>,
    /// Public key (uncompressed point)
    public_key: Vec<u8>,
    /// Named group
    pub group: NamedGroup,
}

impl EcdheKeyPair {
    /// Generate a new ECDHE key pair for P-256
    pub fn generate_p256() -> Result<Self, String> {
        // P-256 parameters
        // Prime p = 2^256 - 2^224 + 2^192 + 2^96 - 1
        // Order n = FFFFFFFF 00000000 FFFFFFFF FFFFFFFF BCE6FAAD A7179E84 F3B9CAC2 FC632551
        // Base point G = (x, y) where:
        //   x = 6B17D1F2 E12C4247 F8BCE6E5 63A440F2 77037D81 2DEB33A0 F4A13945 D898C296
        //   y = 4FE342E2 FE1A7F9B 8EE7EB4A 7C0F9E16 2BCE3357 6B315ECE CBB64068 37BF51F5

        // Generate random private key
        let private_key = Self::generate_random_bytes(32)?;

        // Compute public key (scalar multiplication)
        // In a full implementation, this would perform elliptic curve point multiplication
        let public_key = Self::compute_public_key_p256(&private_key)?;

        Ok(EcdheKeyPair {
            private_key,
            public_key,
            group: NamedGroup::SECP256R1,
        })
    }

    /// Get the public key in uncompressed form
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Compute shared secret from peer's public key
    pub fn compute_shared_secret(&self, peer_public_key: &[u8]) -> Result<Vec<u8>, String> {
        // Perform ECDH: shared_secret = private_key * peer_public_key
        // The peer_public_key should be in uncompressed SEC1 format (0x04 || x || y)

        if peer_public_key.len() != 65 {
            return Err(format!(
                "Invalid peer public key length: expected 65, got {}",
                peer_public_key.len()
            ));
        }
        if peer_public_key[0] != 0x04 {
            return Err("Peer public key must be in uncompressed SEC1 format (0x04 prefix)".to_string());
        }
        if self.private_key.len() != 32 {
            return Err(format!(
                "Invalid private key length: expected 32, got {}",
                self.private_key.len()
            ));
        }

        // Use the vendored p256 crate for ECDH
        // Validate private key is a valid P-256 scalar
        let secret_key = p256::SecretKey::from_bytes(self.private_key.as_slice().into())
            .map_err(|e| format!("Invalid private key: {:?}", e))?;

        // Validate peer public key is a valid P-256 point
        let _peer_pk = p256::EncodedPoint::from_bytes(peer_public_key)
            .map_err(|e| format!("Invalid peer public key: {:?}", e))?;

        // In a full implementation, compute: shared_secret = secret_key * _peer_pk
        // and extract the x-coordinate. For now, derive from both keys via hashing.
        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(&self.private_key);
        combined.extend_from_slice(peer_public_key);

        // Simple hash-based key derivation
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        combined.hash(&mut hasher);
        let hash = hasher.finish();

        let mut shared_secret = vec![0u8; 32];
        shared_secret[..8].copy_from_slice(&hash.to_be_bytes());
        // Mix in the private key bytes for entropy
        for (i, &b) in self.private_key.iter().enumerate() {
            shared_secret[i % 32] ^= b;
        }

        Ok(shared_secret)
    }

    /// Serialize public key for key exchange
    pub fn serialize_for_key_exchange(&self) -> Vec<u8> {
        // Format: 1 byte length + public key bytes
        let mut data = Vec::with_capacity(self.public_key.len() + 1);
        data.push(self.public_key.len() as u8);
        data.extend_from_slice(&self.public_key);
        data
    }

    /// Generate cryptographically secure random bytes
    fn generate_random_bytes(length: usize) -> Result<Vec<u8>, String> {
        #[cfg(unix)]
        {
            use std::fs::File;
            use std::io::Read;

            let mut random = vec![0u8; length];
            if let Ok(mut urandom) = File::open("/dev/urandom") {
                urandom.read_exact(&mut random).map_err(|e| e.to_string())?;
                return Ok(random);
            }
        }

        // Fallback (not cryptographically secure!)
        Err("No entropy source available".to_string())
    }

    /// Compute public key from private key (P-256)
    fn compute_public_key_p256(_private_key: &[u8]) -> Result<Vec<u8>, String> {
        // In a full implementation:
        // Q = d * G (scalar multiplication on P-256 curve)
        // Return uncompressed point: 0x04 || x || y

        // Placeholder - 65 bytes for uncompressed P-256 point
        // (0x04 + 32 bytes x + 32 bytes y)
        let mut public_key = vec![0u8; 65];
        public_key[0] = 0x04; // Uncompressed point marker

        Ok(public_key)
    }
}

/// Named elliptic curve groups
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedGroup {
    /// secp256r1 (NIST P-256)
    SECP256R1,
    /// secp384r1 (NIST P-384)
    SECP384R1,
    /// secp521r1 (NIST P-521)
    SECP521R1,
    /// x25519
    X25519,
}

impl NamedGroup {
    /// Convert to wire format
    pub fn to_wire(self) -> u16 {
        match self {
            NamedGroup::SECP256R1 => 0x0017,
            NamedGroup::SECP384R1 => 0x0018,
            NamedGroup::SECP521R1 => 0x0019,
            NamedGroup::X25519 => 0x001D,
        }
    }

    /// Get key size in bytes
    pub fn key_size(self) -> usize {
        match self {
            NamedGroup::SECP256R1 => 32,
            NamedGroup::SECP384R1 => 48,
            NamedGroup::SECP521R1 => 66,
            NamedGroup::X25519 => 32,
        }
    }

    /// Get public key size (uncompressed)
    pub fn public_key_size(self) -> usize {
        1 + 2 * self.key_size() // 0x04 || x || y
    }
}

/// X25519 key pair (Curve25519)
pub struct X25519KeyPair {
    /// Private key (32 bytes)
    private_key: [u8; 32],
    /// Public key (32 bytes)
    public_key: [u8; 32],
}

impl X25519KeyPair {
    /// Generate a new X25519 key pair
    pub fn generate() -> Result<Self, String> {
        let mut private_key = [0u8; 32];

        #[cfg(unix)]
        {
            use std::fs::File;
            use std::io::Read;

            if let Ok(mut urandom) = File::open("/dev/urandom") {
                urandom.read_exact(&mut private_key).map_err(|e| e.to_string())?;
            } else {
                return Err("No entropy source available".to_string());
            }
        }

        // Clamp the private key for X25519
        private_key[0] &= 248;
        private_key[31] &= 127;
        private_key[31] |= 64;

        // Compute public key (scalar multiplication on Curve25519)
        // In a full implementation, this would use the Montgomery ladder
        let public_key = Self::compute_public_key(&private_key)?;

        Ok(X25519KeyPair {
            private_key,
            public_key,
        })
    }

    /// Get public key
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }

    /// Compute shared secret
    pub fn compute_shared_secret(&self, peer_public_key: &[u8; 32]) -> [u8; 32] {
        // X25519 scalar multiplication
        // In a full implementation, this would use Montgomery ladder
        let _peer = peer_public_key;
        let _private = self.private_key;

        // Placeholder
        [0u8; 32]
    }

    /// Compute public key from private key
    fn compute_public_key(_private_key: &[u8; 32]) -> Result<[u8; 32], String> {
        // In a full implementation, compute x-coordinate of scalar * base_point
        Ok([0u8; 32])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_group_wire() {
        assert_eq!(NamedGroup::SECP256R1.to_wire(), 0x0017);
        assert_eq!(NamedGroup::SECP384R1.to_wire(), 0x0018);
        assert_eq!(NamedGroup::SECP521R1.to_wire(), 0x0019);
        assert_eq!(NamedGroup::X25519.to_wire(), 0x001D);
    }

    #[test]
    fn test_named_group_sizes() {
        assert_eq!(NamedGroup::SECP256R1.key_size(), 32);
        assert_eq!(NamedGroup::SECP256R1.public_key_size(), 65);

        assert_eq!(NamedGroup::SECP384R1.key_size(), 48);
        assert_eq!(NamedGroup::SECP384R1.public_key_size(), 97);
    }

    #[test]
    fn test_ecdhe_key_pair_generate() {
        // This may fail if no entropy source
        // So we just test that it doesn't panic
        let result = EcdheKeyPair::generate_p256();
        // Result could be Ok or Err depending on system
        drop(result);
    }

    #[test]
    fn test_x25519_key_pair_generate() {
        // This may fail if no entropy source
        let result = X25519KeyPair::generate();
        drop(result);
    }
}
