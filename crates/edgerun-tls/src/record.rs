//! TLS 1.3 record layer with AES-GCM encryption.
//!
//! TLS 1.3 records use AEAD encryption. The plaintext has an implicit
//! ContentType byte appended (RFC 8446 §5.4):
//!
//!   TLSCiphertext = content_type || encrypted_auth_data || AEAD-encrypted
//!
//! The AEAD nonce is computed as: nonce = write_iv XOR (sequence_number as 12 bytes)

use alloc::{format, string::{String, ToString}, vec::Vec};
use edgerun_crypto::aes_gcm::aead::generic_array::GenericArray;
use edgerun_crypto::{Aes256GcmCipher as AeadCipher, AeadInPlace};

/// TLS record layer for encryption/decryption
pub struct RecordCipher {
    cipher: AeadCipher,
    /// 96-bit nonce base (write_iv)
    iv: [u8; 12],
    /// Sequence number for record ordering
    seq: u64,
    /// AEAD tag length in bytes
    tag_len: usize,
}

impl RecordCipher {
    /// Create a new record cipher from key and IV
    pub fn new(key: &[u8], iv: &[u8]) -> Result<Self, crate::TlsError> {
        if iv.len() != 12 {
            return Err(crate::TlsError::Cipher(format!(
                "IV must be 12 bytes, got {}",
                iv.len()
            )));
        }
        let mut iv_arr = [0u8; 12];
        iv_arr.copy_from_slice(iv);

        let cipher =
            AeadCipher::new_from_key(key).map_err(|e| crate::TlsError::Cipher(e.to_string()))?;

        Ok(RecordCipher {
            cipher,
            iv: iv_arr,
            seq: 0,
            tag_len: 16, // AES-GCM tag length
        })
    }

    /// Build the TLS 1.3 record header for use as AEAD AAD (RFC 8446 §5.2).
    ///
    /// The AAD is the 5-byte TLSPlaintext header:
    ///   content_type(1) || legacy_record_version(2) || length(2)
    ///
    /// Where `length` is the length of the encrypted payload
    /// (plaintext + content_type byte + tag).
    fn build_aad(content_type: u8, plaintext_len: usize) -> [u8; 5] {
        let length = plaintext_len + 1 /* content_type byte */ + Self::TAG_LEN;
        [
            content_type,
            0x03,
            0x03, // TLS 1.2 version (middlebox compat)
            (length >> 8) as u8,
            length as u8,
        ]
    }

    /// AEAD tag length for this cipher
    const TAG_LEN: usize = 16;

    /// Encrypt a TLS 1.3 record.
    ///
    /// Per RFC 8446 §5.2, the AAD is the 5-byte TLSPlaintext header.
    /// The plaintext is first appended with the content_type byte,
    /// then encrypted with AES-GCM using the AAD.
    /// The resulting output is: ciphertext + tag.
    pub fn encrypt(&mut self, content_type: u8, plaintext: &[u8]) -> Vec<u8> {
        // TLS 1.3: plaintext = opaque_content + ContentType byte
        let mut buffer = Vec::with_capacity(plaintext.len() + 1 + Self::TAG_LEN);
        buffer.extend_from_slice(plaintext);
        buffer.push(content_type);

        // AAD is the 5-byte record header that will wrap the ciphertext
        let aad = Self::build_aad(0x17, plaintext.len()); // 0x17 = application_data

        let nonce = self.make_nonce();
        let nonce = GenericArray::from_slice(&nonce);
        let tag = self
            .cipher
            .encrypt_in_place_detached(nonce, &aad, &mut buffer)
            .expect("AEAD encryption failed");

        buffer.extend_from_slice(tag.as_ref());
        self.seq += 1;
        buffer
    }

    /// Decrypt a TLS 1.3 record.
    ///
    /// Per RFC 8446 §5.2, the AAD is the 5-byte record header
    /// (content_type=0x17, version=0x0303, length=ciphertext+tag).
    /// Returns (content_type, plaintext).
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<(u8, Vec<u8>), String> {
        if ciphertext.len() < Self::TAG_LEN {
            return Err("Ciphertext too short for AEAD tag".into());
        }

        let nonce = self.make_nonce();

        // AAD is the 5-byte record header
        let aad = Self::build_aad(0x17, ciphertext.len() - Self::TAG_LEN - 1);

        let mut buffer = ciphertext.to_vec();
        let tag_offset = buffer.len() - Self::TAG_LEN;
        let tag_bytes: [u8; 16] = buffer[tag_offset..]
            .try_into()
            .map_err(|_| "invalid tag length")?;
        let tag = edgerun_crypto::aes_gcm::Tag::from(tag_bytes);
        buffer.truncate(tag_offset);

        let nonce = GenericArray::from_slice(&nonce);
        self.cipher
            .decrypt_in_place_detached(nonce, &aad, &mut buffer, &tag)
            .map_err(|e| format!("AEAD decryption failed: {:?}", e))?;

        // Last byte is the real ContentType (RFC 8446 §5.4)
        if buffer.is_empty() {
            return Err("Decrypted plaintext is empty".into());
        }
        let content_type = buffer[buffer.len() - 1];
        let data = buffer[..buffer.len() - 1].to_vec();

        self.seq += 1;
        Ok((content_type, data))
    }

    /// Derive the per-record nonce: write_iv XOR sequence_number.
    ///
    /// Per RFC 8446 §5.3, the nonce is computed as:
    ///   nonce = iv XOR (0x00...00 || seq_number)
    ///
    /// Where the sequence number is zero-padded on the left to 12 bytes.
    /// This is equivalent to: nonce[0..4] = iv[0..4], nonce[4..12] = iv[4..12] XOR seq.
    fn make_nonce(&self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        let seq_bytes = self.seq.to_be_bytes();
        // Copy the full IV, then XOR the sequence number into the last 8 bytes
        nonce.copy_from_slice(&self.iv);
        for i in 0..8 {
            nonce[4 + i] ^= seq_bytes[i];
        }
        nonce
    }
}

/// A raw TLS record on the wire.
pub struct TlsRecord {
    /// Record content type (22=handshake, 23=application_data, 21=alert).
    pub content_type: u8,
    /// Protocol version (0x0303 for TLS 1.2 compatibility on outer records).
    pub version: u16,
    /// Encrypted or plaintext payload.
    pub fragment: Vec<u8>,
}

impl TlsRecord {
    /// Serialize to wire format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(5 + self.fragment.len());
        out.push(self.content_type);
        out.extend_from_slice(&self.version.to_be_bytes());
        out.extend_from_slice(&(self.fragment.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.fragment);
        out
    }

    /// Parse from wire
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < 5 {
            return Err("Record too short".into());
        }
        let content_type = data[0];
        let version = u16::from_be_bytes([data[1], data[2]]);
        let length = u16::from_be_bytes([data[3], data[4]]) as usize;

        if data.len() < 5 + length {
            return Err("Record fragment incomplete".into());
        }

        let fragment = data[5..5 + length].to_vec();
        Ok((
            TlsRecord {
                content_type,
                version,
                fragment,
            },
            5 + length,
        ))
    }
}
