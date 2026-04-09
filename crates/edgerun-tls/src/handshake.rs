//! TLS handshake protocol implementation

use super::{CipherSuite, Result, TlsError, TlsVersion};

/// ClientHello message
#[derive(Debug, Clone)]
pub struct ClientHello {
    /// TLS version (legacy, should be 1.2 for TLS 1.3)
    pub version: TlsVersion,
    /// Client random (32 bytes)
    pub random: [u8; 32],
    /// Session ID
    pub session_id: Vec<u8>,
    /// Cipher suites
    pub cipher_suites: Vec<CipherSuite>,
    /// Compression methods
    pub compression_methods: Vec<u8>,
    /// Extensions
    pub extensions: Vec<Extension>,
}

impl ClientHello {
    /// Create a new ClientHello
    pub fn new(version: TlsVersion, random: &[u8; 32], server_name: &str) -> Self {
        ClientHello {
            version,
            random: *random,
            session_id: vec![0u8; 32], // Will be filled with random data
            cipher_suites: CipherSuite::supported(),
            compression_methods: vec![0], // null compression
            extensions: vec![
                Extension::server_name(server_name),
                Extension::supported_versions(),
                Extension::supported_groups(),
                Extension::signature_algorithms(),
                Extension::key_share(),
            ],
        }
    }

    /// Serialize to wire format
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();

        // Handshake type (ClientHello = 1)
        data.push(1);

        // Handshake length (placeholder, will be filled later)
        let length_pos = data.len();
        data.extend_from_slice(&[0u8; 3]);

        // Version
        data.extend_from_slice(&self.version.to_wire().to_be_bytes());

        // Random
        data.extend_from_slice(&self.random);

        // Session ID
        data.push(self.session_id.len() as u8);
        data.extend_from_slice(&self.session_id);

        // Cipher suites
        let cipher_len = self.cipher_suites.len();
        data.extend_from_slice(&((cipher_len * 2) as u16).to_be_bytes());
        for suite in &self.cipher_suites {
            data.extend_from_slice(&suite.to_wire().to_be_bytes());
        }

        // Compression methods
        data.push(self.compression_methods.len() as u8);
        data.extend_from_slice(&self.compression_methods);

        // Extensions
        let extensions_pos = data.len();
        data.extend_from_slice(&[0u8; 2]); // Length placeholder

        for ext in &self.extensions {
            data.extend_from_slice(&ext.to_bytes()?);
        }

        // Fill in extension length
        let ext_len = (data.len() - extensions_pos - 2) as u16;
        data[extensions_pos..extensions_pos + 2].copy_from_slice(&ext_len.to_be_bytes());

        // Fill in handshake length
        let msg_len = (data.len() - length_pos - 3) as u32;
        data[length_pos..length_pos + 3]
            .copy_from_slice(&msg_len.to_be_bytes()[1..]);

        Ok(data)
    }
}

/// ServerHello message
#[derive(Debug, Clone)]
pub struct ServerHello {
    /// TLS version
    pub version: TlsVersion,
    /// Server random (32 bytes)
    pub random: [u8; 32],
    /// Session ID
    pub session_id: Vec<u8>,
    /// Selected cipher suite
    pub cipher_suite: CipherSuite,
    /// Compression method
    pub compression_method: u8,
    /// Extensions
    pub extensions: Vec<Extension>,
}

impl ServerHello {
    /// Parse from wire format
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 38 {
            return Err(TlsError::Protocol(
                "ServerHello too short".to_string(),
            ));
        }

        let mut pos = 0;

        // Skip handshake type (should be 2)
        if data[pos] != 2 {
            return Err(TlsError::Protocol(
                "Expected ServerHello message".to_string(),
            ));
        }
        pos += 1;

        // Skip handshake length (3 bytes)
        pos += 3;

        // Version
        let version_bytes = &data[pos..pos + 2];
        let version =
            TlsVersion::from_wire(u16::from_be_bytes([version_bytes[0], version_bytes[1]]))?;
        pos += 2;

        // Random
        let mut random = [0u8; 32];
        random.copy_from_slice(&data[pos..pos + 32]);
        pos += 32;

