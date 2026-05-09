//! TLS 1.3 handshake messages (RFC 8446 §4).
//!
//! The TLS 1.3 handshake flow:
//! 1. Client → Server: ClientHello (with key_share, supported_versions, etc.)
//! 2. Server → Client: Encrypted [ServerHello, EncryptedExtensions, Certificate, CertificateVerify, Finished]
//! 3. Client → Server: Encrypted [Finished]
//!
//! All messages after ServerHello are encrypted.

use super::Result;
use super::TlsError;
use super::cipher::NamedGroup;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use edgerun_crypto::CipherSuite;
use edgerun_encoding::byteorder::{
    push_u16_be, push_u32_be, read_u16_be, read_u24_be, write_u16_be, write_u24_be,
};

/// ClientHello message builder
pub struct ClientHelloBuilder {
    random: [u8; 32],
    session_id: Vec<u8>,
    cipher_suites: Vec<CipherSuite>,
    server_name: String,
    supported_groups: Vec<NamedGroup>,
    key_share: Vec<u8>,
    /// ALPN protocols to advertise (e.g., &["h3"])
    alpn_protocols: Vec<Vec<u8>>,
    /// PSK for session resumption (ticket + binder)
    psk_ticket: Option<Vec<u8>>,
    /// Cipher suite index for the PSK (which cipher suite the ticket was negotiated with)
    psk_cipher_index: Option<usize>,
    /// Cookie from HelloRetryRequest (RFC 8446 §4.2.2)
    hrr_cookie: Option<Vec<u8>>,
}

impl ClientHelloBuilder {
    /// Create a new ClientHello builder with the given random bytes and server name.
    pub fn new(random: [u8; 32], server_name: &str) -> Self {
        ClientHelloBuilder {
            random,
            session_id: random.to_vec(), // TLS 1.3 compat: non-empty, unpredictable session id
            cipher_suites: CipherSuite::client_default(),
            server_name: server_name.to_string(),
            supported_groups: NamedGroup::client_default(),
            key_share: Vec::new(),
            alpn_protocols: Vec::new(),
            psk_ticket: None,
            psk_cipher_index: None,
            hrr_cookie: None,
        }
    }

    /// Add a PSK identity for session resumption (RFC 8446 §4.2.11).
    ///
    /// `ticket` is the opaque ticket from a previous NewSessionTicket.
    /// `cipher_index` is the index into the cipher_suites list (0 for first).
    /// `obfuscated_age` is the ticket age in ms + age_add (from the original ticket).
    pub fn psk_identity(mut self, ticket: &[u8], cipher_index: usize, obfuscated_age: u32) -> Self {
        self.psk_ticket = Some(ticket.to_vec());
        self.psk_cipher_index = Some(cipher_index);
        // Also set the session_id to the first 32 bytes of the ticket hash for middlebox compat
        if ticket.len() >= 32 {
            self.session_id = ticket[..32].to_vec();
        }
        self
    }

    /// Add a key share entry for the given group and public key.
    pub fn key_share(mut self, public_key: &[u8], group: NamedGroup) -> Self {
        // KeyShareEntry encoding:
        //   group (2 bytes) + key_exchange length (2 bytes) + key_exchange (variable)
        self.key_share.clear();
        push_u16_be(&mut self.key_share, group.to_wire());
        push_u16_be(&mut self.key_share, public_key.len() as u16);
        self.key_share.extend_from_slice(public_key);
        self
    }

    /// Set ALPN protocols to advertise (RFC 7301).
    ///
    /// For HTTP/3, use `&["h3"]`.
    pub fn alpn_protocols(mut self, protocols: &[&[u8]]) -> Self {
        self.alpn_protocols = protocols.iter().map(|p| p.to_vec()).collect();
        self
    }

