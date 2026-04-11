//! TLS 1.3 handshake messages (RFC 8446 §4).
//!
//! The TLS 1.3 handshake flow:
//! 1. Client → Server: ClientHello (with key_share, supported_versions, etc.)
//! 2. Server → Client: Encrypted [ServerHello, EncryptedExtensions, Certificate, CertificateVerify, Finished]
//! 3. Client → Server: Encrypted [Finished]
//!
//! All messages after ServerHello are encrypted.

use crate::cipher::{CipherSuite, NamedGroup};
use crate::Result;
use crate::TlsError;
use std::io::Read;

/// ClientHello message builder
pub struct ClientHelloBuilder {
    random: [u8; 32],
    session_id: Vec<u8>,
    cipher_suites: Vec<CipherSuite>,
    server_name: String,
    supported_groups: Vec<NamedGroup>,
    key_share: Vec<u8>,
}

impl ClientHelloBuilder {
    /// Create a new ClientHello builder with the given random bytes and server name.
    pub fn new(random: [u8; 32], server_name: &str) -> Self {
        ClientHelloBuilder {
            random,
            session_id: vec![0u8; 32], // TLS 1.3 compat: still include for middleboxes
            cipher_suites: CipherSuite::client_default(),
            server_name: server_name.to_string(),
            supported_groups: NamedGroup::client_default(),
            key_share: Vec::new(),
        }
    }

    /// Add a key share entry for the given group and public key.
    pub fn key_share(mut self, public_key: &[u8], group: NamedGroup) -> Self {
        // KeyShareEntry encoding:
        //   group (2 bytes) + key_exchange length (2 bytes) + key_exchange (variable)
        self.key_share.clear();
        self.key_share.extend_from_slice(&group.to_wire().to_be_bytes());
        self.key_share.extend_from_slice(&(public_key.len() as u16).to_be_bytes());
        self.key_share.extend_from_slice(public_key);
        self
    }

    /// Serialize ClientHello handshake message (without record header)
    pub fn build(&self) -> Result<Vec<u8>> {
        let mut msg = Vec::new();

        // Handshake type: ClientHello = 1
        msg.push(1);
        // Handshake length placeholder (3 bytes)
        msg.extend_from_slice(&[0u8; 3]);

        // Legacy version (TLS 1.2 = 0x0303 for TLS 1.3 compatibility)
        msg.extend_from_slice(&0x0303u16.to_be_bytes());

        // Random (32 bytes)
        msg.extend_from_slice(&self.random);

        // Legacy session ID
        msg.push(self.session_id.len() as u8);
        msg.extend_from_slice(&self.session_id);

        // Cipher suites
        let cs_len = self.cipher_suites.len() * 2;
        msg.extend_from_slice(&(cs_len as u16).to_be_bytes());
        for cs in &self.cipher_suites {
            msg.extend_from_slice(&cs.to_wire().to_be_bytes());
        }

        // Legacy compression methods (always just null)
        msg.push(1);
        msg.push(0);

        // Extensions
        let ext_start = msg.len();
        msg.extend_from_slice(&[0u8; 2]); // length placeholder

        // 1. supported_versions (ext 43)
        {
            let data = vec![0x04, 0x03, 0x04]; // TLS 1.3 only
            msg.extend_from_slice(&43u16.to_be_bytes());
            msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
            msg.extend_from_slice(&data);
        }

        // 2. supported_groups (ext 10)
        {
            let mut data = Vec::new();
            data.extend_from_slice(&((self.supported_groups.len() * 2) as u16).to_be_bytes());
            for g in &self.supported_groups {
                data.extend_from_slice(&g.to_wire().to_be_bytes());
            }
            msg.extend_from_slice(&10u16.to_be_bytes());
            msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
            msg.extend_from_slice(&data);
        }

        // 3. signature_algorithms (ext 13)
        {
            // ecdsa_secp256r1_sha256 (0x0403), rsa_pss_rsae_sha256 (0x0804)
            let data: Vec<u8> = vec![0x00, 0x04, 0x04, 0x03, 0x08, 0x04];
            msg.extend_from_slice(&13u16.to_be_bytes());
            msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
            msg.extend_from_slice(&data);
        }

        // 4. key_share (ext 51)
        {
            let mut data = Vec::new();
            data.extend_from_slice(&(self.key_share.len() as u16).to_be_bytes());
            data.extend_from_slice(&self.key_share);
            msg.extend_from_slice(&51u16.to_be_bytes());
            msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
            msg.extend_from_slice(&data);
        }

        // 5. psk_key_exchange_modes (ext 45) — required for TLS 1.3
        {
            let data = vec![0x01, 0x01]; // psk_dhe_ke
            msg.extend_from_slice(&45u16.to_be_bytes());
            msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
            msg.extend_from_slice(&data);
        }

        // 6. server_name (ext 0) — SNI
        {
            let mut data = Vec::new();
            let name_entry_len = 1 + 2 + self.server_name.len(); // type(1) + len(2) + name
            data.extend_from_slice(&(name_entry_len as u16).to_be_bytes());
            data.push(0); // host_name type
            data.extend_from_slice(&(self.server_name.len() as u16).to_be_bytes());
            data.extend_from_slice(self.server_name.as_bytes());
            msg.extend_from_slice(&0u16.to_be_bytes());
            msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
            msg.extend_from_slice(&data);
        }

        // Fill extension length
        let ext_len = (msg.len() - ext_start - 2) as u16;
        msg[ext_start..ext_start + 2].copy_from_slice(&ext_len.to_be_bytes());

        // Fill handshake message length
        let msg_len = (msg.len() - 4) as u32;
        msg[1..4].copy_from_slice(&msg_len.to_be_bytes()[1..]);

        Ok(msg)
    }
}

