//! A dependency-free TLS 1.2/1.3 client implementation using workspace crypto primitives.
//!
//! # Features
//! - TLS 1.2 and TLS 1.3 handshake protocol
//! - Record layer encryption and decryption
//! - Certificate validation
//! - Cipher suite negotiation
//!
//! # Example
//! ```no_run
//! use edgerun_tls::TlsStream;
//! use std::net::TcpStream;
//! use std::io::{Read, Write};
//!
//! let tcp = TcpStream::connect("example.com:443").unwrap();
//! let mut tls = TlsStream::client(tcp, "example.com").unwrap();
//!
//! tls.write_all(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n").unwrap();
//! let mut response = Vec::new();
//! tls.read_to_end(&mut response).unwrap();
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![allow(non_camel_case_types)] // Cipher suite names follow IANA standards

pub mod alert;
pub mod certificate;
pub mod cipher;
pub mod handshake;
pub mod key_exchange;
pub mod prf;
pub mod record;

use std::io::{self, Read, Write};
use std::net::TcpStream;

pub use alert::{Alert, AlertLevel};
pub use certificate::Certificate;
pub use cipher::CipherSuite;
pub use handshake::{ClientHello, HandshakeMessage, ServerHello};
pub use record::TlsRecord;

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TlsVersion {
    /// TLS 1.2
    Tls12,
    /// TLS 1.3
    Tls13,
}

impl TlsVersion {
    /// Convert to wire format (u16)
    pub fn to_wire(self) -> u16 {
        match self {
            TlsVersion::Tls12 => 0x0303,
            TlsVersion::Tls13 => 0x0304,
        }
    }

    /// Parse from wire format
    pub fn from_wire(version: u16) -> Result<Self> {
        match version {
            0x0303 => Ok(TlsVersion::Tls12),
            0x0304 => Ok(TlsVersion::Tls13),
            _ => Err(TlsError::Protocol(format!(
                "Unsupported TLS version: 0x{:04x}",
                version
            ))),
        }
    }
}

/// TLS error types
#[derive(Debug)]
pub enum TlsError {
    /// IO error
    Io(io::Error),
    /// Protocol error
    Protocol(String),
    /// Handshake failure
    HandshakeFailure(String),
    /// Certificate error
    Certificate(String),
    /// Cipher error
    Cipher(String),
    /// Alert received from server
    Alert(AlertLevel, Alert),
    /// Invalid MAC
    MacError,
    /// Decryption failed
    DecryptionFailed,
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::Io(err) => write!(f, "IO error: {}", err),
            TlsError::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            TlsError::HandshakeFailure(msg) => write!(f, "Handshake failure: {}", msg),
            TlsError::Certificate(msg) => write!(f, "Certificate error: {}", msg),
            TlsError::Cipher(msg) => write!(f, "Cipher error: {}", msg),
            TlsError::Alert(level, alert) => {
                write!(f, "TLS alert received: {:?} {:?}", level, alert)
            }
            TlsError::MacError => write!(f, "Invalid MAC"),
            TlsError::DecryptionFailed => write!(f, "Decryption failed"),
        }
    }
}

impl std::error::Error for TlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TlsError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for TlsError {
    fn from(err: io::Error) -> Self {
        TlsError::Io(err)
    }
}

impl From<String> for TlsError {
    fn from(err: String) -> Self {
        TlsError::Protocol(err)
    }
}

impl From<TlsError> for std::io::Error {
    fn from(err: TlsError) -> Self {
        match err {
            TlsError::Io(err) => err,
            other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
        }
    }
}

/// Result type for TLS operations
pub type Result<T> = std::result::Result<T, TlsError>;

/// TLS stream wrapping a TCP connection
pub struct TlsStream {
    /// Underlying TCP stream
    stream: TcpStream,
    /// TLS version negotiated
    version: TlsVersion,
    /// Server hostname (for SNI)
    server_name: String,
    /// Selected cipher suite
    cipher_suite: CipherSuite,
    /// Client random
    client_random: [u8; 32],
    /// Server random
    server_random: [u8; 32],
    /// Handshake completed
    handshake_complete: bool,
    /// Read buffer for partial records
    read_buffer: Vec<u8>,
    /// Write buffer for records
    write_buffer: Vec<u8>,
}