    /// Set the cookie from a HelloRetryRequest (RFC 8446 §4.2.2).
    pub fn cookie(mut self, cookie: &[u8]) -> Self {
        self.hrr_cookie = Some(cookie.to_vec());
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
        push_u16_be(&mut msg, 0x0303);

        // Random (32 bytes)
        msg.extend_from_slice(&self.random);

        // Legacy session ID
        msg.push(self.session_id.len() as u8);
        msg.extend_from_slice(&self.session_id);

        // Cipher suites
        let cs_len = self.cipher_suites.len() * 2;
        push_u16_be(&mut msg, cs_len as u16);
        for cs in &self.cipher_suites {
            push_u16_be(&mut msg, cs.to_wire());
        }

        // Legacy compression methods (always just null)
        msg.push(1);
        msg.push(0);

        // Extensions
        let ext_start = msg.len();
        msg.extend_from_slice(&[0u8; 2]); // length placeholder

        // 1. supported_versions (ext 43)
        {
            let data = vec![0x02, 0x03, 0x04]; // TLS 1.3 only
            push_u16_be(&mut msg, 43);
            push_u16_be(&mut msg, data.len() as u16);
            msg.extend_from_slice(&data);
        }

        // 2. supported_groups (ext 10)
        {
            let mut data = Vec::new();
            push_u16_be(&mut data, (self.supported_groups.len() * 2) as u16);
            for g in &self.supported_groups {
                push_u16_be(&mut data, g.to_wire());
            }
            push_u16_be(&mut msg, 10);
            push_u16_be(&mut msg, data.len() as u16);
            msg.extend_from_slice(&data);
        }

        // 3. signature_algorithms (ext 13)
        {
            let schemes = [
                0x0403u16, // ecdsa_secp256r1_sha256
                0x0804,    // rsa_pss_rsae_sha256
                0x0805,    // rsa_pss_rsae_sha384
                0x0806,    // rsa_pss_rsae_sha512
                0x0807,    // ed25519
                0x0809,    // rsa_pss_pss_sha256
                0x080a,    // rsa_pss_pss_sha384
                0x080b,    // rsa_pss_pss_sha512
            ];
            let mut data = Vec::new();
            push_u16_be(&mut data, (schemes.len() * 2) as u16);
            for scheme in schemes {
                push_u16_be(&mut data, scheme);
            }
            push_u16_be(&mut msg, 13);
            push_u16_be(&mut msg, data.len() as u16);
            msg.extend_from_slice(&data);
        }

        // 4. cookie (ext 44) — from HelloRetryRequest (RFC 8446 §4.2.2)
        if let Some(ref cookie) = self.hrr_cookie {
            push_u16_be(&mut msg, 44);
            push_u16_be(&mut msg, cookie.len() as u16);
            msg.extend_from_slice(cookie);
        }

        // 5. key_share (ext 51)
        {
            let mut data = Vec::new();
            push_u16_be(&mut data, self.key_share.len() as u16);
            data.extend_from_slice(&self.key_share);
            push_u16_be(&mut msg, 51);
            push_u16_be(&mut msg, data.len() as u16);
            msg.extend_from_slice(&data);
        }

        // 6. psk_key_exchange_modes (ext 45) — required for TLS 1.3
        {
            let data = vec![0x01, 0x01]; // psk_dhe_ke
            push_u16_be(&mut msg, 45);
            push_u16_be(&mut msg, data.len() as u16);
            msg.extend_from_slice(&data);
        }

        // 7. server_name (ext 0) — SNI
        {
            let mut data = Vec::new();
            let name_entry_len = 1 + 2 + self.server_name.len(); // type(1) + len(2) + name
            push_u16_be(&mut data, name_entry_len as u16);
            data.push(0); // host_name type
            push_u16_be(&mut data, self.server_name.len() as u16);
            data.extend_from_slice(self.server_name.as_bytes());
            push_u16_be(&mut msg, 0);
            push_u16_be(&mut msg, data.len() as u16);
            msg.extend_from_slice(&data);
        }

        // 8. application_layer_protocol_negiation (ext 16) — ALPN (RFC 7301)
        if !self.alpn_protocols.is_empty() {
            let mut proto_list = Vec::new();
            // Total length placeholder (2 bytes)
            proto_list.extend_from_slice(&[0u8; 2]);
            for proto in &self.alpn_protocols {
                proto_list.push(proto.len() as u8);
                proto_list.extend_from_slice(proto);
            }
            // Fill in total length
            let total_len = (proto_list.len() - 2) as u16;
            write_u16_be(&mut proto_list, 0, total_len);

            push_u16_be(&mut msg, 16); // ALPN extension type
            push_u16_be(&mut msg, proto_list.len() as u16);
            msg.extend_from_slice(&proto_list);
        }

        // 9. pre_shared_key (ext 41) — session resumption (RFC 8446 §4.2.11)
        // MUST be the last extension per RFC 8446 §4.2.11
        if let (Some(ticket), Some(_cipher_idx)) = (&self.psk_ticket, self.psk_cipher_index) {
            let obfuscated_age: u32 = 0; // Simplified — proper implementation needs age tracking
            let mut psk_ext = Vec::new();

            // PSK identities list
            let identities_start = psk_ext.len();
            psk_ext.extend_from_slice(&[0u8; 2]); // identities length placeholder
            // Identity entry
            push_u16_be(&mut psk_ext, ticket.len() as u16);
            psk_ext.extend_from_slice(ticket);
            push_u32_be(&mut psk_ext, obfuscated_age);
            // Fill in identities length
            let id_len = (psk_ext.len() - identities_start - 2) as u16;
            write_u16_be(&mut psk_ext, identities_start, id_len);

            // PSK binders list
            let binders_start = psk_ext.len();
            psk_ext.extend_from_slice(&[0u8; 2]); // binders length placeholder
            // Placeholder binder (1 byte len + zeros) — real binder requires HMAC of truncated CH
            let binder_len = 32; // SHA-256 output
            psk_ext.push(binder_len as u8);
            psk_ext.extend_from_slice(&vec![0u8; binder_len]);
            // Fill in binders length
            let b_len = (psk_ext.len() - binders_start - 2) as u16;
            write_u16_be(&mut psk_ext, binders_start, b_len);

            // We need to move the cipher suite to the front of the list when using PSK
            // so the server sees it first (per RFC 8446 §4.2.11)
            // For now, we just add the extension — server will ignore binder if it
            // can't verify, and fall back to full handshake.

            push_u16_be(&mut msg, 41);
            push_u16_be(&mut msg, psk_ext.len() as u16);
            msg.extend_from_slice(&psk_ext);
        }

        // Fill extension length
        let ext_len = (msg.len() - ext_start - 2) as u16;
        write_u16_be(&mut msg, ext_start, ext_len);

        // Fill handshake message length
        let msg_len = (msg.len() - 4) as u32;
        write_u24_be(&mut msg, 1, msg_len);

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
            return Err(TlsError::HandshakeFailure(format!(
                "Expected ServerHello (type 2), got {}",
                data[0]
            )));
        }

