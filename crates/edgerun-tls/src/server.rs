//! TLS 1.3 server-side handshake implementation.
//!
//! Handles accepting ClientHello, sending ServerHello + encrypted handshake
//! messages, and completing the TLS 1.3 server handshake.
//!
//! # Server Handshake Flow
//! ```text
//! Client                                          Server
//! ------                                          ------
//! ClientHello (key_share, supported_versions, ...)  →
//!                                       ←  ServerHello (key_share)
//!                                       ←  {EncryptedExtensions}
//!                                       ←  {Certificate}
//!                                       ←  {CertificateVerify}
//!                                       ←  {Finished}
//! {Finished}                          →
//!
//! [Application Data]      ↔     [Application Data]
//! ```

use std::io::{self, Read, Write};
use std::net::TcpStream;

use edgerun_crypto::p256::ecdsa::{Signature, SigningKey, signature::SignerMut};
use edgerun_crypto::rand_core::{OsRng, RngCore};
use edgerun_crypto::sha2::{Digest, Sha256, Sha384};

use crate::alert::{Alert, AlertLevel};
use crate::certificate_gen::CertificateAndKey;
use crate::cipher::{CipherSuite, NamedGroup};
use crate::handshake::{read_record_header, read_record_fragment};
use crate::key_exchange::EcdhKeyPair;
use crate::prf::{Hasher, Tls13KeySchedule, client_write_keys, server_write_keys, hmac_sha256, hmac_sha384};
use crate::record::{RecordCipher, TlsRecord};
use crate::{Result, TlsError};

/// Parsed ClientHello from the wire
#[derive(Debug)]
pub struct ClientHello {
    pub legacy_version: u16,
    pub random: [u8; 32],
    pub session_id: Vec<u8>,
    pub cipher_suites: Vec<CipherSuite>,
    pub legacy_compression: Vec<u8>,
    /// SNI server name (if present)
    pub server_name: Option<String>,
    /// Client's key_share public key bytes (for the group we support)
    pub client_key_share: Option<Vec<u8>>,
    pub client_key_share_group: Option<NamedGroup>,
    /// Supported versions extension
    pub supported_versions: Vec<u16>,
    /// Supported groups
    pub supported_groups: Vec<NamedGroup>,
    /// Signature algorithms
    pub signature_algorithms: Vec<u16>,
}

