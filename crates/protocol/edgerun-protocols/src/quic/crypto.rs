//! QUIC packet protection (RFC 9001)
//!
//! Implements QUIC packet payload protection with AES-GCM plus RFC 9001
//! AES header protection. The QUIC-TLS handshake adapter lives in the
//! handshake modules and HTTP/3 integration layer.
//!
//! This module provides:
//! - `PacketProtection` — AEAD encrypt/decrypt with QUIC-specific nonce and AAD,
//!   plus QUIC header protection mask/unmask
//! - `ProtectionKeys` — derived traffic keys for Initial/Handshake/1-RTT levels
//! - Hardcoded test keys for unit testing the packet layer

use alloc::{
    collections::BTreeMap as HashMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use edgerun_crypto::aes::{Aes128, Aes256};
use edgerun_crypto::aes_gcm::aead::{AeadInPlace, KeyInit};
use edgerun_crypto::aes_gcm::{self, Aes128Gcm, Aes256Gcm};
use edgerun_crypto::CipherSuite;

/// Crypto phase / encryption level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CryptoPhase {
    /// Initial keys (derived from initial salt + destination CID)
    Initial,
    /// Handshake keys (derived from ECDH shared secret)
    Handshake,
    /// Application data keys (1-RTT, derived from master secret)
    Application,
}

#[derive(Clone)]
enum QuicAead {
    Aes128(Aes128Gcm),
    Aes256(Aes256Gcm),
}

impl QuicAead {
    fn new(suite: CipherSuite, key: &[u8]) -> Result<Self, aes_gcm::aead::Error> {
        match suite {
            CipherSuite::TLS_AES_128_GCM_SHA256 => {
                let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| aes_gcm::aead::Error)?;
                Ok(Self::Aes128(cipher))
            }
            CipherSuite::TLS_AES_256_GCM_SHA384 => {
                let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| aes_gcm::aead::Error)?;
                Ok(Self::Aes256(cipher))
            }
        }
    }

    fn encrypt_in_place_detached(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<aes_gcm::Tag, aes_gcm::aead::Error> {
        let nonce = aes_gcm::Nonce::from_slice(nonce);
        match self {
            Self::Aes128(cipher) => cipher.encrypt_in_place_detached(nonce, aad, buffer),
            Self::Aes256(cipher) => cipher.encrypt_in_place_detached(nonce, aad, buffer),
        }
    }

    fn decrypt_in_place_detached(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
        tag: &aes_gcm::Tag,
    ) -> Result<(), aes_gcm::aead::Error> {
        let nonce = aes_gcm::Nonce::from_slice(nonce);
        match self {
            Self::Aes128(cipher) => cipher.decrypt_in_place_detached(nonce, aad, buffer, tag),
            Self::Aes256(cipher) => cipher.decrypt_in_place_detached(nonce, aad, buffer, tag),
        }
    }
}

#[derive(Clone)]
enum HeaderProtectionCipher {
    Aes128(Aes128),
    Aes256(Aes256),
}

impl HeaderProtectionCipher {
    fn new(suite: CipherSuite, key: &[u8]) -> Result<Self, String> {
        match suite {
            CipherSuite::TLS_AES_128_GCM_SHA256 => {
                if key.len() != 16 {
                    return Err(format!(
                        "AES-128 header protection key must be 16 bytes, got {}",
                        key.len()
                    ));
                }
                let mut key_bytes = [0u8; 16];
                key_bytes.copy_from_slice(key);
                Ok(Self::Aes128(Aes128::new(&key_bytes)))
            }
            CipherSuite::TLS_AES_256_GCM_SHA384 => {
                if key.len() != 32 {
                    return Err(format!(
                        "AES-256 header protection key must be 32 bytes, got {}",
                        key.len()
                    ));
                }
                let mut key_bytes = [0u8; 32];
                key_bytes.copy_from_slice(key);
                Ok(Self::Aes256(Aes256::new(&key_bytes)))
            }
        }
    }

    fn mask(&self, sample: &[u8]) -> Result<[u8; 5], String> {
        if sample.len() != 16 {
            return Err(format!(
                "header protection sample must be 16 bytes, got {}",
                sample.len()
            ));
        }

        let mut block = [0u8; 16];
        block.copy_from_slice(sample);
        let encrypted = match self {
            Self::Aes128(cipher) => cipher.encrypt_block(&block),
            Self::Aes256(cipher) => cipher.encrypt_block(&block),
        };

        let mut mask = [0u8; 5];
        mask.copy_from_slice(&encrypted[..5]);
        Ok(mask)
    }
}

impl CryptoPhase {
    pub fn preferred() -> CipherSuite {
        CipherSuite::TLS_AES_128_GCM_SHA256
    }
}

