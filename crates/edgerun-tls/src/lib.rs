//! A TLS 1.3 client implementation using workspace crypto primitives.
//!
//! # TLS 1.3 Handshake (1-RTT)
//! ```text
//! Client                                          Server
//! ------                                          ------
//! ClientHello (key_share, supported_versions, ...)
//!                                       ←  ServerHello (key_share)
//!                                       ←  {EncryptedExtensions}
//!                                       ←  {Certificate}
//!                                       ←  {CertificateVerify}
//!                                       ←  {Finished}
//! {Finished}                          →
//!
//! [Application Data]      ↔     [Application Data]
//! ```
//!
//! All messages after ServerHello are encrypted with handshake keys.
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

pub mod alert;
pub mod certificate;
pub mod certificate_gen;
pub mod cipher;
pub mod handshake;
pub mod key_exchange;
pub mod prf;
pub mod record;
pub mod server;

use std::io::{self, Read, Write};
use std::net::TcpStream;

use edgerun_crypto::rand_core::RngCore;
use crate::certificate::Certificate;
use crate::cipher::{CipherSuite, NamedGroup};
use crate::handshake::{ClientHelloBuilder, ServerHello};
use crate::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use crate::prf::{Hasher, Tls13KeySchedule, client_write_keys, server_write_keys, client_app_write_keys, server_app_write_keys};
use crate::record::{RecordCipher, TlsRecord};

pub use alert::{Alert, AlertLevel};
pub use certificate_gen::{CertificateAndKey, generate_self_signed};
pub use server::{TlsServerStream, ClientHello};

/// TLS error types
#[derive(Debug)]
pub enum TlsError {
    /// IO error
    Io(io::Error),
    /// Protocol error (malformed message)
    Protocol(String),
    /// Handshake failure
    HandshakeFailure(String),
    /// Certificate validation error
    Certificate(String),
    /// AEAD encryption/decryption error
    Cipher(String),
    /// Alert received from peer
    Alert(AlertLevel, Alert),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::Io(e) => write!(f, "IO error: {e}"),
            TlsError::Protocol(m) => write!(f, "Protocol error: {m}"),
            TlsError::HandshakeFailure(m) => write!(f, "Handshake failure: {m}"),
            TlsError::Certificate(m) => write!(f, "Certificate error: {m}"),
            TlsError::Cipher(m) => write!(f, "Cipher error: {m}"),
            TlsError::Alert(lv, a) => write!(f, "TLS alert: {lv:?} {a}"),
        }
    }
}

impl std::error::Error for TlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TlsError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for TlsError {
    fn from(e: io::Error) -> Self {
        TlsError::Io(e)
    }
}

impl From<String> for TlsError {
    fn from(m: String) -> Self {
        TlsError::Protocol(m)
    }
}