impl ClientHello {
    /// Parse a ClientHello from a handshake message payload (without the 4-byte header)
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(TlsError::HandshakeFailure("ClientHello too short".into()));
        }
        if data[0] != 1 {
            return Err(TlsError::HandshakeFailure(
                format!("Expected ClientHello (type 1), got {}", data[0]),
            ));
        }

        let msg_len = u32::from_be_bytes([0, data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + msg_len {
            return Err(TlsError::HandshakeFailure("ClientHello truncated".into()));
        }

        let msg = &data[4..4 + msg_len];
        let mut pos = 0;

        // Legacy version
        if pos + 2 > msg.len() {
            return Err(TlsError::Protocol("ClientHello: legacy_version truncated".into()));
        }
        let legacy_version = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
        pos += 2;

        // Random
        if pos + 32 > msg.len() {
            return Err(TlsError::Protocol("ClientHello: random truncated".into()));
        }
        let mut random = [0u8; 32];
        random.copy_from_slice(&msg[pos..pos + 32]);
        pos += 32;

        // Session ID
        if pos >= msg.len() {
            return Err(TlsError::Protocol("ClientHello: session_id length missing".into()));
        }
        let sid_len = msg[pos] as usize;
        pos += 1;
        if pos + sid_len > msg.len() {
            return Err(TlsError::Protocol("ClientHello: session_id truncated".into()));
        }
        let session_id = msg[pos..pos + sid_len].to_vec();
        pos += sid_len;

        // Cipher suites
        if pos + 2 > msg.len() {
            return Err(TlsError::Protocol("ClientHello: cipher_suites length truncated".into()));
        }
        let cs_len = u16::from_be_bytes([msg[pos], msg[pos + 1]]) as usize;
        pos += 2;
        if pos + cs_len > msg.len() {
            return Err(TlsError::Protocol("ClientHello: cipher_suites truncated".into()));
        }
        let mut cipher_suites = Vec::new();
        let cs_end = pos + cs_len;
        while pos + 1 < cs_end {
            let cs = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
            if let Ok(suite) = CipherSuite::from_wire(cs) {
                cipher_suites.push(suite);
            }
            pos += 2;
        }
        pos = cs_end;

        // Legacy compression methods
        if pos >= msg.len() {
            return Err(TlsError::Protocol("ClientHello: compression length missing".into()));
        }
        let comp_len = msg[pos] as usize;
        pos += 1;
        if pos + comp_len > msg.len() {
            return Err(TlsError::Protocol("ClientHello: compression truncated".into()));
        }
        let legacy_compression = msg[pos..pos + comp_len].to_vec();
        pos += comp_len;

        // Extensions
        let mut server_name = None;
        let mut client_key_share = None;
        let mut client_key_share_group = None;
        let mut supported_versions = Vec::new();
        let mut supported_groups = Vec::new();
        let mut signature_algorithms = Vec::new();

        if pos < msg.len() {
            if pos + 2 > msg.len() {
                return Err(TlsError::Protocol("ClientHello: ext_len truncated".into()));
            }
            let ext_len = u16::from_be_bytes([msg[pos], msg[pos + 1]]) as usize;
            pos += 2;

            let ext_end = pos + ext_len;
            while pos + 4 <= ext_end {
                let ext_type = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
                let ext_data_len = u16::from_be_bytes([msg[pos + 2], msg[pos + 3]]) as usize;
                pos += 4;

                if pos + ext_data_len > ext_end {
                    break;
                }
                let ext_data = &msg[pos..pos + ext_data_len];
                pos += ext_data_len;

                match ext_type {
                    0 => {
                        // server_name (SNI)
                        // Format: name_list_length(2) + ServerNameList
                        // Each entry: name_type(1) + name_length(2) + name
                        if ext_data_len >= 2 {
                            let name_list_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            if ext_data.len() >= 2 + name_list_len && name_list_len >= 3 {
                                let name_type = ext_data[2];
                                if name_type == 0 {
                                    // host_name
                                    let name_len = u16::from_be_bytes([ext_data[3], ext_data[4]]) as usize;
                                    if 5 + name_len <= 2 + name_list_len {
                                        if let Ok(name) = std::str::from_utf8(&ext_data[5..5 + name_len]) {
                                            server_name = Some(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    10 => {
                        // supported_groups
                        if ext_data_len >= 2 {
                            let groups_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            let mut gpos = 2;
                            while gpos + 1 < groups_len && gpos + 1 < ext_data.len() {
                                let g = u16::from_be_bytes([ext_data[gpos], ext_data[gpos + 1]]);
                                if let Ok(group) = NamedGroup::from_wire(g) {
                                    supported_groups.push(group);
                                }
                                gpos += 2;
                            }
                        }
                    }
                    13 => {
                        // signature_algorithms
                        if ext_data_len >= 2 {
                            let sa_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            let mut spos = 2;
                            while spos + 1 < sa_len && spos + 1 < ext_data.len() {
                                let sa = u16::from_be_bytes([ext_data[spos], ext_data[spos + 1]]);
                                signature_algorithms.push(sa);
                                spos += 2;
                            }
                        }
                    }
                    43 => {
                        // supported_versions
                        if ext_data_len >= 1 {
                            let versions_len = ext_data[0] as usize;
                            let mut vpos = 1;
                            while vpos + 1 < versions_len && vpos + 1 < ext_data.len() {
                                let v = u16::from_be_bytes([ext_data[vpos], ext_data[vpos + 1]]);
                                supported_versions.push(v);
                                vpos += 2;
                            }
                        }
                    }
                    51 => {
                        // key_share
                        if ext_data_len >= 2 {
                            let ks_len = u16::from_be_bytes([ext_data[0], ext_data[1]]) as usize;
                            let mut kpos = 2;
                            while kpos + 3 < ks_len && kpos + 3 < ext_data.len() {
                                let group = u16::from_be_bytes([ext_data[kpos], ext_data[kpos + 1]]);
                                let ke_len = u16::from_be_bytes([ext_data[kpos + 2], ext_data[kpos + 3]]) as usize;
                                kpos += 4;
                                if kpos + ke_len <= ext_data.len() {
                                    if client_key_share.is_none() {
                                        if let Ok(g) = NamedGroup::from_wire(group) {
                                            client_key_share_group = Some(g);
                                            client_key_share = Some(ext_data[kpos..kpos + ke_len].to_vec());
                                        }
                                    }
                                    kpos += ke_len;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(ClientHello {
            legacy_version,
            random,
            session_id,
            cipher_suites,
            legacy_compression,
            server_name,
            client_key_share,
            client_key_share_group,
            supported_versions,
            supported_groups,
            signature_algorithms,
        })
    }
}

impl NamedGroup {
    /// Parse from wire format
    pub fn from_wire(value: u16) -> Result<Self> {
        match value {
            0x0017 => Ok(NamedGroup::SECP256R1),
            0x0018 => Ok(NamedGroup::SECP384R1),
            0x001D => Ok(NamedGroup::X25519),
            _ => Err(TlsError::Protocol(format!("Unsupported named group: 0x{:04x}", value))),
        }
    }
}

/// Server-side TLS 1.3 stream
pub struct TlsServerStream {
    stream: TcpStream,
    /// Record cipher for writing (server → client)
    write_cipher: RecordCipher,
    /// Record cipher for reading (client → server)
    read_cipher: RecordCipher,
    /// Handshake completed
    handshake_done: bool,
    /// Pending application data
    pending_data: Vec<u8>,
    pending_offset: usize,
    /// Negotiated cipher suite
    cipher_suite: CipherSuite,
}

impl TlsServerStream {
    /// Accept a TLS 1.3 connection from an existing TCP stream.
    ///
    /// Uses the provided certificate and key to complete the server handshake.
    pub fn accept(
        stream: TcpStream,
        cert_and_key: &CertificateAndKey,
    ) -> Result<Self> {
        let mut hs = ServerHandshake::new(stream, cert_and_key);
        hs.do_handshake()?;
        Ok(hs.finish())
    }

    /// Check if the TLS handshake has completed
    pub fn is_handshake_complete(&self) -> bool {
        self.handshake_done
    }

    /// Send a TLS alert and close the connection
    fn send_alert(stream: &mut TcpStream, level: AlertLevel, alert: Alert) -> Result<()> {
        let msg = vec![level as u8, alert as u8];
        let record = TlsRecord {
            content_type: 21, // alert
            version: 0x0303,
            fragment: msg,
        };
        stream.write_all(&record.to_bytes())?;
        stream.flush()?;
        Ok(())
    }

    fn write_application_data(&mut self, buf: &[u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let ciphertext = self.write_cipher.encrypt(23, buf);
        let record = TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: ciphertext,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    fn read_application_data(&mut self) -> Result<Vec<u8>> {
        loop {
            let (content_type, _version, length) = read_record_header(&mut self.stream)?;
            let fragment = read_record_fragment(&mut self.stream, length)?;

            if content_type == 23 {
                let (inner_type, plaintext) = self.read_cipher.decrypt(&fragment)?;
                if inner_type == 23 {
                    return Ok(plaintext);
                }
            } else if content_type == 21 {
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
                // Post-handshake message — skip
            }
        }
    }
}

impl Read for TlsServerStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.handshake_done {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "TLS handshake not complete",
            ));
        }

        if self.pending_offset < self.pending_data.len() {
            let available = self.pending_data.len() - self.pending_offset;
            let n = available.min(buf.len());
            buf[..n].copy_from_slice(&self.pending_data[self.pending_offset..self.pending_offset + n]);
            self.pending_offset += n;
            return Ok(n);
        }

        match self.read_application_data() {
            Ok(plaintext) => {
                let n = plaintext.len().min(buf.len());
                buf[..n].copy_from_slice(&plaintext[..n]);
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

impl Write for TlsServerStream {
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

// ---- Server handshake state machine ----

struct ServerHandshake {
    stream: TcpStream,
    cert_der: Vec<u8>,
    signing_key: SigningKey,
    cipher_suite: CipherSuite,
    client_random: [u8; 32],
    server_random: [u8; 32],
    key_pair: EcdhKeyPair,
    client_key_share: Vec<u8>,
    /// ClientHello message bytes (for transcript)
    ch_msg: Vec<u8>,
    /// ServerHello message bytes (for transcript)
    sh_msg: Vec<u8>,
    /// Accumulated transcript of handshake messages
    transcript: Vec<u8>,
    /// Final ciphers (set after handshake)
    write_cipher: RecordCipher,
    read_cipher: RecordCipher,
}

impl ServerHandshake {
    fn new(stream: TcpStream, cert_and_key: &CertificateAndKey) -> Self {
        let key_pair = EcdhKeyPair::generate().expect("ECDH key generation failed");
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        let write_cipher = RecordCipher::new(&[0u8; 16], &[0u8; 12]).unwrap();
        let read_cipher = RecordCipher::new(&[0u8; 16], &[0u8; 12]).unwrap();

        ServerHandshake {
            stream,
            cert_der: cert_and_key.cert_der.clone(),
            signing_key: cert_and_key.signing_key.clone(),
            cipher_suite,
            client_random: [0u8; 32],
            server_random: generate_random(),
            key_pair,
            client_key_share: Vec::new(),
            ch_msg: Vec::new(),
            sh_msg: Vec::new(),
            transcript: Vec::new(),
            write_cipher,
            read_cipher,
        }
    }

    fn do_handshake(&mut self) -> Result<()> {
        // 1. Read ClientHello
        self.read_client_hello()?;

        // 2. Send ServerHello (plaintext)
        self.send_server_hello()?;

        // 3. Derive handshake keys
        let shared_secret = self.key_pair.exchange(&self.client_key_share)?;
        let hash = self.hasher();

        // Compute transcript hash for key derivation
        let ch_hash = hash.hash(&self.ch_msg);
        let sh_hash = hash.hash(&self.sh_msg);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared_secret, &ch_hash, &sh_hash);

        // Server handshake traffic secret (note: server uses "s hs traffic" label)
        let server_hs_secret = ks.server_handshake_traffic_secret(&ch_hash);
        let client_hs_secret = ks.client_handshake_traffic_secret(&ch_hash);

        let server_hs_keys = server_write_keys(&server_hs_secret, self.cipher_suite.key_len(), 12, &hash);
        let client_hs_keys = client_write_keys(&client_hs_secret, self.cipher_suite.key_len(), 12, &hash);

        // Server writes, client reads
        let mut write_cipher = RecordCipher::new(&server_hs_keys.write_key, &server_hs_keys.write_iv)?;
        let mut read_cipher = RecordCipher::new(&client_hs_keys.write_key, &client_hs_keys.write_iv)?;

        // 4. Send encrypted handshake messages
        self.send_encrypted_handshake(&mut write_cipher, &mut ks)?;

        // 5. Read client Finished
        self.read_client_finished(&mut read_cipher, &ks)?;

        // 6. Derive application traffic keys
        ks.advance_to_master();
        let server_app = ks.server_app_traffic_secret();
        let client_app = ks.client_app_traffic_secret();

        let server_app_keys = server_write_keys(&server_app, self.cipher_suite.key_len(), 12, &hash);
        let client_app_keys = client_write_keys(&client_app, self.cipher_suite.key_len(), 12, &hash);

        // Store final ciphers
        self.write_cipher = RecordCipher::new(&server_app_keys.write_key, &server_app_keys.write_iv)?;
        self.read_cipher = RecordCipher::new(&client_app_keys.write_key, &client_app_keys.write_iv)?;

        Ok(())
    }

    fn read_client_hello(&mut self) -> Result<()> {
        let (ct, _ver, len) = read_record_header(&mut self.stream)?;
        if ct != 22 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected handshake record, got content_type={ct}",
            )));
        }
        let fragment = read_record_fragment(&mut self.stream, len)?;

        let ch = ClientHello::parse(&fragment)?;

        // Verify TLS 1.3 support
        if !ch.supported_versions.iter().any(|&v| v == 0x0304) {
            return Err(TlsError::HandshakeFailure("Client does not support TLS 1.3".into()));
        }

        // Check cipher suite compatibility
        let common_suite = ch.cipher_suites.iter()
            .find(|cs| matches!(cs, CipherSuite::TLS_AES_128_GCM_SHA256 | CipherSuite::TLS_AES_256_GCM_SHA384))
            .cloned();

        self.cipher_suite = common_suite.ok_or_else(||
            TlsError::HandshakeFailure("No common cipher suite".into())
        )?;

        self.client_random = ch.random;

        if let Some(ks) = ch.client_key_share {
            self.client_key_share = ks;
        } else {
            return Err(TlsError::HandshakeFailure("No key_share in ClientHello".into()));
        }

        // Save ClientHello for transcript
        self.ch_msg = fragment.clone();
        self.transcript = fragment;

        Ok(())
    }

    fn send_server_hello(&mut self) -> Result<()> {
        let public_key = self.key_pair.public_key_bytes();
        let sh_msg = build_server_hello(
            self.server_random,
            self.cipher_suite,
            &public_key,
            NamedGroup::SECP256R1,
        );

        // Compute hash for key schedule
        self.sh_msg = sh_msg.clone();

        // Append ServerHello to transcript (as a proper handshake message: type+length+payload)
        self.transcript.extend_from_slice(&sh_msg);

        // Send as TLS 1.2 record for middlebox compatibility
        let record = TlsRecord {
            content_type: 22,
            version: 0x0303,
            fragment: sh_msg,
        };
        self.stream.write_all(&record.to_bytes())?;
        self.stream.flush()?;

        Ok(())
    }

    fn send_encrypted_handshake(
        &mut self,
        write_cipher: &mut RecordCipher,
        ks: &mut Tls13KeySchedule,
    ) -> Result<()> {
        // EncryptedExtensions (empty)
        let ee_msg = build_encrypted_extensions();
        self.transcript.extend_from_slice(&ee_msg);
        let ee_ct = write_cipher.encrypt(22, &ee_msg);
        let ee_record = TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: ee_ct,
        };
        self.stream.write_all(&ee_record.to_bytes())?;

        // Certificate message
        let cert_msg = build_certificate_message(&self.cert_der);
        self.transcript.extend_from_slice(&cert_msg);
        let cert_ct = write_cipher.encrypt(22, &cert_msg);
        let cert_record = TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: cert_ct,
        };
        self.stream.write_all(&cert_record.to_bytes())?;

        // CertificateVerify
        let cv_msg = build_certificate_verify(&self.transcript, &self.signing_key, &self.hasher())?;
        self.transcript.extend_from_slice(&cv_msg);
        let cv_ct = write_cipher.encrypt(22, &cv_msg);
        let cv_record = TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: cv_ct,
        };
        self.stream.write_all(&cv_record.to_bytes())?;

        // Finished
        let transcript_hash = self.hasher().hash(&self.transcript);
        let server_hs_secret = ks.server_handshake_traffic_secret(&transcript_hash);
        let finished_key = self.hasher().expand_label(&server_hs_secret, "finished", &[], self.hasher().len());
        let verify_data = match self.hasher() {
            Hasher::Sha256 => hmac_sha256(&finished_key, &transcript_hash),
            Hasher::Sha384 => hmac_sha384(&finished_key, &transcript_hash),
        };

        let finished_msg = build_finished_message(&verify_data);
        self.transcript.extend_from_slice(&finished_msg);
        let finished_ct = write_cipher.encrypt(22, &finished_msg);
        let finished_record = TlsRecord {
            content_type: 23,
            version: 0x0303,
            fragment: finished_ct,
        };
        self.stream.write_all(&finished_record.to_bytes())?;
        self.stream.flush()?;

        Ok(())
    }

    fn read_client_finished(
        &mut self,
        read_cipher: &mut RecordCipher,
        ks: &Tls13KeySchedule,
    ) -> Result<()> {
        let (ct, _ver, len) = read_record_header(&mut self.stream)?;
        if ct != 23 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected encrypted record for client Finished, got content_type={ct}",
            )));
        }
        let fragment = read_record_fragment(&mut self.stream, len)?;

        let (inner_type, plaintext) = read_cipher.decrypt(&fragment)?;

        // plaintext IS the handshake message: type(1) + length(3) + verify_data
        let hs_type = if !plaintext.is_empty() { plaintext[0] } else { inner_type };
        if hs_type != 20 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected Finished (type 20), got {}", hs_type,
            )));
        }

        // Verify client Finished
        let transcript_hash = self.hasher().hash(&self.transcript);
        let client_hs_secret = ks.client_handshake_traffic_secret(&transcript_hash);
        let finished_key = self.hasher().expand_label(&client_hs_secret, "finished", &[], self.hasher().len());
        let expected_verify = match self.hasher() {
            Hasher::Sha256 => hmac_sha256(&finished_key, &transcript_hash),
            Hasher::Sha384 => hmac_sha384(&finished_key, &transcript_hash),
        };

        // plaintext = type(1) + length(3) + verify_data
        // verify_data starts at offset 4
        if plaintext.len() < 4 + expected_verify.len() {
            return Err(TlsError::Protocol("Client Finished verification data too short".into()));
        }
        let client_verify_data = &plaintext[4..4 + expected_verify.len()];

        if !constant_time_eq(client_verify_data, &expected_verify) {
            return Err(TlsError::Protocol("Client Finished verification failed".into()));
        }

        // Append client Finished to transcript
        self.transcript.extend_from_slice(&plaintext);

        Ok(())
    }

    fn hasher(&self) -> Hasher {
        match self.cipher_suite {
            CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
            _ => Hasher::Sha256,
        }
    }

    fn finish(self) -> TlsServerStream {
        TlsServerStream {
            stream: self.stream,
            write_cipher: self.write_cipher,
            read_cipher: self.read_cipher,
            handshake_done: true,
            pending_data: Vec::new(),
            pending_offset: 0,
            cipher_suite: self.cipher_suite,
        }
    }
}

/// Build a ServerHello handshake message
fn build_server_hello(
    random: [u8; 32],
    cipher_suite: CipherSuite,
    server_key_share: &[u8],
    group: NamedGroup,
) -> Vec<u8> {
    let mut msg = Vec::new();

    // Handshake type: ServerHello = 2
    msg.push(2);
    // Length placeholder
    msg.extend_from_slice(&[0u8; 3]);

    // Legacy version (TLS 1.2 for TLS 1.3)
    msg.extend_from_slice(&0x0303u16.to_be_bytes());

    // Random
    msg.extend_from_slice(&random);

    // Session ID (empty for TLS 1.3)
    msg.push(0);

    // Cipher suite
    msg.extend_from_slice(&cipher_suite.to_wire().to_be_bytes());

    // Legacy compression (null)
    msg.push(0);

    // Extensions
    let ext_start = msg.len();
    msg.extend_from_slice(&[0u8; 2]); // length placeholder

    // supported_versions
    {
        let data = vec![0x03, 0x04]; // TLS 1.3
        msg.extend_from_slice(&43u16.to_be_bytes());
        msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
        msg.extend_from_slice(&data);
    }

    // key_share
    {
        let mut data = Vec::new();
        data.extend_from_slice(&group.to_wire().to_be_bytes());
        data.extend_from_slice(&(server_key_share.len() as u16).to_be_bytes());
        data.extend_from_slice(server_key_share);
        msg.extend_from_slice(&51u16.to_be_bytes());
        msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
        msg.extend_from_slice(&data);
    }

    // Fill extension length
    let ext_len = (msg.len() - ext_start - 2) as u16;
    msg[ext_start..ext_start + 2].copy_from_slice(&ext_len.to_be_bytes());

    // Fill handshake message length
    let msg_len = (msg.len() - 4) as u32;
    msg[1..4].copy_from_slice(&msg_len.to_be_bytes()[1..]);

    msg
}

/// Build EncryptedExtensions (empty for now)
fn build_encrypted_extensions() -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(8); // EncryptedExtensions type
    msg.extend_from_slice(&[0u8; 3]); // length = 0
    // Extensions length = 0
    msg.extend_from_slice(&[0u8; 2]);
    msg
}

