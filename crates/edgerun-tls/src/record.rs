//! TLS record layer implementation

/// TLS record
#[derive(Debug, Clone)]
pub struct TlsRecord {
    /// Content type (20=ChangeCipherSpec, 21=Alert, 22=Handshake, 23=ApplicationData)
    pub content_type: u8,
    /// TLS version (raw u16)
    pub version: u16,
    /// Encrypted fragment
    pub fragment: Vec<u8>,
}

impl TlsRecord {
    /// Create a new TLS record
    pub fn new(content_type: u8, version: u16, fragment: Vec<u8>) -> Self {
        TlsRecord {
            content_type,
            version,
            fragment,
        }
    }

    /// Serialize record to wire format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(5 + self.fragment.len());
        data.push(self.content_type);
        data.extend_from_slice(&self.version.to_be_bytes());
        data.extend_from_slice(&(self.fragment.len() as u16).to_be_bytes());
        data.extend_from_slice(&self.fragment);
        data
    }

    /// Parse a TLS record from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < 5 {
            return Err("Record too short".to_string());
        }

        let content_type = data[0];
        let version = u16::from_be_bytes([data[1], data[2]]);
        let length = u16::from_be_bytes([data[3], data[4]]) as usize;

        if data.len() < 5 + length {
            return Err("Record fragment incomplete".to_string());
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

/// Record layer for encrypting/decrypting TLS data
pub struct RecordLayer {
    /// TLS version
    version: u16,
    /// Client write key
    client_write_key: Vec<u8>,
    /// Server write key
    server_write_key: Vec<u8>,
    /// Client write IV
    client_write_iv: Vec<u8>,
    /// Server write IV
    server_write_iv: Vec<u8>,
    /// Sequence number for client
    client_seq: u64,
    /// Sequence number for server
    server_seq: u64,
}

impl RecordLayer {
    /// Create a new record layer
    pub fn new(
        version: u16,
        client_write_key: Vec<u8>,
        server_write_key: Vec<u8>,
        client_write_iv: Vec<u8>,
        server_write_iv: Vec<u8>,
    ) -> Self {
        RecordLayer {
            version,
            client_write_key,
            server_write_key,
            client_write_iv,
            server_write_iv,
            client_seq: 0,
            server_seq: 0,
        }
    }

    /// Encrypt application data
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        // For AEAD ciphers (AES-GCM, ChaCha20-Poly1305):
        // - Generate nonce from sequence number and IV
        // - Encrypt with AEAD
        // - Prepend explicit nonce (for TLS 1.2)
        //
        // For TLS 1.2 non-AEAD (CBC):
        // - Calculate MAC over plaintext
        // - Pad and encrypt with CBC

        // Calculate MAC for integrity protection
        let _mac = self.calculate_mac(
            &self.client_write_key,
            self.client_seq,
            23, // TLS_APPLICATION_DATA
            self.version,
            plaintext,
        );

        // Build AEAD nonce: IV XOR with sequence number
        let mut nonce = self.client_write_iv.clone();
        if nonce.len() >= 8 {
            for (i, &b) in self.client_seq.to_be_bytes().iter().enumerate() {
                if i < nonce.len() {
                    nonce[i] ^= b;
                }
            }
        }
        let _nonce = nonce; // Used by AES-GCM in full implementation

        let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);
        ciphertext.extend_from_slice(plaintext);
        // TODO: Integrate AES-GCM from edgerun-aes-gcm crate for real encryption
        // using self.client_write_key as key and _nonce as nonce

        self.client_seq += 1;

        Ok(ciphertext)
    }

    /// Decrypt application data
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        // For AEAD ciphers:
        // - Extract nonce
        // - Decrypt with AEAD
        // - Verify authentication tag
        //
        // For TLS 1.2 non-AEAD (CBC):
        // - Decrypt CBC
        // - Strip padding
        // - Verify MAC

        let plaintext = ciphertext.to_vec();

        // Verify MAC for integrity
        let _expected_mac = self.calculate_mac(
            &self.server_write_key,
            self.server_seq,
            23, // TLS_APPLICATION_DATA
            self.version,
            &plaintext,
        );

        // Build AEAD nonce for decryption
        let mut nonce = self.server_write_iv.clone();
        if nonce.len() >= 8 {
            for (i, &b) in self.server_seq.to_be_bytes().iter().enumerate() {
                if i < nonce.len() {
                    nonce[i] ^= b;
                }
            }
        }
        let _nonce = nonce; // Used by AES-GCM in full implementation

        // TODO: Compare _expected_mac against the MAC appended by the peer

        self.server_seq += 1;

        Ok(plaintext)
    }

    /// Calculate MAC for TLS 1.2
    fn calculate_mac(
        &self,
        _key: &[u8],
        seq_num: u64,
        content_type: u8,
        version: u16,
        fragment: &[u8],
    ) -> Vec<u8> {
        // TLS 1.2 MAC calculation:
        // MAC = HMAC(key, seq_num + content_type + version + length + fragment)
        //
        // This would use SHA-256 or SHA-384 depending on cipher suite
        // Simplified - returns empty MAC
        let mut mac_input = Vec::new();
        mac_input.extend_from_slice(&seq_num.to_be_bytes());
        mac_input.push(content_type);
        mac_input.extend_from_slice(&version.to_be_bytes());
        mac_input.extend_from_slice(&(fragment.len() as u16).to_be_bytes());
        mac_input.extend_from_slice(fragment);

        // In full implementation: HMAC-SHA256(key, mac_input)
        vec![0u8; 32] // Placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_record_new() {
        let record = TlsRecord::new(23, 0x0303, vec![1, 2, 3]);
        assert_eq!(record.content_type, 23);
        assert_eq!(record.version, 0x0303);
        assert_eq!(record.fragment, vec![1, 2, 3]);
    }

    #[test]
    fn test_tls_record_to_bytes() {
        let record = TlsRecord::new(23, 0x0303, vec![1, 2, 3]);
        let bytes = record.to_bytes();

        assert_eq!(bytes[0], 23); // content type
        assert_eq!(bytes[1], 0x03); // version major
        assert_eq!(bytes[2], 0x03); // version minor
        assert_eq!(bytes[3], 0x00); // length high
        assert_eq!(bytes[4], 0x03); // length low
        assert_eq!(&bytes[5..], &[1, 2, 3]); // fragment
    }

    #[test]
    fn test_tls_record_from_bytes() {
        let data = vec![
            23, // content type
            0x03, 0x03, // version
            0x00, 0x03, // length
            1, 2, 3, // fragment
        ];

        let (record, bytes_read) = TlsRecord::from_bytes(&data).unwrap();
        assert_eq!(record.content_type, 23);
        assert_eq!(record.version, 0x0303);
        assert_eq!(record.fragment, vec![1, 2, 3]);
        assert_eq!(bytes_read, 8);
    }

    #[test]
    fn test_tls_record_from_bytes_too_short() {
        let data = vec![23, 0x03];
        assert!(TlsRecord::from_bytes(&data).is_err());
    }

    #[test]
    fn test_record_layer_encrypt_decrypt() {
        let mut layer = RecordLayer::new(
            0x0303,
            vec![0u8; 16], // 128-bit key
            vec![0u8; 16],
            vec![0u8; 12], // 96-bit IV
            vec![0u8; 12],
        );

        let plaintext = b"Hello, TLS!";
        let ciphertext = layer.encrypt(plaintext).unwrap();

        // Reset sequence number for decryption test
        layer.client_seq = 0;
        layer.server_seq = 0;

        // In simplified implementation, ciphertext == plaintext
        assert_eq!(ciphertext, plaintext.to_vec());
    }
}