/// Keys for protecting packets at a given level
#[derive(Clone)]
pub struct ProtectionKeys {
    /// AEAD algorithm
    pub algorithm: CipherSuite,
    /// Write (encrypt) key
    pub write_key: Vec<u8>,
    /// Write IV (for nonce derivation)
    pub write_iv: Vec<u8>,
    /// Read (decrypt) key
    pub read_key: Vec<u8>,
    /// Read IV
    pub read_iv: Vec<u8>,
    /// Write header protection key
    pub write_hp_key: Vec<u8>,
    /// Read header protection key
    pub read_hp_key: Vec<u8>,
}

impl ProtectionKeys {
    /// Create from raw key material
    pub fn new(
        algorithm: CipherSuite,
        write_key: Vec<u8>,
        write_iv: Vec<u8>,
        read_key: Vec<u8>,
        read_iv: Vec<u8>,
    ) -> Self {
        ProtectionKeys {
            algorithm,
            write_hp_key: write_key.clone(),
            read_hp_key: read_key.clone(),
            write_key,
            write_iv,
            read_key,
            read_iv,
        }
    }

    /// Attach explicit header protection keys derived with QUIC label `"hp"`.
    pub fn with_header_protection(mut self, write_hp_key: Vec<u8>, read_hp_key: Vec<u8>) -> Self {
        self.write_hp_key = write_hp_key;
        self.read_hp_key = read_hp_key;
        self
    }

    /// Create test keys (all zeros) for unit testing
    pub fn test_keys() -> Self {
        let key_len = CipherSuite::TLS_AES_128_GCM_SHA256.key_len();
        let iv_len = 12;
        ProtectionKeys {
            algorithm: CipherSuite::TLS_AES_128_GCM_SHA256,
            write_key: vec![0u8; key_len],
            write_iv: vec![0u8; iv_len],
            read_key: vec![0u8; key_len],
            read_iv: vec![0u8; iv_len],
            write_hp_key: vec![0u8; key_len],
            read_hp_key: vec![0u8; key_len],
        }
    }
}

/// QUIC packet protection engine
pub struct PacketProtection {
    /// Encryption AEAD
    write_aead: QuicAead,
    /// Decryption AEAD
    read_aead: QuicAead,
    /// Header protection cipher for outgoing packets
    write_hp: HeaderProtectionCipher,
    /// Header protection cipher for incoming packets
    read_hp: HeaderProtectionCipher,
    /// Write IV
    write_iv: Vec<u8>,
    /// Read IV
    read_iv: Vec<u8>,
    /// Packet number for nonce generation
    packet_number: u64,
}

impl PacketProtection {
    /// Create from protection keys
    pub fn new(keys: &ProtectionKeys) -> Self {
        let write_aead = QuicAead::new(keys.algorithm, &keys.write_key).expect("valid write key");
        let read_aead = QuicAead::new(keys.algorithm, &keys.read_key).expect("valid read key");
        let write_hp = HeaderProtectionCipher::new(keys.algorithm, &keys.write_hp_key)
            .expect("valid write header protection key");
        let read_hp = HeaderProtectionCipher::new(keys.algorithm, &keys.read_hp_key)
            .expect("valid read header protection key");

        PacketProtection {
            write_aead,
            read_aead,
            write_hp,
            read_hp,
            write_iv: keys.write_iv.clone(),
            read_iv: keys.read_iv.clone(),
            packet_number: 0,
        }
    }

    /// Protect (encrypt) a packet payload
    ///
    /// AAD = unprotected packet header (authenticated but not encrypted)
    /// Nonce = write_iv XOR (packet_number << 8)
    pub fn protect(&mut self, header: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let pn = self.packet_number;
        self.packet_number += 1;
        self.protect_with_nonce_packet_number(pn, header, plaintext)
    }