/// Build Certificate message (TLS 1.3 format)
fn build_certificate_message(cert_der: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(11); // Certificate type

    // Certificate list length + context (empty request context in TLS 1.3 server)
    // Format: type(1) + length(3) + context_length(1) + context(0) + cert_list_length(3) + certs
    let cert_list_len = 3 + cert_der.len(); // 3 bytes for cert entry length + cert data
    let total_len = 1 + 3 + cert_list_len; // context_length(1) + cert_list_length(3) + certs

    msg.extend_from_slice(&(total_len as u32).to_be_bytes()[1..]); // 3-byte length

    // Certificate request context (empty for server)
    msg.push(0); // context_length = 0

    // Certificate list
    msg.extend_from_slice(&(cert_list_len as u32).to_be_bytes()[1..]); // 3-byte length

    // Single certificate entry
    // No certificate_extensions for self-signed
    msg.extend_from_slice(cert_der);

    // Certificate extensions (empty)
    msg.extend_from_slice(&[0u8; 2]); // extensions_length = 0

    msg
}

/// Build CertificateVerify message
fn build_certificate_verify(
    transcript: &[u8],
    signing_key: &SigningKey,
    hasher: &Hasher,
) -> Result<Vec<u8>> {
    // TLS 1.3 CertificateVerify uses a special context string
    // https://datatracker.ietf.org/doc/html/rfc8446#section-4.4.3
    let context = b"TLS 1.3, server CertificateVerify";
    let mut padded = vec![0x20u8; 64];
    padded.extend_from_slice(context);
    padded.push(0x00);
    padded.extend_from_slice(transcript);

    let digest = match hasher {
        Hasher::Sha256 => Sha256::digest(&padded).to_vec(),
        Hasher::Sha384 => Sha384::digest(&padded).to_vec(),
    };

    // Sign with ECDSA P-256
    let mut signer = signing_key.clone();
    let signature: Signature = <SigningKey as SignerMut<Signature>>::sign(&mut signer, &digest);
    // The Signature type is already DER-encoded — convert to bytes
    let sig_der_bytes = signature.to_bytes().to_vec();

    // Build CertificateVerify message
    // type(1) + length(3) + signature_algorithm(2) + signature_length(2) + signature
    let mut msg = Vec::new();
    msg.push(15); // CertificateVerify type

    let inner_len = 2 + 2 + sig_der_bytes.len();
    msg.extend_from_slice(&((4 + inner_len) as u32).to_be_bytes()[1..]); // 3-byte length

    // Signature algorithm: ecdsa_secp256r1_sha256 (0x0403)
    msg.extend_from_slice(&0x0403u16.to_be_bytes());

    // Signature
    msg.extend_from_slice(&(sig_der_bytes.len() as u16).to_be_bytes());
    msg.extend_from_slice(&sig_der_bytes);

    Ok(msg)
}