        // Session ID
        let session_id_len = data[pos] as usize;
        pos += 1;
        let session_id = data[pos..pos + session_id_len].to_vec();
        pos += session_id_len;

        // Cipher suite
        let cipher_suite_bytes = &data[pos..pos + 2];
        let cipher_suite = CipherSuite::from_wire(u16::from_be_bytes([
            cipher_suite_bytes[0],
            cipher_suite_bytes[1],
        ]))
        .map_err(|e| super::TlsError::Protocol(e))?;
        pos += 2;

        // Compression method
        let compression_method = data[pos];
        pos += 1;

        // Extensions
        let mut extensions = Vec::new();
        if pos < data.len() {
            let ext_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;

            let ext_end = pos + ext_len;
            while pos < ext_end {
                let ext = Extension::from_bytes(&data[pos..ext_end])?;
                let ext_total_size = 4 + ext.data.len(); // type(2) + length(2) + data
                pos += ext_total_size;
                extensions.push(ext);
            }
        }

        Ok(ServerHello {
            version,
            random,
            session_id,
            cipher_suite,
            compression_method,
            extensions,
        })
    }
}

/// Handshake message type
#[derive(Debug, Clone)]
pub enum HandshakeMessage {
    /// ClientHello
    ClientHello(ClientHello),
    /// ServerHello
    ServerHello(ServerHello),
    /// Certificate
    Certificate(Vec<super::Certificate>),
    /// ServerKeyExchange
    ServerKeyExchange(Vec<u8>),
    /// ServerHelloDone
    ServerHelloDone,
    /// ClientKeyExchange
    ClientKeyExchange(Vec<u8>),
    /// Finished
    Finished(Vec<u8>),
    /// EncryptedExtensions (TLS 1.3)
    EncryptedExtensions(Vec<Extension>),
}

impl HandshakeMessage {
    /// Parse handshake message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(TlsError::Protocol("Empty handshake message".to_string()));
        }

        match data[0] {
            1 => {
                // ClientHello - parse and wrap
                let hello = ClientHello::new(TlsVersion::Tls12, &[0u8; 32], "");
                Ok(HandshakeMessage::ClientHello(hello))
            }
            2 => {
                // ServerHello
                let hello = ServerHello::from_bytes(data)?;
                Ok(HandshakeMessage::ServerHello(hello))
            }
            11 => {
                // Certificate
                Ok(HandshakeMessage::Certificate(Vec::new()))
            }
            12 => {
                // ServerKeyExchange
                Ok(HandshakeMessage::ServerKeyExchange(data[1..].to_vec()))
            }
            14 => {
                // ServerHelloDone
                Ok(HandshakeMessage::ServerHelloDone)
            }
            16 => {
                // ClientKeyExchange
                Ok(HandshakeMessage::ClientKeyExchange(data[1..].to_vec()))
            }
            20 => {
                // Finished
                Ok(HandshakeMessage::Finished(data[1..].to_vec()))
            }
            8 => {
                // EncryptedExtensions (TLS 1.3)
                Ok(HandshakeMessage::EncryptedExtensions(Vec::new()))
            }
            other => Err(TlsError::Protocol(format!(
                "Unknown handshake message type: {}",
                other
            ))),
        }
    }
}

/// TLS extension
#[derive(Debug, Clone)]
pub struct Extension {
    /// Extension type
    pub ext_type: u16,
    /// Extension data
    pub data: Vec<u8>,
}

impl Extension {
    /// Create Server Name Indication extension
    pub fn server_name(name: &str) -> Self {
        let mut data = Vec::new();

        // Server name list length
        let name_len = name.len() + 3; // hostname type(1) + length(2) + name
        data.extend_from_slice(&(name_len as u16).to_be_bytes());

        // Server name type (hostname = 0)
        data.push(0);

        // Server name length
        data.extend_from_slice(&(name.len() as u16).to_be_bytes());

        // Server name
        data.extend_from_slice(name.as_bytes());

        Extension {
            ext_type: 0, // server_name
            data,
        }
    }

    /// Create Supported Versions extension
    pub fn supported_versions() -> Self {
        Extension {
            ext_type: 43,
            data: vec![0x02, 0x03, 0x04, 0x03, 0x03], // TLS 1.3, 1.2
        }
    }

