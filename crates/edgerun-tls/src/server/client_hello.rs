//! ClientHello parsing for TLS 1.3 server handshake.
//!
//! Parses the wire format of a TLS 1.3 ClientHello message, extracting
//! all relevant extensions: SNI, key_share, supported_versions,
//! supported_groups, and signature_algorithms.

use crate::cipher::NamedGroup;
use crate::{Result, TlsError};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::str;
use edgerun_crypto::CipherSuite;
use edgerun_encoding::byteorder::{read_u16_be, read_u24_be};

/// Parsed ClientHello from the wire format (RFC 8446 §4.1.2).
#[derive(Debug)]
pub struct ClientHello {
    /// Legacy version (always 0x0303 for TLS 1.3)
    pub legacy_version: u16,
    /// 32-byte random value
    pub random: [u8; 32],
    /// Session ID (0-32 bytes)
    pub session_id: Vec<u8>,
    /// Cipher suites offered by the client
    pub cipher_suites: Vec<CipherSuite>,
    /// Legacy compression methods (should be [0])
    pub legacy_compression: Vec<u8>,
    /// SNI server name (if present, from ext 0)
    pub server_name: Option<String>,
    /// Client's key_share public key bytes (for the group we support)
    pub client_key_share: Option<Vec<u8>>,
    /// The named group for the client's key_share
    pub client_key_share_group: Option<NamedGroup>,
    /// All client key shares (group -> key bytes)
    pub all_key_shares: Vec<(NamedGroup, Vec<u8>)>,
    /// Supported versions extension (ext 43)
    pub supported_versions: Vec<u16>,
    /// Supported groups (ext 10)
    pub supported_groups: Vec<NamedGroup>,
    /// Signature algorithms (ext 13)
    pub signature_algorithms: Vec<u16>,
    /// ALPN protocols offered by the client (ext 16)
    pub alpn_protocols: Vec<Vec<u8>>,
}