        let msg_len = read_u24_be(data, 1) as usize;
        if data.len() < 4 + msg_len {
            return Err(TlsError::HandshakeFailure("ServerHello truncated".into()));
        }

        let msg = &data[4..4 + msg_len];
        let mut pos = 0;

        // Legacy version
        if pos + 2 > msg.len() {
            return Err(TlsError::HandshakeFailure(
                "ServerHello: legacy_version truncated".into(),
            ));
        }
        let legacy_version = read_u16_be(msg, pos);
        pos += 2;

        // Random
        if pos + 32 > msg.len() {
            return Err(TlsError::HandshakeFailure(
                "ServerHello: random truncated".into(),
            ));
        }
        let mut random = [0u8; 32];
        random.copy_from_slice(&msg[pos..pos + 32]);
        pos += 32;

        // Session ID
        if pos >= msg.len() {
            return Err(TlsError::HandshakeFailure(
                "ServerHello: session_id length missing".into(),
            ));
        }
        let sid_len = msg[pos] as usize;
        pos += 1;
        if pos + sid_len > msg.len() {
            return Err(TlsError::HandshakeFailure(
                "ServerHello: session_id truncated".into(),
            ));
        }
        let session_id = msg[pos..pos + sid_len].to_vec();
        pos += sid_len;