/// Parsed ServerHello message.
#[derive(Debug)]
pub struct ServerHello {
    /// Legacy protocol version (should be 0x0303 for TLS 1.2).
    pub legacy_version: u16,
    /// 32 bytes of server random data.
    pub random: [u8; 32],
    /// Session ID echoed from ClientHello.
    pub session_id: Vec<u8>,
    /// The cipher suite selected by the server.
    pub cipher_suite: CipherSuite,
    /// Legacy compression method (always 0 for TLS 1.3).
    pub legacy_compression: u8,
    /// Server's key exchange data.
    pub server_key_share: Vec<u8>,
    /// Negotiated protocol version (Some(0x0304) for TLS 1.3).
    pub supported_version: Option<u16>,
}

impl ServerHello {
    /// Parse from handshake fragment bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(TlsError::HandshakeFailure("ServerHello too short".into()));
        }

        if data[0] != 2 {
            return Err(TlsError::HandshakeFailure(
                format!("Expected ServerHello (type 2), got {}", data[0]),
            ));
        }

        let msg_len = u32::from_be_bytes([0, data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + msg_len {
            return Err(TlsError::HandshakeFailure("ServerHello truncated".into()));
        }

        let msg = &data[4..4 + msg_len];
        let mut pos = 0;

        // Legacy version
        if pos + 2 > msg.len() {
            return Err(TlsError::HandshakeFailure("ServerHello: legacy_version truncated".into()));
        }
        let legacy_version = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
        pos += 2;

        // Random
        if pos + 32 > msg.len() {
            return Err(TlsError::HandshakeFailure("ServerHello: random truncated".into()));
        }
        let mut random = [0u8; 32];
        random.copy_from_slice(&msg[pos..pos + 32]);
        pos += 32;

        // Session ID
        if pos >= msg.len() {
            return Err(TlsError::HandshakeFailure("ServerHello: session_id length missing".into()));
        }
        let sid_len = msg[pos] as usize;
        pos += 1;
        if pos + sid_len > msg.len() {
            return Err(TlsError::HandshakeFailure("ServerHello: session_id truncated".into()));
        }
        let session_id = msg[pos..pos + sid_len].to_vec();
        pos += sid_len;

        // Cipher suite
        if pos + 2 > msg.len() {
            return Err(TlsError::HandshakeFailure("ServerHello: cipher_suite truncated".into()));
        }
        let cs = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
        let cipher_suite = CipherSuite::from_wire(cs)
            .map_err(|e| TlsError::HandshakeFailure(e))?;
        pos += 2;

        // Legacy compression
        if pos >= msg.len() {
            return Err(TlsError::HandshakeFailure("ServerHello: compression truncated".into()));
        }
        let legacy_compression = msg[pos];
        pos += 1;

        // Extensions
        let mut server_key_share = Vec::new();
        let mut supported_version = None;

        if pos < msg.len() {
            if pos + 2 > msg.len() {
                return Err(TlsError::HandshakeFailure("ServerHello: ext_len truncated".into()));
            }
            let ext_len = u16::from_be_bytes([msg[pos], msg[pos + 1]]) as usize;
            pos += 2;

            let ext_end = pos + ext_len;
            while pos < ext_end {
                if pos + 4 > ext_end {
                    break;
                }
                let ext_type = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
                let ext_data_len = u16::from_be_bytes([msg[pos + 2], msg[pos + 3]]) as usize;
                pos += 4;

                if pos + ext_data_len > ext_end {
                    break;
                }
                let ext_data = &msg[pos..pos + ext_data_len];
                pos += ext_data_len;

                match ext_type {
                    51 => {
                        // key_share
                        if ext_data_len >= 4 {
                            let _group = u16::from_be_bytes([ext_data[0], ext_data[1]]);
                            let ke_len = u16::from_be_bytes([ext_data[2], ext_data[3]]) as usize;
                            if ext_data_len >= 4 + ke_len {
                                server_key_share = ext_data[4..4 + ke_len].to_vec();
                            }
                        }
                    }
                    43 => {
                        // supported_versions
                        if ext_data_len >= 2 {
                            supported_version = Some(u16::from_be_bytes([ext_data[0], ext_data[1]]));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(ServerHello {
            legacy_version,
            random,
            session_id,
            cipher_suite,
            legacy_compression,
            server_key_share,
            supported_version,
        })
    }
}

/// Read exactly n bytes from the stream
fn read_exact(stream: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    stream.read_exact(buf).map_err(TlsError::Io)
}

/// Read a TLS record header
pub fn read_record_header(stream: &mut impl Read) -> Result<(u8, u16, usize)> {
    let mut hdr = [0u8; 5];
    read_exact(stream, &mut hdr)?;
    let content_type = hdr[0];
    let version = u16::from_be_bytes([hdr[1], hdr[2]]);
    let length = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
    Ok((content_type, version, length))
}

/// Read a full TLS record fragment
pub fn read_record_fragment(stream: &mut impl Read, length: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; length];
    read_exact(stream, &mut buf)?;
    Ok(buf)
}