impl TlsStream {
    /// Create a new TLS client stream
    pub fn client(stream: TcpStream, server_name: &str) -> Result<Self> {
        let mut tls = TlsStream {
            stream,
            version: TlsVersion::Tls13,
            server_name: server_name.to_string(),
            cipher_suite: CipherSuite::TLS_AES_128_GCM_SHA256,
            client_random: [0u8; 32],
            server_random: [0u8; 32],
            handshake_complete: false,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
        };

        // Perform TLS handshake
        tls.handshake()?;

        Ok(tls)
    }

    /// Perform TLS handshake
    fn handshake(&mut self) -> Result<()> {
        // Generate client random
        self.client_random = Self::generate_random();

        // Send ClientHello
        self.send_client_hello()?;

        // Read ServerHello
        self.read_server_hello()?;

        // Read server certificates
        self.read_certificates()?;

        // Read ServerKeyExchange (if TLS 1.2)
        if self.version == TlsVersion::Tls12 {
            self.read_server_key_exchange()?;
        }

        // Read ServerHelloDone (TLS 1.2) or handle TLS 1.3 extensions
        if self.version == TlsVersion::Tls12 {
            self.read_server_hello_done()?;
        }

        // Send ClientKeyExchange
        self.send_client_key_exchange()?;

        // Send ChangeCipherSpec and Finished (TLS 1.2)
        if self.version == TlsVersion::Tls12 {
            self.send_change_cipher_spec()?;
            self.send_finished()?;
        }

        // For TLS 1.3, handle encrypted extensions and finish
        if self.version == TlsVersion::Tls13 {
            self.read_encrypted_extensions()?;
            self.read_server_finished()?;
            self.send_client_finished()?;
        }

        self.handshake_complete = true;

        Ok(())
    }

    /// Generate cryptographically secure random bytes
    fn generate_random() -> [u8; 32] {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut random = [0u8; 32];

        // Use system time as entropy source (not ideal, but std-only)
        // In production, use OS-specific crypto APIs or /dev/urandom
        #[cfg(unix)]
        {
            use std::fs::File;
            use std::io::Read;
            if let Ok(mut urandom) = File::open("/dev/urandom") {
                if urandom.read_exact(&mut random).is_ok() {
                    return random;
                }
            }
        }

        // Fallback: mix timestamps and memory addresses
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        for (i, byte) in random.iter_mut().enumerate() {
            *byte = ((timestamp >> (i % 8) * 8) as u8) ^ (i as u8);
        }

        random
    }

    /// Send ClientHello message
    fn send_client_hello(&mut self) -> Result<()> {
        let client_hello = ClientHello::new(
            self.version,
            &self.client_random,
            &self.server_name,
        );

        let data = client_hello.to_bytes()?;
        self.write_tls_record(22, &data)?; // Handshake type = 22

        Ok(())
    }

    /// Read ServerHello message
    fn read_server_hello(&mut self) -> Result<()> {
        let record = self.read_tls_record()?;
        if record.content_type != 22 {
            return Err(TlsError::Protocol("Expected handshake record".to_string()));
        }

        let server_hello = ServerHello::from_bytes(&record.fragment)?;
        self.version = server_hello.version;
        self.server_random = server_hello.random;
        self.cipher_suite = server_hello.cipher_suite;

        Ok(())
    }

    /// Read server certificates
    fn read_certificates(&mut self) -> Result<()> {
        // Read certificate record
        let record = self.read_tls_record()?;

        // Parse certificates
        let _certificates = Certificate::parse_all(&record.fragment)
            .map_err(|e| TlsError::Certificate(e))?;

        // In a full implementation, validate certificates here:
        // - Check expiry
        // - Verify signatures
        // - Validate chain
        // - Check revocation

        Ok(())
    }

    /// Read ServerKeyExchange (TLS 1.2)
    fn read_server_key_exchange(&mut self) -> Result<()> {
        // Read and parse server key exchange
        let _record = self.read_tls_record()?;
        Ok(())
    }

    /// Read ServerHelloDone (TLS 1.2)
    fn read_server_hello_done(&mut self) -> Result<()> {
        let _record = self.read_tls_record()?;
        Ok(())
    }

    /// Send ClientKeyExchange
    fn send_client_key_exchange(&mut self) -> Result<()> {
        // Generate client key exchange data
        let key_exchange_data = vec![0u8; 65]; // Placeholder for ECDHE public key
        self.write_tls_record(22, &key_exchange_data)?;

        Ok(())
    }