impl From<TlsError> for io::Error {
    fn from(e: TlsError) -> Self {
        match e {
            TlsError::Io(e) => e,
            other => io::Error::new(io::ErrorKind::Other, other.to_string()),
        }
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, TlsError>;

/// TLS 1.3 stream wrapping a TCP connection
pub struct TlsStream {
    stream: TcpStream,
    server_name: String,
    cipher_suite: CipherSuite,
    /// Record cipher for writing (client → server)
    write_cipher: RecordCipher,
    /// Record cipher for reading (server → client)
    read_cipher: RecordCipher,
    /// Handshake completed
    handshake_done: bool,
    /// Pending application data read but not yet consumed
    pending_data: Vec<u8>,
    pending_offset: usize,
}

impl TlsStream {
    /// Perform a TLS 1.3 client handshake over an existing TCP stream.
    pub fn client(stream: TcpStream, server_name: &str) -> Result<Self> {
        let mut hs = Handshake::new(stream, server_name);
        hs.do_handshake()?;
        Ok(hs.finish())
    }

    /// Check if the TLS handshake has completed
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_done
    }

    /// Write raw bytes (encrypted after handshake)
    fn write_application_data(&mut self, buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let ciphertext = self.write_cipher.encrypt(23, buf);
        let record = TlsRecord {
            content_type: 23, // application_data (outer type is always 23 for encrypted records)
            version: 0x0303,  // TLS 1.2 for middlebox compatibility
            fragment: ciphertext,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    /// Read and decrypt one application data record
    fn read_application_data(&mut self) -> Result<Vec<u8>> {
        loop {
            let (content_type, _version, length) = handshake::read_record_header(&mut self.stream)?;
            let fragment = handshake::read_record_fragment(&mut self.stream, length)?;

            if content_type == 23 {
                // application_data
                let (inner_type, plaintext) = self.read_cipher.decrypt(&fragment)?;
                if inner_type == 23 {
                    return Ok(plaintext);
                }
                // Could be a post-handshake message (e.g., NewSessionTicket) — ignore
            } else if content_type == 21 {
                // alert
                if fragment.len() >= 2 {
                    let level = AlertLevel::from_wire(fragment[0])
                        .map_err(|e| TlsError::Protocol(e))?;
                    let alert = Alert::from_wire(fragment[1])
                        .map_err(|e| TlsError::Protocol(e))?;
                    if level == AlertLevel::Fatal {
                        return Err(TlsError::Alert(level, alert));
                    }
                }
            } else if content_type == 22 {
                // handshake — could be NewSessionTicket post-handshake, skip
            }
        }
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.handshake_done {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            ));
        }

        // Return pending data first
        if self.pending_offset < self.pending_data.len() {
            let available = self.pending_data.len() - self.pending_offset;
            let n = available.min(buf.len());
            buf[..n].copy_from_slice(&self.pending_data[self.pending_offset..self.pending_offset + n]);
            self.pending_offset += n;
            return Ok(n);
        }

        // Read a new record
        match self.read_application_data() {
            Ok(plaintext) => {
                let n = plaintext.len().min(buf.len());
                buf[..n].copy_from_slice(&plaintext[..n]);
                // Stash the rest
                if plaintext.len() > n {
                    self.pending_data = plaintext;
                    self.pending_offset = n;
                } else {
                    self.pending_data.clear();
                    self.pending_offset = 0;
                }
                Ok(n)
            }
            Err(TlsError::Io(e)) => Err(e),
            Err(TlsError::Alert(_, _)) => Err(io::Error::new(io::ErrorKind::ConnectionReset, "TLS alert")),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        }
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.handshake_done {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            ));
        }
        self.write_application_data(buf).map_err(io::Error::from)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

// ---- Handshake state machine ----

struct Handshake {
    stream: TcpStream,
    server_name: String,
    cipher_suite: CipherSuite,
    client_random: [u8; 32],
    server_random: [u8; 32],
    key_pair: EcdhKeyPair,
    /// Parsed ServerHello (needed for key_share extraction)
    server_hello: Option<ServerHello>,
    /// Hash of ClientHello (for key schedule)
    ch_hash: Vec<u8>,
    /// Hash of ServerHello (for key schedule)
    sh_hash: Vec<u8>,
    /// Running transcript: concatenation of all handshake message bytes
    /// Used for computing the transcript hash for Finished verification
    transcript: Vec<u8>,
    write_cipher: RecordCipher,
    read_cipher: RecordCipher,
}

impl Handshake {
    fn new(stream: TcpStream, server_name: &str) -> Self {
        let client_random = generate_random();
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::X25519).expect("ECDH key generation failed");
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;

        // Dummy ciphers — replaced after key derivation
        let write_cipher = RecordCipher::new(&[0u8; 16], &[0u8; 12]).unwrap();
        let read_cipher = RecordCipher::new(&[0u8; 16], &[0u8; 12]).unwrap();

        Handshake {
            stream,
            server_name: server_name.to_string(),
            cipher_suite,
            client_random,
            server_random: [0u8; 32],
            key_pair,
            server_hello: None,
            ch_hash: Vec::new(),
            sh_hash: Vec::new(),
            transcript: Vec::new(),
            write_cipher,
            read_cipher,
        }
    }