    /// Protect a packet payload using an explicit packet number.
    ///
    /// Packet number is already serialized in the QUIC header. Callers that
    /// allocate packet numbers outside this type must use the same value for
    /// nonce construction or the peer will be unable to decrypt the packet.
    pub fn protect_with_packet_number(
        &mut self,
        packet_number: u64,
        header: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, String> {
        if packet_number >= self.packet_number {
            self.packet_number = packet_number + 1;
        }
        self.protect_with_nonce_packet_number(packet_number, header, plaintext)
    }

    fn protect_with_nonce_packet_number(
        &self,
        packet_number: u64,
        header: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, String> {
        let nonce = self.make_nonce(&self.write_iv, packet_number);

        let mut buffer = plaintext.to_vec();

        let tag = self
            .write_aead
            .encrypt_in_place_detached(&nonce, header, &mut buffer)
            .map_err(|e| format!("AEAD encrypt failed: {:?}", e))?;

        buffer.extend_from_slice(tag.as_slice());

        Ok(buffer)
    }

    /// Unprotect (decrypt) a packet payload
    ///
    /// AAD = unprotected packet header (authenticated but not encrypted)
    /// Nonce = read_iv XOR (packet_number << 8)
    pub fn unprotect(
        &mut self,
        header: &[u8],
        packet_number: u64,
        ciphertext_and_tag: &[u8],
    ) -> Result<Vec<u8>, String> {
        let nonce = self.make_nonce(&self.read_iv, packet_number);

        if ciphertext_and_tag.len() < 16 {
            return Err("Ciphertext too short for AEAD tag".to_string());
        }
        let tag_start = ciphertext_and_tag.len() - 16;
        let tag = aes_gcm::Tag::from_slice(&ciphertext_and_tag[tag_start..]);
        let mut buffer = ciphertext_and_tag[..tag_start].to_vec();

        self.read_aead
            .decrypt_in_place_detached(&nonce, header, &mut buffer, tag)
            .map_err(|e| format!("AEAD decrypt failed: {:?}", e))?;

        Ok(buffer)
    }

    /// Apply QUIC header protection to a full packet after payload AEAD.
    ///
    /// `pn_offset` is the byte offset of the encoded packet number and
    /// `pn_len` is the encoded packet number length in bytes. The packet must
    /// already contain ciphertext because the RFC 9001 sample starts four bytes
    /// after the packet number offset.
    pub fn protect_header(
        &self,
        packet: &mut [u8],
        pn_offset: usize,
        pn_len: usize,
    ) -> Result<(), String> {
        if !(1..=4).contains(&pn_len) {
            return Err(format!(
                "packet number length must be 1..=4, got {}",
                pn_len
            ));
        }
        if pn_offset
            .checked_add(pn_len)
            .filter(|end| *end <= packet.len())
            .is_none()
        {
            return Err("packet number exceeds packet length".to_string());
        }

        let mask = self.header_protection_mask(packet, pn_offset, true)?;
        if packet[0] & 0x80 != 0 {
            packet[0] ^= mask[0] & 0x0f;
        } else {
            packet[0] ^= mask[0] & 0x1f;
        }
        for i in 0..pn_len {
            packet[pn_offset + i] ^= mask[i + 1];
        }
        Ok(())
    }

    /// Remove QUIC header protection from a full packet before parsing.
    ///
    /// Returns the unmasked packet number length. The sample offset is
    /// `pn_offset + 4`, so this can recover the length bits even though they
    /// are masked in the first byte.
    pub fn unprotect_header(&self, packet: &mut [u8], pn_offset: usize) -> Result<usize, String> {
        if packet.is_empty() {
            return Err("packet is empty".to_string());
        }
        if pn_offset >= packet.len() {
            return Err("packet number offset exceeds packet length".to_string());
        }

        let mask = self.header_protection_mask(packet, pn_offset, false)?;
        if packet[0] & 0x80 != 0 {
            packet[0] ^= mask[0] & 0x0f;
        } else {
            packet[0] ^= mask[0] & 0x1f;
        }

        let pn_len = ((packet[0] & 0x03) + 1) as usize;
        if pn_offset + pn_len > packet.len() {
            return Err("packet number exceeds packet length".to_string());
        }
        for i in 0..pn_len {
            packet[pn_offset + i] ^= mask[i + 1];
        }
        Ok(pn_len)
    }

    fn header_protection_mask(
        &self,
        packet: &[u8],
        pn_offset: usize,
        write: bool,
    ) -> Result<[u8; 5], String> {
        let sample_offset = pn_offset
            .checked_add(4)
            .ok_or_else(|| "header protection sample offset overflow".to_string())?;
        let sample_end = sample_offset
            .checked_add(16)
            .ok_or_else(|| "header protection sample end overflow".to_string())?;
        let sample = packet
            .get(sample_offset..sample_end)
            .ok_or_else(|| "packet too short for header protection sample".to_string())?;

        if write {
            self.write_hp.mask(sample)
        } else {
            self.read_hp.mask(sample)
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

    /// Make nonce as a standalone function for unit testing
    pub fn test_make_nonce(iv: &[u8], packet_number: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(iv);
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
/// NOTE: This is a low-level key container and packet-protection context. The
/// QUIC-TLS adapter that derives real Initial, Handshake, 0-RTT, and 1-RTT
/// keys is implemented in the handshake modules and wired by HTTP/3.
///
/// What IS implemented:
/// - `PacketProtection` — AEAD encrypt/decrypt with proper QUIC nonce and header protection
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
        assert_eq!(keys.write_hp_key.len(), 16);
        assert_eq!(keys.read_hp_key.len(), 16);
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
    fn test_client_server_nonce_inverse() {
        // Client uses server_in secret for encryption, server uses client_in for decryption
        // But both should XOR with same packet number
        let client_iv = [
            0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7a, 0x8b, 0x9c, 0xad, 0xbe, 0xcf,
        ];
        let server_iv = [
            0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7a, 0x8b, 0x9c, 0xad, 0xbe, 0xcf,
        ];

        let client_nonce = PacketProtection::test_make_nonce(&client_iv, 0);
        let server_nonce = PacketProtection::test_make_nonce(&server_iv, 0);

        // Both start with same IV, XOR with pn=0 means no change
        assert_eq!(client_nonce, client_iv);
        assert_eq!(server_nonce, server_iv);
    }

    #[test]
    fn test_symmetric_encrypt_decrypt() {
        // Use same keys for encrypt and decrypt - should roundtrip
        let keys = ProtectionKeys::test_keys();
        let mut encryptor = PacketProtection::new(&keys);
        let mut decryptor = PacketProtection::new(&keys);

        let header = b"test header";
        let plaintext = b"Hello QUIC!";

        let ciphertext = encryptor.protect(header, plaintext).unwrap();
        let decrypted = decryptor.unprotect(header, 0, &ciphertext).unwrap();

        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let keys = ProtectionKeys::test_keys();
        let mut protection = PacketProtection::new(&keys);

        let header = b"header";
        let plaintext = b"hello quic";
        let ciphertext = protection
            .protect(header, plaintext)
            .expect("encrypt failed");

        assert_ne!(ciphertext, plaintext.to_vec());
        assert!(ciphertext.len() > plaintext.len()); // includes tag

        // Decrypt
        let decrypted = protection
            .unprotect(header, 0, &ciphertext)
            .expect("decrypt failed");
        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_with_explicit_packet_number() {
        let keys = ProtectionKeys::test_keys();
        let mut encryptor = PacketProtection::new(&keys);
        let mut decryptor = PacketProtection::new(&keys);

        let header = b"header with packet number 7";
        let plaintext = b"payload";
        let ciphertext = encryptor
            .protect_with_packet_number(7, header, plaintext)
            .expect("encrypt failed");

        assert!(decryptor.unprotect(header, 6, &ciphertext).is_err());
        let decrypted = decryptor
            .unprotect(header, 7, &ciphertext)
            .expect("decrypt failed");
        assert_eq!(decrypted.as_slice(), plaintext.as_slice());
        assert_eq!(encryptor.packet_number(), 8);
    }

    #[test]
    fn test_long_header_protection_roundtrip() {
        let keys = ProtectionKeys::test_keys();
        let protection = PacketProtection::new(&keys);
        let mut packet = vec![
            0xcf, 0x00, 0x00, 0x00, 0x01, 0x08, 0, 1, 2, 3, 4, 5, 6, 7, 0x00, 0x14, 0x00, 0x00,
            0x00, 0x07,
        ];
        packet.extend_from_slice(&[0x42; 32]);
        let original = packet.clone();
        let pn_offset = 16;

        protection
            .protect_header(&mut packet, pn_offset, 4)
            .expect("protect header");
        assert_ne!(packet[0], original[0]);
        assert_ne!(
            &packet[pn_offset..pn_offset + 4],
            &original[pn_offset..pn_offset + 4]
        );

        let pn_len = protection
            .unprotect_header(&mut packet, pn_offset)
            .expect("unprotect header");
        assert_eq!(pn_len, 4);
        assert_eq!(packet, original);
    }

    #[test]
    fn test_short_header_protection_roundtrip() {
        let keys = ProtectionKeys::test_keys();
        let protection = PacketProtection::new(&keys);
        let mut packet = vec![0x45, 1, 2, 3, 4, 0xab, 0xcd];
        packet.extend_from_slice(&[0x24; 32]);
        let original = packet.clone();
        let pn_offset = 5;

        protection
            .protect_header(&mut packet, pn_offset, 2)
            .expect("protect header");
        assert_ne!(packet[0], original[0]);
        assert_ne!(
            &packet[pn_offset..pn_offset + 2],
            &original[pn_offset..pn_offset + 2]
        );

        let pn_len = protection
            .unprotect_header(&mut packet, pn_offset)
            .expect("unprotect header");
        assert_eq!(pn_len, 2);
        assert_eq!(packet, original);
    }

    #[test]
    fn test_header_protection_requires_sample() {
        let keys = ProtectionKeys::test_keys();
        let protection = PacketProtection::new(&keys);
        let mut packet = vec![0x41, 1, 2, 3, 4, 0x07];

        let err = protection.protect_header(&mut packet, 5, 1).unwrap_err();
        assert!(err.contains("sample"));
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