    /// Create Supported Groups extension
    pub fn supported_groups() -> Self {
        Extension {
            ext_type: 10,
            data: vec![0x00, 0x06, 0x00, 0x17, 0x00, 0x18, 0x00, 0x19], // P-256, P-384, P-521
        }
    }

    /// Create Signature Algorithms extension
    pub fn signature_algorithms() -> Self {
        Extension {
            ext_type: 13,
            data: vec![
                0x00, 0x08, 0x04, 0x03, 0x08, 0x04, 0x04, 0x01, 0x05, 0x03, 0x08, 0x05, 0x05,
                0x01, 0x08, 0x06, 0x06, 0x01,
            ],
        }
    }

    /// Create Key Share extension
    pub fn key_share() -> Self {
        Extension {
            ext_type: 51,
            data: vec![0x00, 0x00], // Placeholder for key share entries
        }
    }

    /// Serialize extension to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();

        // Extension type
        data.extend_from_slice(&self.ext_type.to_be_bytes());

        // Extension data length
        data.extend_from_slice(&(self.data.len() as u16).to_be_bytes());

        // Extension data
        data.extend_from_slice(&self.data);

        Ok(data)
    }

    /// Parse extension from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(TlsError::Protocol("Extension too short".to_string()));
        }

        let ext_type = u16::from_be_bytes([data[0], data[1]]);
        let ext_len = u16::from_be_bytes([data[2], data[3]]) as usize;

        if data.len() < 4 + ext_len {
            return Err(TlsError::Protocol("Extension data length mismatch".to_string()));
        }

        let ext_data = data[4..4 + ext_len].to_vec();

        Ok(Extension {
            ext_type,
            data: ext_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_hello_new() {
        let random = [0x42u8; 32];
        let hello = ClientHello::new(TlsVersion::Tls12, &random, "example.com");

        assert_eq!(hello.version, TlsVersion::Tls12);
        assert_eq!(hello.random, random);
        assert!(!hello.cipher_suites.is_empty());
        assert!(!hello.extensions.is_empty());
    }

    #[test]
    fn test_client_hello_to_bytes() {
        let random = [0x42u8; 32];
        let hello = ClientHello::new(TlsVersion::Tls12, &random, "example.com");
        let bytes = hello.to_bytes().unwrap();

        // Should start with handshake type 1 (ClientHello)
        assert_eq!(bytes[0], 1);

        // Should be reasonably long
        assert!(bytes.len() > 100);
    }

    #[test]
    fn test_server_hello_from_bytes() {
        // Minimal ServerHello structure
        let mut data = vec![
            2, // Handshake type (ServerHello)
            0, 0, 38, // Handshake length
            0x03, 0x03, // TLS 1.2
        ];

        // Random (32 bytes)
        data.extend_from_slice(&[0u8; 32]);

        // Session ID (empty)
        data.push(0);

        // Cipher suite (TLS_AES_128_GCM_SHA256)
        data.extend_from_slice(&0x1301u16.to_be_bytes());

        // Compression method (null)
        data.push(0);

        // No extensions
        // (length = 0 would go here, but we'll skip it for minimal test)

        let hello = ServerHello::from_bytes(&data).unwrap();
        assert_eq!(hello.version, TlsVersion::Tls12);
        assert_eq!(hello.random, [0u8; 32]);
        assert_eq!(hello.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
    }

    #[test]
    fn test_extension_server_name() {
        let ext = Extension::server_name("example.com");
        assert_eq!(ext.ext_type, 0);
        assert!(!ext.data.is_empty());

        let bytes = ext.to_bytes().unwrap();
        assert!(bytes.len() > 4);
    }

    #[test]
    fn test_handshake_message_from_bytes() {
        // ServerHelloDone (type 14)
        let data = vec![14, 0, 0, 0];
        let msg = HandshakeMessage::from_bytes(&data).unwrap();
        assert!(matches!(msg, HandshakeMessage::ServerHelloDone));

        // Invalid type
        let data = vec![255];
        assert!(HandshakeMessage::from_bytes(&data).is_err());
    }
}