impl ClientHello {
    /// Parse a ClientHello from a handshake message payload (without the 4-byte header).
    ///
    /// The input should be the raw handshake message bytes starting with the
    /// ClientHello type byte (0x01).
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(TlsError::HandshakeFailure("ClientHello too short".into()));
        }
        if data[0] != 1 {
            return Err(TlsError::HandshakeFailure(format!(
                "Expected ClientHello (type 1), got {}",
                data[0]
            )));
        }

        let msg_len = read_u24_be(data, 1) as usize;
        if data.len() < 4 + msg_len {
            return Err(TlsError::HandshakeFailure("ClientHello truncated".into()));
        }

        let msg = &data[4..4 + msg_len];
        let mut pos = 0;

        // Legacy version
        if pos + 2 > msg.len() {
            return Err(TlsError::Protocol(
                "ClientHello: legacy_version truncated".into(),
            ));
        }
        let legacy_version = read_u16_be(msg, pos);
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
            return Err(TlsError::Protocol(
                "ClientHello: session_id length missing".into(),
            ));
        }
        let sid_len = msg[pos] as usize;
        pos += 1;
        if sid_len > 32 {
            return Err(TlsError::Protocol(
                "ClientHello: session_id too long".into(),
            ));
        }
        if pos + sid_len > msg.len() {
            return Err(TlsError::Protocol(
                "ClientHello: session_id truncated".into(),
            ));
        }
        let session_id = msg[pos..pos + sid_len].to_vec();
        pos += sid_len;

        // Cipher suites
        if pos + 2 > msg.len() {
            return Err(TlsError::Protocol(
                "ClientHello: cipher_suites length truncated".into(),
            ));
        }
        let cs_len = read_u16_be(msg, pos) as usize;
        pos += 2;
        if !cs_len.is_multiple_of(2) {
            return Err(TlsError::Protocol(
                "ClientHello: cipher_suites length not even".into(),
            ));
        }
        if pos + cs_len > msg.len() {
            return Err(TlsError::Protocol(
                "ClientHello: cipher_suites truncated".into(),
            ));
        }
        let mut cipher_suites = Vec::new();
        let cs_end = pos + cs_len;
        while pos + 1 < cs_end {
            let cs = read_u16_be(msg, pos);
            if let Ok(suite) = CipherSuite::from_wire(cs) {
                cipher_suites.push(suite);
            }
            pos += 2;
        }
        pos = cs_end;

        // Legacy compression methods
        if pos >= msg.len() {
            return Err(TlsError::Protocol(
                "ClientHello: compression length missing".into(),
            ));
        }
        let comp_len = msg[pos] as usize;
        pos += 1;
        if pos + comp_len > msg.len() {
            return Err(TlsError::Protocol(
                "ClientHello: compression truncated".into(),
            ));
        }
        let legacy_compression = msg[pos..pos + comp_len].to_vec();
        pos += comp_len;

        // Extensions
        let mut server_name = None;
        let mut client_key_share = None;
        let mut client_key_share_group = None;
        let mut all_key_shares = Vec::new();
        let mut supported_versions = Vec::new();
        let mut supported_groups = Vec::new();
        let mut signature_algorithms = Vec::new();
        let mut alpn_protocols = Vec::new();

        if pos < msg.len() {
            if pos + 2 > msg.len() {
                return Err(TlsError::Protocol("ClientHello: ext_len truncated".into()));
            }
            let ext_len = read_u16_be(msg, pos) as usize;
            pos += 2;

            let ext_end = pos + ext_len;
            while pos + 4 <= ext_end {
                let ext_type = read_u16_be(msg, pos);
                let ext_data_len = read_u16_be(msg, pos + 2) as usize;
                pos += 4;

                if pos + ext_data_len > ext_end {
                    break;
                }
                let ext_data = &msg[pos..pos + ext_data_len];
                pos += ext_data_len;

                match ext_type {
                    0
                        // server_name (SNI) - RFC 6066
                        if ext_data_len >= 2 => {
                            let name_list_len = read_u16_be(ext_data, 0) as usize;
                            if ext_data.len() >= 2 + name_list_len && name_list_len >= 3 {
                                let name_type = ext_data[2];
                                if name_type == 0 {
                                    let name_len = read_u16_be(ext_data, 3) as usize;
                                    if 5 + name_len <= 2 + name_list_len {
                                        if let Ok(name) = str::from_utf8(&ext_data[5..5 + name_len]) {
                                            server_name = Some(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    10
                        // supported_groups
                        if ext_data_len >= 2 => {
                            let groups_len = read_u16_be(ext_data, 0) as usize;
                            let mut gpos = 2;
                            while gpos + 1 < groups_len && gpos + 1 < ext_data.len() {
                                let g = read_u16_be(ext_data, gpos);
                                if let Ok(group) = NamedGroup::from_wire(g) {
                                    supported_groups.push(group);
                                }
                                gpos += 2;
                            }
                        }
                    13
                        // signature_algorithms
                        if ext_data_len >= 2 => {
                            let sa_len = read_u16_be(ext_data, 0) as usize;
                            let mut spos = 2;
                            while spos + 1 < sa_len && spos + 1 < ext_data.len() {
                                let sa = read_u16_be(ext_data, spos);
                                signature_algorithms.push(sa);
                                spos += 2;
                            }
                        }
                    43
                        // supported_versions
                        if ext_data_len >= 1 => {
                            let _versions_len = ext_data[0] as usize;
                            let mut vpos = 1;
                            while vpos + 1 < ext_data.len() {
                                let v = read_u16_be(ext_data, vpos);
                                supported_versions.push(v);
                                vpos += 2;
                            }
                        }
                    51
                        // key_share
                        if ext_data_len >= 2 => {
                            let ks_len = read_u16_be(ext_data, 0) as usize;
                            let mut kpos = 2;
                            while kpos + 3 < ks_len && kpos + 3 < ext_data.len() {
                                let group = read_u16_be(ext_data, kpos);
                                let ke_len = read_u16_be(ext_data, kpos + 2) as usize;
                                kpos += 4;
                                if kpos + ke_len <= ext_data.len() {
                                    if let Ok(g) = NamedGroup::from_wire(group) {
                                        all_key_shares.push((g, ext_data[kpos..kpos + ke_len].to_vec()));
                                        if client_key_share.is_none() {
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
                    16
                        // application_layer_protocol_negotiation (ALPN) - RFC 7301
                        if ext_data_len >= 2 => {
                            let proto_list_len = read_u16_be(ext_data, 0) as usize;
                            let mut ppos = 2;
                            while ppos < ext_data.len() && ppos < 2 + proto_list_len {
                                if ppos + 1 > ext_data.len() { break; }
                                let proto_len = ext_data[ppos] as usize;
                                ppos += 1;
                                if ppos + proto_len <= ext_data.len() {
                                    alpn_protocols.push(ext_data[ppos..ppos + proto_len].to_vec());
                                    ppos += proto_len;
                                } else {
                                    break;
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
            all_key_shares,
            supported_versions,
            supported_groups,
            signature_algorithms,
            alpn_protocols,
        })
    }
}

impl NamedGroup {
    /// Parse NamedGroup from wire format.
    pub fn from_wire(value: u16) -> Result<Self> {
        match value {
            0x0017 => Ok(NamedGroup::SECP256R1),
            0x0018 => Ok(NamedGroup::SECP384R1),
            0x001D => Ok(NamedGroup::X25519),
            _ => Err(TlsError::Protocol(format!(
                "Unsupported named group: 0x{:04x}",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handshake::ClientHelloBuilder;
    use crate::key_exchange::{EcdhKeyPair, KeyExchangeGroup};

    #[test]
    fn test_client_hello_too_short() {
        assert!(ClientHello::parse(&[]).is_err());
        assert!(ClientHello::parse(&[0x01]).is_err());
    }

    #[test]
    fn test_client_hello_wrong_type() {
        let data = [0x02, 0x00, 0x00, 0x00];
        assert!(ClientHello::parse(&data).is_err());
    }

    #[test]
    fn test_client_hello_roundtrip() {
        let random = [0x42u8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "example.com")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        assert_eq!(ch_bytes[0], 1);
        let ch = ClientHello::parse(&ch_bytes).unwrap();
        assert_eq!(ch.random, random);
        assert_eq!(ch.server_name, Some("example.com".to_string()));
        assert!(ch.client_key_share.is_some());
        assert_eq!(ch.client_key_share_group, Some(NamedGroup::SECP256R1));
        assert!(
            ch.cipher_suites
                .contains(&CipherSuite::TLS_AES_128_GCM_SHA256)
        );
        assert!(ch.supported_versions.contains(&0x0304));
    }

    #[test]
    fn test_client_hello_parses_from_real_client() {
        let random = [0xABu8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "localhost")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let ch = ClientHello::parse(&ch_bytes).unwrap();
        assert_eq!(ch.random, random);
        assert_eq!(ch.server_name, Some("localhost".to_string()));
        assert!(ch.client_key_share.is_some());
        let ks = ch.client_key_share.unwrap();
        assert_eq!(ks.len(), 65);
        assert_eq!(ks[0], 0x04);
    }

    #[test]
    fn test_client_hello_parses_all_extensions() {
        let random = [0x55u8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
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
    }

    #[test]
    fn test_named_group_wire_roundtrip() {
        for group in [
            NamedGroup::SECP256R1,
            NamedGroup::SECP384R1,
            NamedGroup::X25519,
        ] {
            let wire = group.to_wire();
            let parsed = NamedGroup::from_wire(wire).unwrap();
            assert_eq!(parsed, group);
        }
    }

    #[test]
    fn test_named_group_unsupported() {
        assert!(NamedGroup::from_wire(0x001E).is_err());
        assert!(NamedGroup::from_wire(0xFFFF).is_err());
    }

    #[test]
    fn test_client_hello_sni_with_subdomain() {
        let random = [0x11u8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "api.example.com")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let ch = ClientHello::parse(&ch_bytes).unwrap();
        assert_eq!(ch.server_name, Some("api.example.com".to_string()));
    }

    #[test]
    fn test_client_hello_cipher_suites_parsed() {
        let random = [0x33u8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let public_key = key_pair.public_key_bytes();

        let ch_bytes = ClientHelloBuilder::new(random, "localhost")
            .key_share(&public_key, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let ch = ClientHello::parse(&ch_bytes).unwrap();
        assert!(
            ch.cipher_suites
                .contains(&CipherSuite::TLS_AES_128_GCM_SHA256)
        );
        assert!(
            ch.cipher_suites
                .contains(&CipherSuite::TLS_AES_256_GCM_SHA384)
        );
    }
}