        // Cipher suite
        if pos + 2 > msg.len() {
            return Err(TlsError::HandshakeFailure(
                "ServerHello: cipher_suite truncated".into(),
            ));
        }
        let cs = read_u16_be(msg, pos);
        let cipher_suite =
            CipherSuite::from_wire(cs).map_err(|e| TlsError::HandshakeFailure(e.to_string()))?;
        pos += 2;

        // Legacy compression
        if pos >= msg.len() {
            return Err(TlsError::HandshakeFailure(
                "ServerHello: compression truncated".into(),
            ));
        }
        let legacy_compression = msg[pos];
        pos += 1;

        // Extensions
        let mut server_key_share = Vec::new();
        let mut supported_version = None;

        if pos < msg.len() {
            if pos + 2 > msg.len() {
                return Err(TlsError::HandshakeFailure(
                    "ServerHello: ext_len truncated".into(),
                ));
            }
            let ext_len = read_u16_be(msg, pos) as usize;
            pos += 2;

            let ext_end = pos + ext_len;
            while pos < ext_end {
                if pos + 4 > ext_end {
                    break;
                }
                let ext_type = read_u16_be(msg, pos);
                let ext_data_len = read_u16_be(msg, pos + 2) as usize;
                pos += 4;

                if pos + ext_data_len > ext_end {
                    break;
                }
                let ext_data = &msg[pos..pos + ext_data_len];
                pos += ext_data_len;

                match ext_type {
                    51
                        // key_share
                        if ext_data_len >= 4 => {
                            let _group = read_u16_be(ext_data, 0);
                            let ke_len = read_u16_be(ext_data, 2) as usize;
                            if ext_data_len >= 4 + ke_len {
                                server_key_share = ext_data[4..4 + ke_len].to_vec();
                            }
                        }
                    43
                        // supported_versions
                        if ext_data_len >= 2 => {
                            supported_version = Some(read_u16_be(ext_data, 0));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn extension<'a>(client_hello: &'a [u8], extension_type: u16) -> Option<&'a [u8]> {
        let body_len = read_u24_be(client_hello, 1) as usize;
        let body = &client_hello[4..4 + body_len];
        let mut pos = 2 + 32;
        let session_id_len = body[pos] as usize;
        pos += 1 + session_id_len;
        let cipher_suites_len = read_u16_be(body, pos) as usize;
        pos += 2 + cipher_suites_len;
        let compression_methods_len = body[pos] as usize;
        pos += 1 + compression_methods_len;
        let extensions_len = read_u16_be(body, pos) as usize;
        pos += 2;
        let extensions_end = pos + extensions_len;

        while pos + 4 <= extensions_end {
            let current_type = read_u16_be(body, pos);
            let len = read_u16_be(body, pos + 2) as usize;
            pos += 4;
            let data = &body[pos..pos + len];
            pos += len;
            if current_type == extension_type {
                return Some(data);
            }
        }

        None
    }

    #[test]
    fn client_hello_supported_versions_vector_matches_payload() {
        let random = [7u8; 32];
        let key_share = [9u8; 32];
        let client_hello = ClientHelloBuilder::new(random, "example.com")
            .key_share(&key_share, NamedGroup::X25519)
            .build()
            .expect("build client hello");

        let supported_versions =
            extension(&client_hello, 43).expect("supported_versions extension");

        assert_eq!(supported_versions, &[0x02, 0x03, 0x04]);
    }
}
