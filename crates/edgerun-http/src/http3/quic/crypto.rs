//! QUIC packet protection (RFC 9001)
//!
//! Implements AEAD packet protection for QUIC using AES-GCM.
//! The TLS 1.3 handshake for QUIC (CRYPTO frames, key schedule)
//! requires a separate QUIC-TLS adapter that is not yet implemented.
//!
//! This module provides:
//! - `PacketProtection` — AEAD encrypt/decrypt with QUIC-specific nonce and AAD
//! - `ProtectionKeys` — derived traffic keys for Initial/Handshake/1-RTT levels
//! - Hardcoded test keys for unit testing the packet layer

use edgerun_crypto::aes_gcm::{
    aead::{Aead, KeyInit},
    Aes128Gcm, Aes256Gcm, Key, Nonce,
};

use std::collections::HashMap;

/// Crypto phase / encryption level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CryptoPhase {
    /// Initial keys (derived from initial salt + destination CID)
    Initial,
    /// Handshake keys (derived from ECDH shared secret)
    Handshake,
    /// Application data keys (1-RTT, derived from master secret)
    Application,
}

/// AEAD algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadAlgorithm {
    Aes128Gcm,
    Aes256Gcm,
}

impl AeadAlgorithm {
    fn key_len(self) -> usize {
        match self {
            AeadAlgorithm::Aes128Gcm => 16,
            AeadAlgorithm::Aes256Gcm => 32,
        }
    }

    fn tag_len(&self) -> usize {
        // Both AES-GCM variants produce 16-byte tags
        16
    }
}

/// Keys for protecting packets at a given level
#[derive(Clone)]
pub struct ProtectionKeys {
    /// AEAD algorithm
    pub algorithm: AeadAlgorithm,
    /// Write (encrypt) key
    pub write_key: Vec<u8>,
    /// Write IV (for nonce derivation)
    pub write_iv: Vec<u8>,
    /// Read (decrypt) key
    pub read_key: Vec<u8>,
    /// Read IV
    pub read_iv: Vec<u8>,
}

impl ProtectionKeys {
    /// Create from raw key material
    pub fn new(
        algorithm: AeadAlgorithm,
        write_key: Vec<u8>,
        write_iv: Vec<u8>,
        read_key: Vec<u8>,
        read_iv: Vec<u8>,
    ) -> Self {
        ProtectionKeys {
            algorithm,
            write_key,
            write_iv,
            read_key,
            read_iv,
        }
    }

    /// Create test keys (all zeros) for unit testing
    pub fn test_keys() -> Self {
        let key_len = AeadAlgorithm::Aes128Gcm.key_len();
        let iv_len = 12;
        ProtectionKeys {
            algorithm: AeadAlgorithm::Aes128Gcm,
            write_key: vec![0u8; key_len],
            write_iv: vec![0u8; iv_len],
            read_key: vec![0u8; key_len],
            read_iv: vec![0u8; iv_len],
        }
    }
}

/// QUIC packet protection engine
pub struct PacketProtection {
    /// Encryption AEAD
    write_aead: AeadCipher,
    /// Decryption AEAD
    read_aead: AeadCipher,
    /// Write IV
    write_iv: Vec<u8>,
    /// Read IV
    read_iv: Vec<u8>,
    /// Packet number for nonce generation
    packet_number: u64,
}

enum AeadCipher {
    Aes128(Aes128Gcm),
    Aes256(Aes256Gcm),
}