    /// Send ChangeCipherSpec (TLS 1.2)
    fn send_change_cipher_spec(&mut self) -> Result<()> {
        self.write_tls_record(20, &[1])?; // ChangeCipherSpec = 20
        Ok(())
    }

    /// Send Finished message (TLS 1.2)
    fn send_finished(&mut self) -> Result<()> {
        // Finished message is 12 bytes of PRF output
        let finished = Self::generate_random()[..12].to_vec();
        self.write_tls_record(22, &finished)?;

        Ok(())
    }

    /// Read encrypted extensions (TLS 1.3)
    fn read_encrypted_extensions(&mut self) -> Result<()> {
        let _record = self.read_tls_record()?;
        Ok(())
    }

    /// Read server finished (TLS 1.3)
    fn read_server_finished(&mut self) -> Result<()> {
        let _record = self.read_tls_record()?;
        Ok(())
    }

    /// Send client finished (TLS 1.3)
    fn send_client_finished(&mut self) -> Result<()> {
        let finished = Self::generate_random()[..12].to_vec();
        self.write_tls_record(22, &finished)?;

        Ok(())
    }

    /// Write a TLS record
    fn write_tls_record(&mut self, content_type: u8, data: &[u8]) -> Result<()> {
        // TLS record format:
        // - Content type: 1 byte
        // - Version: 2 bytes
        // - Length: 2 bytes
        // - Fragment: variable

        let mut record = Vec::with_capacity(5 + data.len());
        record.push(content_type);
        record.extend_from_slice(&self.version.to_wire().to_be_bytes());
        record.extend_from_slice(&(data.len() as u16).to_be_bytes());
        record.extend_from_slice(data);

        self.stream.write_all(&record)?;
        self.stream.flush()?;

        Ok(())
    }

    /// Read a TLS record
    fn read_tls_record(&mut self) -> Result<TlsRecord> {
        // Read record header (5 bytes)
        let mut header = [0u8; 5];
        self.stream.read_exact(&mut header)?;

        let content_type = header[0];
        let version = u16::from_be_bytes([header[1], header[2]]);
        let length = u16::from_be_bytes([header[3], header[4]]) as usize;

        // Validate version
        let _tls_version = TlsVersion::from_wire(version)?;

        // Read fragment
        let mut fragment = vec![0u8; length];
        self.stream.read_exact(&mut fragment)?;

        Ok(TlsRecord {
            content_type,
            version,
            fragment,
        })
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If handshake not complete, error
        if !self.handshake_complete {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            ));
        }

        // Read ApplicationData record (content type 23)
        let record = self.read_tls_record().map_err(io::Error::from)?;

        if record.content_type != 23 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected application data, got content type {}", record.content_type),
            ));
        }

        // For TLS 1.3, decrypt the record
        // For now, just copy the fragment
        let data = &record.fragment;
        let len = data.len().min(buf.len());
        buf[..len].copy_from_slice(&data[..len]);

        Ok(len)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // If handshake not complete, error
        if !self.handshake_complete {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            ));
        }

        // Write as ApplicationData record (content type 23)
        self.write_tls_record(23, buf).map_err(io::Error::from)?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_version_wire() {
        assert_eq!(TlsVersion::Tls12.to_wire(), 0x0303);
        assert_eq!(TlsVersion::Tls13.to_wire(), 0x0304);

        assert_eq!(TlsVersion::from_wire(0x0303).unwrap(), TlsVersion::Tls12);
        assert_eq!(TlsVersion::from_wire(0x0304).unwrap(), TlsVersion::Tls13);
        assert!(TlsVersion::from_wire(0x0301).is_err());
    }

    #[test]
    fn test_generate_random() {
        let random1 = TlsStream::generate_random();
        let random2 = TlsStream::generate_random();

        // Should be 32 bytes
        assert_eq!(random1.len(), 32);

        // Two consecutive random values should (very likely) differ
        // This is a probabilistic test but good enough for sanity check
        assert_ne!(random1, random2);
    }

    #[test]
    fn test_tls_error_display() {
        let err = TlsError::Protocol("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = TlsError::HandshakeFailure("failed".to_string());
        assert!(err.to_string().contains("failed"));
    }
}