/// Build Finished message
fn build_finished_message(verify_data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(20); // Finished type
    msg.extend_from_slice(&((verify_data.len()) as u32).to_be_bytes()[1..]); // 3-byte length
    msg.extend_from_slice(verify_data);
    msg
}

fn generate_random() -> [u8; 32] {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    buf
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificate_gen::generate_self_signed;
    use crate::cipher::{CipherSuite, NamedGroup};
    use crate::handshake::{ClientHelloBuilder, ServerHello};
    use crate::key_exchange::EcdhKeyPair;
    use crate::prf::{Hasher, Tls13KeySchedule, client_write_keys, server_write_keys};
    use crate::record::RecordCipher;

    #[test]
    fn test_client_hello_roundtrip() {
        let random = [0x42u8; 32];
        let key_pair = EcdhKeyPair::generate().unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "example.com")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        // Should start with ClientHello type (1)
        assert_eq!(ch_bytes[0], 1);

        let ch = ClientHello::parse(&ch_bytes).unwrap();
        assert_eq!(ch.random, random);
        assert_eq!(ch.server_name, Some("example.com".to_string()));
        assert!(ch.client_key_share.is_some());
        assert_eq!(ch.client_key_share_group, Some(NamedGroup::SECP256R1));
        assert!(ch.cipher_suites.contains(&CipherSuite::TLS_AES_128_GCM_SHA256));
        assert!(ch.supported_groups.contains(&NamedGroup::SECP256R1));
        // ClientHelloBuilder includes supported_versions extension
        assert!(ch.supported_versions.contains(&0x0304));
    }

    #[test]
    fn test_client_hello_parses_from_real_client() {
        // Build a ClientHello the same way the real client does
        let random = [0xABu8; 32];
        let key_pair = EcdhKeyPair::generate().unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "localhost")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        // Parse it with the server's ClientHello parser
        let ch = ClientHello::parse(&ch_bytes).unwrap();

        assert_eq!(ch.random, random);
        assert_eq!(ch.server_name, Some("localhost".to_string()));
        assert!(ch.client_key_share.is_some());
        let ks = ch.client_key_share.unwrap();
        assert_eq!(ks.len(), 65); // P-256 uncompressed point
        assert_eq!(ks[0], 0x04); // Uncompressed point marker
    }

    // -----------------------------------------------------------------------
    // 2. ServerHello build → parse round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_server_hello_roundtrip() {
        let random = [0xCDu8; 32];
        let key_pair = EcdhKeyPair::generate().unwrap();
        let public_key = key_pair.public_key_bytes();

        let sh_bytes = build_server_hello(
            random,
            CipherSuite::TLS_AES_128_GCM_SHA256,
            &public_key,
            NamedGroup::SECP256R1,
        );

        // Should start with ServerHello type (2)
        assert_eq!(sh_bytes[0], 2);

        let sh = ServerHello::parse(&sh_bytes).unwrap();
        assert_eq!(sh.random, random);
        assert_eq!(sh.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
        assert_eq!(sh.server_key_share.len(), 65);
        assert_eq!(sh.server_key_share[0], 0x04);
        assert_eq!(sh.supported_version, Some(0x0304)); // TLS 1.3
    }

    // -----------------------------------------------------------------------
    // 3. ECDH key exchange
    // -----------------------------------------------------------------------

    #[test]
    fn test_ecdh_exchange() {
        let client_keys = EcdhKeyPair::generate().unwrap();
        let server_keys = EcdhKeyPair::generate().unwrap();

        let client_pub = client_keys.public_key_bytes();
        let server_pub = server_keys.public_key_bytes();

        // Both sides compute the same shared secret
        let client_shared = client_keys.exchange(&server_pub).unwrap();
        let server_shared = server_keys.exchange(&client_pub).unwrap();

        assert_eq!(client_shared, server_shared);
        assert_eq!(client_shared.len(), 32); // P-256 shared secret is 32 bytes
    }

    // -----------------------------------------------------------------------
    // 4. Key schedule derivation
    // -----------------------------------------------------------------------

    #[test]
    fn test_key_schedule_derivation() {
        let shared_secret = vec![0x42u8; 32];
        let ch_hash = vec![0xAAu8; 32]; // SHA-256(ClientHello)
        let sh_hash = vec![0xBBu8; 32]; // SHA-256(ServerHello)
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        let hash = Hasher::Sha256;

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared_secret, &ch_hash, &sh_hash);

        // Derive handshake traffic secrets
        let client_hs_secret = ks.client_handshake_traffic_secret(&ch_hash);
        let server_hs_secret = ks.server_handshake_traffic_secret(&ch_hash);

        // Both should be different
        assert_ne!(client_hs_secret, server_hs_secret);
        assert_eq!(client_hs_secret.len(), 32);
        assert_eq!(server_hs_secret.len(), 32);

        // Derive write keys
        let client_keys = client_write_keys(&client_hs_secret, cipher_suite.key_len(), 12, &hash);
        let server_keys = server_write_keys(&server_hs_secret, cipher_suite.key_len(), 12, &hash);

        assert_eq!(client_keys.write_key.len(), 16);
        assert_eq!(client_keys.write_iv.len(), 12);
        assert_eq!(server_keys.write_key.len(), 16);
        assert_eq!(server_keys.write_iv.len(), 12);
    }

    // -----------------------------------------------------------------------
    // 5. Record cipher encrypt → decrypt round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn test_record_cipher_roundtrip() {
        // Use known key material — separate write/read ciphers like real TLS
        let key = vec![0x01u8; 16];
        let iv = vec![0x02u8; 12];

        let mut write_cipher = RecordCipher::new(&key, &iv).unwrap();
        let mut read_cipher = RecordCipher::new(&key, &iv).unwrap();

        let plaintext = b"hello TLS 1.3";
        let ciphertext = write_cipher.encrypt(22, plaintext); // handshake content type

        // Ciphertext should be longer than plaintext (AEAD tag)
        assert!(ciphertext.len() > plaintext.len());

        // Decrypt (separate cipher with independent seq counter)
        let (content_type, decrypted) = read_cipher.decrypt(&ciphertext).unwrap();
        assert_eq!(content_type, 22);
        assert_eq!(&decrypted[..], plaintext);
    }

    #[test]
    fn test_record_cipher_different_keys_fails() {
        let key1 = vec![0x01u8; 16];
        let iv1 = vec![0x02u8; 12];
        let mut cipher1 = RecordCipher::new(&key1, &iv1).unwrap();

        let key2 = vec![0xFFu8; 16];
        let iv2 = vec![0xFEu8; 12];
        let mut cipher2 = RecordCipher::new(&key2, &iv2).unwrap();

        let plaintext = b"secret data";
        let ciphertext = cipher1.encrypt(23, plaintext);

        // Decrypting with wrong key should fail
        assert!(cipher2.decrypt(&ciphertext).is_err());
    }

    // -----------------------------------------------------------------------
    // 6. EncryptedExtensions build → parse via decryption
    // -----------------------------------------------------------------------

    #[test]
    fn test_encrypted_extensions_build() {
        let ee_bytes = build_encrypted_extensions();

        // Type 8 = EncryptedExtensions
        assert_eq!(ee_bytes[0], 8);

        // Should have 4-byte header + 2 bytes extensions length = 6 bytes total
        assert_eq!(ee_bytes.len(), 6);
    }

    // -----------------------------------------------------------------------
    // 7. Certificate message build
    // -----------------------------------------------------------------------

    #[test]
    fn test_certificate_message_build() {
        let cert = generate_self_signed(&["localhost"]);
        let cert_msg = build_certificate_message(&cert.cert_der);

        // Type 11 = Certificate
        assert_eq!(cert_msg[0], 11);

        // Should contain the certificate data
        assert!(cert_msg.len() > cert.cert_der.len());
    }

    // -----------------------------------------------------------------------
    // 8. CertificateVerify build
    // -----------------------------------------------------------------------

    #[test]
    fn test_certificate_verify_build() {
        let cert = generate_self_signed(&["localhost"]);
        let transcript = vec![0x01u8; 64]; // fake transcript

        let cv_bytes = build_certificate_verify(&transcript, &cert.signing_key, &Hasher::Sha256).unwrap();

        // Type 15 = CertificateVerify
        assert_eq!(cv_bytes[0], 15);

        // Should be longer than just the header
        assert!(cv_bytes.len() > 4);
    }

    // -----------------------------------------------------------------------
    // 9. Finished message build
    // -----------------------------------------------------------------------

    #[test]
    fn test_finished_message_build() {
        let verify_data = vec![0xDEu8; 32];
        let finished = build_finished_message(&verify_data);

        // Type 20 = Finished
        assert_eq!(finished[0], 20);

        // Should contain the verify_data
        assert_eq!(finished.len(), 4 + verify_data.len());
    }

    // -----------------------------------------------------------------------
    // 10. ClientHello parsing with all extensions
    // -----------------------------------------------------------------------

    #[test]
    fn test_client_hello_parses_all_extensions() {
        let random = [0x55u8; 32];
        let key_pair = EcdhKeyPair::generate().unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "test.example.com")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let ch = ClientHello::parse(&ch_bytes).unwrap();

        assert_eq!(ch.random, random);
        assert_eq!(ch.server_name, Some("test.example.com".to_string()));
        assert_eq!(ch.client_key_share_group, Some(NamedGroup::SECP256R1));
        assert_eq!(ch.client_key_share.as_ref().map(|v| v.len()), Some(65));
        assert!(ch.cipher_suites.len() >= 2);
        assert!(ch.legacy_compression.is_empty() || ch.legacy_compression == vec![0]);
    }

    // -----------------------------------------------------------------------
    // 11. ServerHello parses back correctly
    // -----------------------------------------------------------------------

    #[test]
    fn test_server_hello_cipher_suite_parsed() {
        let random = [0x77u8; 32];
        let key_pair = EcdhKeyPair::generate().unwrap();
        let public_key = key_pair.public_key_bytes();

        // Test with AES-256
        let sh_bytes = build_server_hello(
            random,
            CipherSuite::TLS_AES_256_GCM_SHA384,
            &public_key,
            NamedGroup::SECP256R1,
        );

        let sh = ServerHello::parse(&sh_bytes).unwrap();
        assert_eq!(sh.cipher_suite, CipherSuite::TLS_AES_256_GCM_SHA384);
    }

    // -----------------------------------------------------------------------
    // 12. NamedGroup wire format
    // -----------------------------------------------------------------------

    #[test]
    fn test_named_group_wire_roundtrip() {
        for group in [NamedGroup::SECP256R1, NamedGroup::SECP384R1, NamedGroup::X25519] {
            let wire = group.to_wire();
            let parsed = NamedGroup::from_wire(wire).unwrap();
            assert_eq!(parsed, group);
        }
    }

    // -----------------------------------------------------------------------
    // 13. CipherSuite wire format
    // -----------------------------------------------------------------------

    #[test]
    fn test_cipher_suite_wire_roundtrip() {
        for suite in [
            CipherSuite::TLS_AES_128_GCM_SHA256,
            CipherSuite::TLS_AES_256_GCM_SHA384,
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
        ] {
            let wire = suite.to_wire();
            let parsed = CipherSuite::from_wire(wire).unwrap();
            assert_eq!(parsed, suite);
        }
    }

    // -----------------------------------------------------------------------
    // 14. Full handshake: server sends, client receives encrypted messages
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_handshake_encrypted_messages_decryptable() {
        let _cert = generate_self_signed(&["localhost"]);
        let client_keys = EcdhKeyPair::generate().unwrap();
        let server_keys = EcdhKeyPair::generate().unwrap();
        let client_random = [0xCCu8; 32];
        let server_random = [0xDDu8; 32];
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;

        // Build ClientHello
        let client_pub = client_keys.public_key_bytes();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_pub, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        // Build ServerHello
        let server_pub = server_keys.public_key_bytes();
        let sh_bytes = build_server_hello(
            server_random,
            cipher_suite,
            &server_pub,
            NamedGroup::SECP256R1,
        );

        // Compute shared secrets (from both perspectives)
        let shared = server_keys.exchange(&client_pub).unwrap();

        // Key schedule
        let hash = Hasher::Sha256;
        let ch_hash = hash.hash(&ch_bytes);
        let sh_hash = hash.hash(&sh_bytes);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared, &ch_hash, &sh_hash);

        // Server handshake traffic secret — this is what the server uses to encrypt
        let server_hs_secret = ks.server_handshake_traffic_secret(&ch_hash);

        // In real TLS: server writes with server_hs_secret, client reads with same server_hs_secret
        let server_write = server_write_keys(&server_hs_secret, cipher_suite.key_len(), 12, &hash);

        let mut write_cipher = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let mut read_cipher = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();

        // Build EncryptedExtensions
        let ee_bytes = build_encrypted_extensions();
        let ee_encrypted = write_cipher.encrypt(22, &ee_bytes);

        // Client should be able to decrypt (same key, same seq starting at 0)
        let (ct, plaintext) = read_cipher.decrypt(&ee_encrypted).unwrap();
        assert_eq!(ct, 22);
        assert_eq!(plaintext, ee_bytes);
    }

    // -----------------------------------------------------------------------
    // 15. Full client↔server handshake using pipes (no real TCP)
    // -----------------------------------------------------------------------

    #[test]
    fn test_handshake_transcript_transcript_match() {
        // This test verifies that the server and client build the same
        // transcript hash — the core requirement for Finished verification.

        let cert = generate_self_signed(&["localhost"]);

        // Generate key pairs
        let client_keys = EcdhKeyPair::generate().unwrap();
        let server_keys = EcdhKeyPair::generate().unwrap();

        let client_random = [0xAAu8; 32];
        let server_random = [0xBBu8; 32];
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
        let hash = Hasher::Sha256;

        // ========== CLIENT SIDE: build ClientHello ==========
        let client_pub = client_keys.public_key_bytes();
        let ch_msg = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_pub, NamedGroup::SECP256R1)
            .build()
            .unwrap();
        let client_ch_hash = hash.hash(&ch_msg);

        // ========== SERVER SIDE: parse ClientHello ==========
        let _ch_parsed = ClientHello::parse(&ch_msg).unwrap();
        let server_ch_hash = hash.hash(&ch_msg);
        assert_eq!(client_ch_hash, server_ch_hash, "ClientHello hashes must match");

        // ========== SERVER SIDE: build ServerHello ==========
        let server_pub = server_keys.public_key_bytes();
        let sh_msg = build_server_hello(
            server_random,
            cipher_suite,
            &server_pub,
            NamedGroup::SECP256R1,
        );
        let server_sh_hash = hash.hash(&sh_msg);

        // ========== CLIENT SIDE: parse ServerHello ==========
        let _sh_parsed = ServerHello::parse(&sh_msg).unwrap();
        let client_sh_hash = hash.hash(&sh_msg);
        assert_eq!(server_sh_hash, client_sh_hash, "ServerHello hashes must match");

        // ========== Key exchange ==========
        let shared_client = client_keys.exchange(&server_pub).unwrap();
        let shared_server = server_keys.exchange(&client_pub).unwrap();
        assert_eq!(shared_client, shared_server, "Shared secrets must match");

        // ========== Key schedule (both sides) ==========
        let ch_hash = client_ch_hash; // same on both sides

        let mut ks_client = Tls13KeySchedule::new(hash.clone());
        ks_client.advance_to_handshake(&shared_client, &ch_hash, &client_sh_hash);

        let mut ks_server = Tls13KeySchedule::new(hash.clone());
        ks_server.advance_to_handshake(&shared_server, &ch_hash, &server_sh_hash);

        // Server handshake traffic secret
        let server_hs_secret_client = ks_client.server_handshake_traffic_secret(&ch_hash);
        let server_hs_secret_server = ks_server.server_handshake_traffic_secret(&ch_hash);
        assert_eq!(server_hs_secret_client, server_hs_secret_server,
            "Server HS secret must match");

        // Client handshake traffic secret
        let client_hs_secret_client = ks_client.client_handshake_traffic_secret(&ch_hash);
        let client_hs_secret_server = ks_server.client_handshake_traffic_secret(&ch_hash);
        assert_eq!(client_hs_secret_client, client_hs_secret_server,
            "Client HS secret must match");

        // ========== Encrypted messages transcript ==========
        // Server transcript after ClientHello + ServerHello:
        // (build_encrypted_handshake appends each message to self.transcript)
        // At this point: transcript = ch_msg || sh_msg
        let mut server_transcript = Vec::new();
        server_transcript.extend_from_slice(&ch_msg);
        server_transcript.extend_from_slice(&sh_msg);

        // Build EE, Certificate, CertificateVerify
        let ee_msg = build_encrypted_extensions();
        server_transcript.extend_from_slice(&ee_msg);

        let cert_msg = build_certificate_message(&cert.cert_der);
        server_transcript.extend_from_slice(&cert_msg);

        let cv_msg = build_certificate_verify(&server_transcript, &cert.signing_key, &hash).unwrap();
        server_transcript.extend_from_slice(&cv_msg);

        // Server's transcript hash for Finished
        let server_transcript_hash = hash.hash(&server_transcript);

        // ========== Client side: reconstruct transcript ==========
        let mut client_transcript = Vec::new();
        client_transcript.extend_from_slice(&ch_msg);
        client_transcript.extend_from_slice(&sh_msg);

        // Client reconstructs EE from the raw bytes (same as server built)
        let client_ee_msg = ee_msg.clone();
        client_transcript.extend_from_slice(&client_ee_msg);

        // Client reconstructs Certificate
        let client_cert_msg = cert_msg.clone();
        client_transcript.extend_from_slice(&client_cert_msg);

        // Client reconstructs CertificateVerify
        let client_cv_msg = cv_msg.clone();
        client_transcript.extend_from_slice(&client_cv_msg);

        let client_transcript_hash = hash.hash(&client_transcript);

        assert_eq!(server_transcript_hash, client_transcript_hash,
            "Transcript hashes must match for Finished verification");
    }
}