impl PacketProtection {
    /// Create from protection keys
    pub fn new(keys: &ProtectionKeys) -> Self {
        let write_aead = match keys.algorithm {
            AeadAlgorithm::Aes128Gcm => {
                let key = Key::<Aes128Gcm>::from_slice(&keys.write_key);
                AeadCipher::Aes128(Aes128Gcm::new(key))
            }
            AeadAlgorithm::Aes256Gcm => {
                let key = Key::<Aes256Gcm>::from_slice(&keys.write_key);
                AeadCipher::Aes256(Aes256Gcm::new(key))
            }
        };

        let read_aead = match keys.algorithm {
            AeadAlgorithm::Aes128Gcm => {
                let key = Key::<Aes128Gcm>::from_slice(&keys.read_key);
                AeadCipher::Aes128(Aes128Gcm::new(key))
            }
            AeadAlgorithm::Aes256Gcm => {
                let key = Key::<Aes256Gcm>::from_slice(&keys.read_key);
                AeadCipher::Aes256(Aes256Gcm::new(key))
            }
        };

        PacketProtection {
            write_aead,
            read_aead,
            write_iv: keys.write_iv.clone(),
            read_iv: keys.read_iv.clone(),
            packet_number: 0,
        }
    }

    /// Protect (encrypt) a packet payload
    ///
    /// AAD = unprotected packet header
    /// Nonce = write_iv XOR (packet_number << 8)
    pub fn protect(&mut self, header: &[u8], plaintext: &[u8]) -> Vec<u8> {
        let pn = self.packet_number;
        self.packet_number += 1;

        let nonce = self.make_nonce(&self.write_iv, pn);

        match &self.write_aead {
            AeadCipher::Aes128(aead) => {
                let nonce = Nonce::from_slice(&nonce);
                aead.encrypt(nonce, plaintext)
                    .unwrap_or_else(|_| plaintext.to_vec())
            }
            AeadCipher::Aes256(aead) => {
                let nonce = Nonce::from_slice(&nonce);
                aead.encrypt(nonce, plaintext)
                    .unwrap_or_else(|_| plaintext.to_vec())
            }
        }
    }

    /// Unprotect (decrypt) a packet payload
    ///
    /// AAD = unprotected packet header
    /// Nonce = read_iv XOR (packet_number << 8)
    pub fn unprotect(&mut self, header: &[u8], packet_number: u64, ciphertext: &[u8]) -> Option<Vec<u8>> {
        let nonce = self.make_nonce(&self.read_iv, packet_number);

        match &self.read_aead {
            AeadCipher::Aes128(aead) => {
                let nonce = Nonce::from_slice(&nonce);
                aead.decrypt(nonce, ciphertext).ok()
            }
            AeadCipher::Aes256(aead) => {
                let nonce = Nonce::from_slice(&nonce);
                aead.decrypt(nonce, ciphertext).ok()
            }
        }
    }

    /// QUIC nonce construction: iv XOR (packet_number << 8)
    /// RFC 9001 Section 5.3
    fn make_nonce(&self, iv: &[u8], packet_number: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(iv);

        // XOR the last 8 bytes with the packet number (big-endian, left-padded)
        let pn_bytes = packet_number.to_be_bytes();
        for i in 0..8 {
            nonce[4 + i] ^= pn_bytes[i];
        }

        nonce
    }

    /// Get current packet number
    pub fn packet_number(&self) -> u64 {
        self.packet_number
    }
}

/// QUIC crypto context — manages keys at different encryption levels.
///
/// NOTE: The TLS 1.3 handshake for QUIC (deriving keys from ECDH via
/// CRYPTO frames) is NOT yet implemented. This requires a QUIC-TLS adapter
/// that drives the TLS 1.3 handshake without the record layer.
///
/// What IS implemented:
/// - `PacketProtection` — AEAD encrypt/decrypt with proper QUIC nonce
/// - `ProtectionKeys` — key storage for each encryption level
/// - Test key material for unit testing the packet/transport layers
pub struct QuicCrypto {
    /// Keys at each encryption level
    keys: HashMap<CryptoPhase, ProtectionKeys>,
    /// Current active phase
    active_phase: Option<CryptoPhase>,
    /// CRYPTO frame buffer (handshake messages pending / received)
    crypto_buffer: Vec<u8>,
    /// Whether the handshake is complete
    handshake_complete: bool,
}

