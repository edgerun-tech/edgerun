//! TLS 1.3 record layer with AES-GCM encryption.
//!
//! TLS 1.3 records use AEAD encryption. The plaintext has an implicit
//! ContentType byte appended (RFC 8446 §5.4):
//!
//!   TLSCiphertext = content_type || encrypted_auth_data || AEAD-encrypted
//!
//! The AEAD nonce is computed as: nonce = write_iv XOR (sequence_number as 12 bytes)

use aes_gcm::{
    aead::{AeadInPlace, KeyInit},
    Aes128Gcm, Aes256Gcm, Nonce,
};

/// TLS record layer for encryption/decryption
pub struct RecordCipher {
    inner: CipherImpl,
    /// 96-bit nonce base (write_iv)
    iv: [u8; 12],
    /// Sequence number for record ordering
    seq: u64,
}

enum CipherImpl {
    Aes128(Aes128Gcm),
    Aes256(Aes256Gcm),
}

impl RecordCipher {
    /// Create a new record cipher from key and IV
    pub fn new(key: &[u8], iv: &[u8]) -> Result<Self, String> {
        if iv.len() != 12 {
            return Err(format!("IV must be 12 bytes, got {}", iv.len()));
        }
        let mut iv_arr = [0u8; 12];
        iv_arr.copy_from_slice(iv);

        let inner = match key.len() {
            16 => CipherImpl::Aes128(
                Aes128Gcm::new_from_slice(key).map_err(|e| format!("Invalid AES-128 key: {:?}", e))?,
            ),
            32 => CipherImpl::Aes256(
                Aes256Gcm::new_from_slice(key).map_err(|e| format!("Invalid AES-256 key: {:?}", e))?,
            ),
            _ => return Err(format!("Unsupported key length: {} bytes", key.len())),
        };

        Ok(RecordCipher { inner, iv: iv_arr, seq: 0 })
    }

    /// Encrypt a TLS 1.3 application data record.
    ///
    /// The plaintext is first appended with the content_type byte (23 = application_data),
    /// then encrypted with AES-GCM. The resulting ciphertext is the AEAD ciphertext + tag.
    pub fn encrypt(&mut self, content_type: u8, plaintext: &[u8]) -> Vec<u8> {
        // TLS 1.3: plaintext = opaque_content + ContentType byte
        let mut buffer = Vec::with_capacity(plaintext.len() + 1 + 16);
        buffer.extend_from_slice(plaintext);
        buffer.push(content_type);

        let nonce = self.make_nonce();
        let tag = match &self.inner {
            CipherImpl::Aes128(cipher) => cipher
                .encrypt_in_place_detached(&nonce, &[], &mut buffer)
                .expect("AEAD encryption failed"),
            CipherImpl::Aes256(cipher) => cipher
                .encrypt_in_place_detached(&nonce, &[], &mut buffer)
                .expect("AEAD encryption failed"),
        };

        buffer.extend_from_slice(tag.as_slice());
        self.seq += 1;
        buffer
    }

    /// Decrypt a TLS 1.3 record.
    /// Returns (content_type, plaintext).
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<(u8, Vec<u8>), String> {
        if ciphertext.len() < 16 {
            return Err("Ciphertext too short for AEAD tag".into());
        }

        let nonce = self.make_nonce();
        let mut buffer = ciphertext.to_vec();
        let tag_offset = buffer.len() - 16;
        let tag: aes_gcm::Tag = aes_gcm::Tag::clone_from_slice(&buffer[tag_offset..]);
        buffer.truncate(tag_offset);

        match &self.inner {
            CipherImpl::Aes128(cipher) => cipher
                .decrypt_in_place_detached(&nonce, &[], &mut buffer, &tag)
                .map_err(|e| format!("AEAD decryption failed: {:?}", e))?,
            CipherImpl::Aes256(cipher) => cipher
                .decrypt_in_place_detached(&nonce, &[], &mut buffer, &tag)
                .map_err(|e| format!("AEAD decryption failed: {:?}", e))?,
        };

        // Last byte is the real ContentType (RFC 8446 §5.4)
        if buffer.is_empty() {
            return Err("Decrypted plaintext is empty".into());
        }
        let content_type = buffer[buffer.len() - 1];
        let data = buffer[..buffer.len() - 1].to_vec();

        self.seq += 1;
        Ok((content_type, data))
    }

    /// Derive the per-record nonce: write_iv XOR sequence_number
    fn make_nonce(&self) -> Nonce<aes_gcm::aead::consts::U12> {
        let mut nonce = [0u8; 12];
        let seq_bytes = self.seq.to_be_bytes();
        for i in 0..8 {
            nonce[4 + i] = self.iv[4 + i] ^ seq_bytes[i];
        }
        Nonce::from(nonce)
    }
}

/// Parse a raw TLS record from the wire (unencrypted handshake records or outer encrypted records)
pub struct TlsRecord {
    pub content_type: u8,
    pub version: u16,
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
        Ok((TlsRecord { content_type, version, fragment }, 5 + length))
    }
}