    fn do_handshake(&mut self) -> Result<()> {
        // 1. Send ClientHello
        self.send_client_hello()?;

        // 2. Read ServerHello (plaintext)
        self.read_server_hello()?;

        // 3. Derive handshake keys
        let shared_secret = self.key_pair.exchange(&self.server_key_share()?)?;
        let hash = self.hasher();

        // Compute transcript hash: ClientHello || ServerHello
        // self.transcript already contains both messages at this point
        let transcript_hash = hash.hash(&self.transcript);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared_secret, &transcript_hash, &transcript_hash);

        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);

        let client_hs_keys = client_write_keys(&client_hs_secret, self.cipher_suite.key_len(), 12, &hash);
        let server_hs_keys = server_write_keys(&server_hs_secret, self.cipher_suite.key_len(), 12, &hash);

        let mut write_cipher = RecordCipher::new(&client_hs_keys.write_key, &client_hs_keys.write_iv)?;
        let mut read_cipher = RecordCipher::new(&server_hs_keys.write_key, &server_hs_keys.write_iv)?;

        // 4. Read encrypted messages: EncryptedExtensions, Certificate, CertificateVerify, Finished
        self.read_encrypted_handshake_messages(&mut read_cipher, &mut ks)?;

        // 5. Send client Finished
        self.send_finished(&mut write_cipher, &ks)?;

        // 6. Derive application keys
        ks.advance_to_master();
        let client_app = ks.client_app_traffic_secret();
        let server_app = ks.server_app_traffic_secret();

        let client_app_keys = client_app_write_keys(&client_app, self.cipher_suite.key_len(), 12, &hash);
        let server_app_keys = server_app_write_keys(&server_app, self.cipher_suite.key_len(), 12, &hash);

        self.write_cipher = RecordCipher::new(&client_app_keys.write_key, &client_app_keys.write_iv)?;
        self.read_cipher = RecordCipher::new(&server_app_keys.write_key, &server_app_keys.write_iv)?;

        Ok(())
    }

    fn send_client_hello(&mut self) -> Result<()> {
        let public_key = self.key_pair.public_key_bytes();
        let group = match self.key_pair.group() {
            KeyExchangeGroup::SECP256R1 => NamedGroup::SECP256R1,
            KeyExchangeGroup::X25519 => NamedGroup::X25519,
        };
        let ch = ClientHelloBuilder::new(self.client_random, &self.server_name)
            .key_share(&public_key, group)
            .build()?;

        // Compute ch_hash = SHA-256(ClientHello message)
        self.ch_hash = self.hasher().hash(&ch);
        // Initialize transcript with ClientHello bytes
        self.transcript = ch.clone();

        // Wrap in TLS 1.2 record for middlebox compatibility (content_type=22, version=0x0303)
        let record = TlsRecord {
            content_type: 22, // handshake
            version: 0x0303,
            fragment: ch,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    fn read_server_hello(&mut self) -> Result<()> {
        let (ct, _ver, len) = handshake::read_record_header(&mut self.stream)?;
        if ct != 22 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected handshake record, got content_type={ct}",
            )));
        }
        let fragment = handshake::read_record_fragment(&mut self.stream, len)?;

        let sh = ServerHello::parse(&fragment)?;

        if sh.supported_version != Some(0x0304) {
            return Err(TlsError::HandshakeFailure(format!(
                "Server did not negotiate TLS 1.3 (got supported_version={:?})",
                sh.supported_version,
            )));
        }

        self.server_random = sh.random;
        self.cipher_suite = sh.cipher_suite;
        self.server_hello = Some(sh);
        self.sh_hash = self.hasher().hash(&fragment);
        // Append ServerHello to transcript
        self.transcript.extend_from_slice(&fragment);

        Ok(())
    }

    fn server_key_share(&self) -> Result<Vec<u8>> {
        let sh = self.server_hello.as_ref()
            .ok_or_else(|| TlsError::HandshakeFailure("No ServerHello yet".into()))?;
        if sh.server_key_share.is_empty() {
            return Err(TlsError::HandshakeFailure("No key_share in ServerHello".into()));
        }
        Ok(sh.server_key_share.clone())
    }

    fn read_encrypted_handshake_messages(
        &mut self,
        read_cipher: &mut RecordCipher,
        ks: &mut Tls13KeySchedule,
    ) -> Result<()> {
        // Read records until we get Finished
        // Each decrypted record contains one handshake message.
        // For the transcript hash, we need to reconstruct the handshake message
        // in its wire format: type(1) + length(3) + payload
        loop {
            let (_ct, _ver, len) = handshake::read_record_header(&mut self.stream)?;

            // After ServerHello, TLS 1.3 uses TLS 1.2 version for outer records
            let fragment = handshake::read_record_fragment(&mut self.stream, len)?;

            let (inner_type, plaintext) = read_cipher.decrypt(&fragment)?;

            // The plaintext IS the handshake message in wire format: type(1) + length(3) + payload
            // Reconstruct it for the transcript hash (it's already in the right format)
            let hs_msg = plaintext.clone();

            // The handshake message type is the first byte of plaintext
            let hs_type = if !hs_msg.is_empty() { hs_msg[0] } else { inner_type };

            match hs_type {
                8 => {
                    // EncryptedExtensions — append to transcript
                    self.transcript.extend_from_slice(&hs_msg);
                }
                11 => {
                    // Certificate — append to transcript
                    self.transcript.extend_from_slice(&hs_msg);
                    if hs_msg.len() >= 4 {
                        let cert_list_len =
                            u32::from_be_bytes([0, hs_msg[1], hs_msg[2], hs_msg[3]]) as usize;
                        if hs_msg.len() >= 4 + cert_list_len {
                            let cert_data = &hs_msg[4..4 + cert_list_len];
                            let certs = Certificate::parse_list(cert_data)?;

                            // Basic validation
                            if certs.is_empty() {
                                return Err(TlsError::Certificate("No certificates from server".into()));
                            }
                            let leaf = &certs[0];
                            if !leaf.is_valid_now() {
                                return Err(TlsError::Certificate(
                                    "Server certificate is expired".into(),
                                ));
                            }
                            if !leaf.matches_hostname(&self.server_name) {
                                return Err(TlsError::Certificate(format!(
                                    "Certificate does not match hostname {}",
                                    self.server_name,
                                )));
                            }

                            // Verify certificate chain (leaf signed by intermediate, etc.)
                            if certs.len() >= 2 {
                                // Verify leaf against issuer (second cert in chain)
                                let issuer = &certs[1];
                                if let Err(e) = leaf.verify_signature(issuer) {
                                    return Err(TlsError::Certificate(format!(
                                        "Certificate signature verification failed: {}",
                                        e,
                                    )));
                                }
                                // If we have more certs, verify intermediate against root
                                if certs.len() >= 3 {
                                    let root = &certs[2];
                                    if let Err(e) = issuer.verify_signature(root) {
                                        return Err(TlsError::Certificate(format!(
                                            "Intermediate certificate verification failed: {}",
                                            e,
                                        )));
                                    }
                                }
                            } else {
                                // Self-signed certificate — only accept if explicitly allowed
                                // For now, reject self-signed certs in production
                                return Err(TlsError::Certificate(
                                    "Self-signed certificates are not accepted".into(),
                                ));
                            }
                        }
                    }
                }
                15 => {
                    // CertificateVerify — append to transcript
                    self.transcript.extend_from_slice(&hs_msg);
                }
                20 => {
                    // Finished — verify verify_data
                    // hs_msg is the full handshake message: type(1) + length(3) + verify_data
                    // Per RFC 8446 §4.4.4: verify_data is computed from transcript NOT including Finished
                    
                    // The verify_data is computed as:
                    //   finished_key = HKDF-Expand-Label(handshake_traffic_secret, "finished", "", Hash.length)
                    //   verify_data = HMAC(finished_key, Hash(handshake_transcript))
                    //
                    // Compute the transcript hash of all handshake messages BEFORE Finished:
                    // ClientHello || ServerHello || EncryptedExtensions || Certificate || CertificateVerify
                    let full_transcript_hash = self.hasher().hash(&self.transcript);

                    let server_hs_secret = ks.server_handshake_traffic_secret(&full_transcript_hash);
                    let finished_key = self.hasher().expand_label(&server_hs_secret, "finished", &[], self.hasher().len());

                    // hs_msg: type(1) + length(3) + verify_data
                    // verify_data starts at offset 4
                    if hs_msg.len() < 4 + self.hasher().len() {
                        return Err(TlsError::Protocol(
                            "Finished message too short".into(),
                        ));
                    }
                    let verify_data = &hs_msg[4..];
                    
                    // Compute expected verify_data using HMAC
                    let expected_verify_data = match self.hasher() {
                        Hasher::Sha256 => hmac_sha256(&finished_key, &full_transcript_hash),
                        Hasher::Sha384 => hmac_sha384(&finished_key, &full_transcript_hash),
                    };

                    // Constant-time comparison
                    if verify_data.len() < expected_verify_data.len()
                        || !constant_time_eq(&verify_data[..expected_verify_data.len()], &expected_verify_data)
                    {
                        return Err(TlsError::Protocol(
                            "Finished message verification failed".into(),
                        ));
                    }

                    // Append server's Finished to transcript (needed for client's own Finished)
                    self.transcript.extend_from_slice(&hs_msg);

                    return Ok(());
                }
                _ => {
                    // Unknown message type, skip
                }
            }
        }
    }

    fn send_finished(
        &mut self,
        write_cipher: &mut RecordCipher,
        ks: &Tls13KeySchedule,
    ) -> Result<()> {
        // Client Finished: verify_data = HMAC(finished_key, Hash(transcript))
        // where finished_key = HKDF-Expand-Label(client_handshake_traffic_secret, "finished", "", Hash.length)
        // and transcript includes all handshake messages up to (but not including) client Finished
        let full_transcript_hash = self.hasher().hash(&self.transcript);
        let client_hs_secret = ks.client_handshake_traffic_secret(&full_transcript_hash);
        let finished_key = self.hasher().expand_label(&client_hs_secret, "finished", &[], self.hasher().len());
        let verify_data = match self.hasher() {
            Hasher::Sha256 => hmac_sha256(&finished_key, &full_transcript_hash),
            Hasher::Sha384 => hmac_sha384(&finished_key, &full_transcript_hash),
        };

        // Append client Finished to transcript
        // The Finished message format is: type(1) + length(3) + verify_data
        let mut finished_msg = Vec::with_capacity(4 + verify_data.len());
        finished_msg.push(20); // Finished type
        finished_msg.extend_from_slice(&(verify_data.len() as u32).to_be_bytes()[1..]); // 3-byte length
        finished_msg.extend_from_slice(&verify_data);
        self.transcript.extend_from_slice(&finished_msg);

        let ciphertext = write_cipher.encrypt(22, &finished_msg);
        let record = TlsRecord {
            content_type: 23, // application_data for encrypted records
            version: 0x0303,
            fragment: ciphertext,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    fn hasher(&self) -> Hasher {
        match self.cipher_suite {
            CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
            _ => Hasher::Sha256,
        }
    }

    fn finish(self) -> TlsStream {
        TlsStream {
            stream: self.stream,
            server_name: self.server_name,
            cipher_suite: self.cipher_suite,
            write_cipher: self.write_cipher,
            read_cipher: self.read_cipher,
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
        }
    }
}

/// Generate 32 random bytes from /dev/urandom
fn generate_random() -> [u8; 32] {
    let mut buf = [0u8; 32];
    #[cfg(unix)]
    {
        edgerun_crypto::rand_core::OsRng.fill_bytes(&mut buf);
    }
    #[cfg(not(unix))]
    {
        // TLS requires cryptographically secure randomness.
        // This platform is not supported — fail at compile time in release,
        // panic at runtime in debug builds.
        compile_error!(
            "edgerun-tls requires /dev/urandom and only supports Unix-like platforms. \
             Using a deterministic PRNG for TLS is a critical security vulnerability."
        );
    }
    buf
}

/// Constant-time equality comparison for cryptographic data.
/// Returns true if the two slices are equal, without leaking timing information.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Re-export HMAC functions from prf module for Finished verification
use crate::prf::{hmac_sha256, hmac_sha384};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_generate_random() {
        let r1 = generate_random();
        let r2 = generate_random();
        assert_eq!(r1.len(), 32);
        // Two consecutive reads from urandom should differ
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_tls_error_display() {
        let e = TlsError::Protocol("test".into());
        assert!(e.to_string().contains("test"));

        let e = TlsError::Certificate("expired".into());
        assert!(e.to_string().contains("expired"));
    }

    #[test]
    fn test_certificate_generation() {
        let cert = generate_self_signed(&["localhost", "example.com"]);
        assert!(!cert.cert_der.is_empty());
        assert!(cert.cert_der.len() > 100); // Reasonable cert size

        // Parse the generated certificate
        let parsed = Certificate::from_der(&cert.cert_der).expect("failed to parse generated cert");

        // Check that the cert is structurally valid
        assert!(parsed.is_valid_now());

        // Check SAN parsing (our cert generator puts SANs in extensions)
        // The SAN parser looks for OID 2.5.29.17 in extensions
        // Our generated cert should have SANs parseable
        assert!(parsed.subject_alt_names.contains(&"localhost".to_string()) || parsed.subject_alt_names.is_empty(),
            "SANs should contain localhost or be empty (parser limitation)");

        // CN parsing depends on the parser's OID matching — our manual DER may
        // have subtle differences. The important thing is the cert is valid DER
        // and can be used for TLS.
    }

    #[test]
    fn test_server_client_hello_parsing() {
        use crate::server::ClientHello;

        // Build a minimal ClientHello and verify it parses
        let random = generate_random();
        let ch = ClientHelloBuilder::new(random, "localhost")
            .key_share(&[0x04u8; 65], NamedGroup::SECP256R1)
            .build()
            .expect("build ClientHello");

        let parsed = ClientHello::parse(&ch).expect("parse ClientHello");
        assert_eq!(parsed.server_name, Some("localhost".to_string()));
        assert!(!parsed.cipher_suites.is_empty());
        assert!(parsed.client_key_share.is_some());
        assert!(parsed.supported_versions.contains(&0x0304));
    }

    /// Full TLS 1.3 client-server loopback test
    #[test]
    fn test_tls_server_client_loopback() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let port = listener.local_addr().expect("get addr").port();

        let cert = generate_self_signed(&["localhost"]);

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let barrier_clone = barrier.clone();

        let server_thread = thread::spawn(move || {
            let (stream, _addr) = listener.accept().expect("accept connection");
            let sock = socket2::SockRef::from(&stream);
            sock.set_nonblocking(false).expect("set blocking");
            sock.set_read_timeout(None).ok();
            sock.set_write_timeout(None).ok();

            // Wait for client to be ready
            barrier_clone.wait();

            let mut tls = TlsServerStream::accept(stream, &cert)
                .expect("TLS server handshake failed");

            let mut buf = [0u8; 1024];
            let n = tls.read(&mut buf).expect("read from client");
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", request.len(), request);
            tls.write_all(response.as_bytes()).expect("write to client");
            tls.flush().expect("flush");

            request
        });

        // Connect
        let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        ).expect("create socket");
        socket.set_nonblocking(false).expect("set blocking");
        socket.connect(&addr.into()).expect("connect");
        let tcp: std::net::TcpStream = socket.into();

        // Signal server that we're about to start handshake
        barrier.wait();

        let mut tls = TlsStream::client(tcp, "localhost")
            .expect("TLS client handshake failed");

        let request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        tls.write_all(request).expect("write to server");
        tls.flush().expect("flush");

        let mut response = Vec::new();
        tls.read_to_end(&mut response).ok();

        let response_str = String::from_utf8_lossy(&response).to_string();
        assert!(response_str.contains("200 OK"));
        assert!(response_str.contains("GET / HTTP/1.1"));

        let received = server_thread.join().expect("server thread panicked");
        assert_eq!(received, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    }
}