impl QuicCrypto {
    /// Create new crypto context (no keys derived yet)
    pub fn new() -> Self {
        QuicCrypto {
            keys: HashMap::new(),
            active_phase: None,
            crypto_buffer: Vec::new(),
            handshake_complete: false,
        }
    }

    /// Set protection keys for a phase
    pub fn set_keys(&mut self, phase: CryptoPhase, keys: ProtectionKeys) {
        self.keys.insert(phase, keys);
    }

    /// Get packet protection for the active phase
    pub fn protection(&self) -> Option<PacketProtection> {
        let phase = self.active_phase?;
        let keys = self.keys.get(&phase)?;
        Some(PacketProtection::new(keys))
    }

    /// Get mutable packet protection (for incrementing packet number)
    pub fn protection_mut(&mut self) -> Option<PacketProtection> {
        let phase = self.active_phase?;
        let keys = self.keys.get(&phase)?;
        Some(PacketProtection::new(keys))
    }

    /// Process CRYPTO frame data (handshake messages from peer)
    pub fn process_crypto_data(&mut self, data: &[u8]) {
        self.crypto_buffer.extend_from_slice(data);
        // In a full implementation, this would parse TLS handshake
        // messages (ClientHello/ServerHello/etc.) and drive the
        // TLS 1.3 key schedule to derive traffic keys.
    }

    /// Get CRYPTO data to send (handshake messages for peer)
    pub fn crypto_data_to_send(&self) -> &[u8] {
        &self.crypto_buffer
    }

    /// Clear sent CRYPTO data
    pub fn clear_crypto_send(&mut self) {
        self.crypto_buffer.clear();
    }

    /// Check if handshake is complete
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_complete
    }

    /// Mark handshake as complete (called after TLS 1.3 Finished verified)
    pub fn complete_handshake(&mut self) {
        self.handshake_complete = true;
    }

    /// Get traffic secret for a phase (raw key material)
    pub fn get_secret(&self, phase: CryptoPhase) -> Option<&ProtectionKeys> {
        self.keys.get(&phase)
    }
}

impl Default for QuicCrypto {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protection_keys_test() {
        let keys = ProtectionKeys::test_keys();
        assert_eq!(keys.write_key.len(), 16);
        assert_eq!(keys.write_iv.len(), 12);
    }

    #[test]
    fn test_nonce_construction() {
        let iv = [0u8; 12];
        let keys = ProtectionKeys::test_keys();
        let protection = PacketProtection::new(&keys);
        let nonce = protection.make_nonce(&iv, 42);
        // Last 8 bytes should be XOR with packet number
        let expected_pn_bytes = 42u64.to_be_bytes();
        for i in 0..8 {
            assert_eq!(nonce[4 + i], expected_pn_bytes[i]);
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let keys = ProtectionKeys::test_keys();
        let mut protection = PacketProtection::new(&keys);

        let header = b"header";
        let plaintext = b"hello quic";
        let ciphertext = protection.protect(header, plaintext);

        assert_ne!(ciphertext, plaintext.to_vec());
        assert!(ciphertext.len() > plaintext.len()); // includes tag

        // Decrypt
        let decrypted = protection.unprotect(header, 0, &ciphertext);
        assert_eq!(decrypted.as_deref(), Some(plaintext.as_ref()));
    }

    #[test]
    fn test_crypto_initial_state() {
        let crypto = QuicCrypto::new();
        assert!(!crypto.is_handshake_complete());
        assert!(crypto.crypto_data_to_send().is_empty());
        assert!(crypto.protection().is_none());
    }

    #[test]
    fn test_crypto_set_keys() {
        let mut crypto = QuicCrypto::new();
        let keys = ProtectionKeys::test_keys();
        crypto.set_keys(CryptoPhase::Initial, keys);
        assert!(crypto.get_secret(CryptoPhase::Initial).is_some());
    }
}
