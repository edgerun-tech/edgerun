//! Unified QUIC-TLS handshake driver (RFC 9001 + RFC 8446).
//!
//! Provides a unified API for both client and server handshake operations,
//! consolidating the shared cryptographic logic from the separate handshakers.
//!
//! # Handshake Flow
//! ```text
//! Client                                          Server
//! ------                                          ------
//! Initial: CRYPTO( ClientHello )                ->
//!                                       <-  Initial: CRYPTO( ServerHello )
//!                                       <-  Handshake: CRYPTO( EE, Cert, CertVerify, Finished )
//! Handshake: CRYPTO( ClientFinished )      ->
//!
//! [1-RTT packets with HTTP/3 data]          <->     [1-RTT packets]
//! ```

use alloc::{string::String, vec, vec::Vec};
use edgerun_crypto::CipherSuite;
use edgerun_tls::cipher::NamedGroup;
use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use edgerun_tls::prf::{
    quic_hp_key, quic_initial_client_keys, quic_initial_server_keys, quic_traffic_keys, Hasher,
    Tls13KeySchedule, TrafficKeys, INITIAL_SALT_V1,
};

use super::crypto::ProtectionKeys;
use super::QuicFrame;
use crate::ConnectionId;

pub use super::handshake::HandshakeResult;
pub use super::handshake::QuicTlsHandshaker;
pub use super::server_handshake::QuicTlsServerHandshaker;
pub use super::server_handshake::ServerHandshakeResult;

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::fill_random;
    use edgerun_tls::handshake::ClientHelloBuilder;
    use edgerun_tls::prf::Hasher;

    const TEST_DCID: [u8; 8] = [0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];

    fn hasher_for_suite(suite: CipherSuite) -> Hasher {
        match suite {
            CipherSuite::TLS_AES_256_GCM_SHA384 => Hasher::Sha384,
            _ => Hasher::Sha256,
        }
    }

    #[test]
    fn test_unified_initial_keys_derivation() {
        let hasher = Hasher::Sha256;

        let client_keys = quic_initial_client_keys(&TEST_DCID, 16, 12, &hasher);
        let server_keys = quic_initial_server_keys(&TEST_DCID, 16, 12, &hasher);

        assert_eq!(client_keys.0.write_key.len(), 16);
        assert_eq!(client_keys.0.write_iv.len(), 12);
        assert_eq!(server_keys.0.write_key.len(), 16);
        assert_eq!(server_keys.0.write_iv.len(), 12);
    }

    #[test]
    fn test_client_server_hello_roundtrip() {
        let mut client_random = [0u8; 32];
        fill_random(&mut client_random).unwrap();

        let client_kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_kp.public_key_bytes(), NamedGroup::X25519)
            .build()
            .unwrap();

        assert_eq!(ch_bytes[0], 1);
        assert!(ch_bytes.len() > 100);

        let mut hs = QuicTlsHandshaker::new("localhost");
        let initial_data = hs.initial_crypto_data();
        assert_eq!(initial_data[0], 1);
        assert!(initial_data.len() > 100);
    }

    #[test]
    fn test_server_processes_client_hello() {
        let mut client_random = [0u8; 32];
        fill_random(&mut client_random).unwrap();

        let client_kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_kp.public_key_bytes(), NamedGroup::X25519)
            .build()
            .unwrap();

        let cert = edgerun_tls::generate_self_signed(&["localhost"]).unwrap();
        let mut server = QuicTlsServerHandshaker::new(cert);

        let sh_bytes = server.process_client_hello(&ch_bytes).unwrap();
        assert_eq!(sh_bytes[0], 2);
        assert!(server.has_client_key_share());
    }

    #[test]
    fn test_client_initial_keys_derivation() {
        let hs = QuicTlsHandshaker::new("localhost");
        let keys = hs.initial_keys(&TEST_DCID);

        assert_eq!(keys.write_key.len(), 16);
        assert_eq!(keys.write_iv.len(), 12);
        assert_eq!(keys.read_key.len(), 16);
        assert_eq!(keys.read_iv.len(), 12);
    }

    #[test]
    fn test_hs_initial_keys_match() {
        let client_hs = QuicTlsHandshaker::new("localhost");
        let cert = edgerun_tls::generate_self_signed(&["localhost"]).unwrap();
        let server_hs = QuicTlsServerHandshaker::new(cert);

        let client_keys = client_hs.initial_keys(&TEST_DCID);
        let server_keys = server_hs.initial_keys(&TEST_DCID);

        assert_eq!(client_keys.write_key.len(), server_keys.read_key.len());
        assert_eq!(client_keys.write_iv.len(), server_keys.read_iv.len());
    }

    #[test]
    fn test_quic_frame_crypto_roundtrip() {
        let data = b"Hello, CRYPTO frame!".to_vec();
        let frame = QuicFrame::Crypto {
            offset: 0,
            data: data.clone(),
        };

        let encoded = frame.to_bytes();
        let (decoded, consumed) = QuicFrame::from_bytes(&encoded).unwrap();

        assert_eq!(consumed, encoded.len());
        if let QuicFrame::Crypto {
            offset,
            data: decoded_data,
        } = decoded
        {
            assert_eq!(offset, 0);
            assert_eq!(decoded_data, data);
        } else {
            panic!("Expected Crypto frame");
        }
    }

    #[test]
    fn test_quic_frame_stream_roundtrip() {
        let data = b"Hello, STREAM frame!".to_vec();
        let frame = QuicFrame::Stream {
            stream_id: 4,
            offset: 100,
            fin: true,
            data,
        };

        let encoded = frame.to_bytes();
        let (decoded, consumed) = QuicFrame::from_bytes(&encoded).unwrap();

        assert_eq!(consumed, encoded.len());
        if let QuicFrame::Stream {
            stream_id,
            offset,
            fin,
            data: decoded_data,
        } = decoded
        {
            assert_eq!(stream_id, 4);
            assert_eq!(offset, 100);
            assert!(fin);
            assert_eq!(decoded_data, b"Hello, STREAM frame!");
        } else {
            panic!("Expected Stream frame");
        }
    }

    #[test]
    fn test_quic_frame_multiple_in_payload() {
        let crypto_frame = QuicFrame::Crypto {
            offset: 0,
            data: b"crypto".to_vec(),
        };
        let stream_frame = QuicFrame::Stream {
            stream_id: 0,
            offset: 0,
            fin: true,
            data: b"stream".to_vec(),
        };

        let combined = [crypto_frame.to_bytes(), stream_frame.to_bytes()].concat();

        let (f1, n1) = QuicFrame::from_bytes(&combined).unwrap();
        let (f2, _) = QuicFrame::from_bytes(&combined[n1..]).unwrap();

        match f1 {
            QuicFrame::Crypto { data, .. } => assert_eq!(data, b"crypto"),
            _ => panic!("Expected Crypto frame first"),
        }
        match f2 {
            QuicFrame::Stream { data, .. } => assert_eq!(data, b"stream"),
            _ => panic!("Expected Stream frame second"),
        }
    }

    #[test]
    fn test_connection_id_random() {
        let cid1 = ConnectionId::random();
        let cid2 = ConnectionId::random();

        assert_eq!(cid1.len(), 8);
        assert_eq!(cid2.len(), 8);
        assert_ne!(cid1.as_bytes(), cid2.as_bytes());
    }

    #[test]
    fn test_connection_id_from_vec() {
        let cid = ConnectionId::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(cid.len(), 8);
        assert_eq!(cid.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8]);

        let empty = ConnectionId::new(vec![]);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_server_hello_exchange() {
        let mut client_random = [0u8; 32];
        fill_random(&mut client_random).unwrap();

        let client_kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_kp.public_key_bytes(), NamedGroup::X25519)
            .build()
            .unwrap();

        let cert = edgerun_tls::generate_self_signed(&["localhost"]).unwrap();
        let mut server = QuicTlsServerHandshaker::new(cert);

        let sh_bytes = server.process_client_hello(&ch_bytes).unwrap();
        assert_eq!(sh_bytes[0], 2);

        let mut client = QuicTlsHandshaker::new("localhost");
        let ch_data = client.initial_crypto_data().to_vec();

        let sh_data = server.process_client_hello(&ch_data).unwrap();
        client.process_initial_crypto(&sh_data).unwrap();
        assert!(client.has_server_hello());
    }
}
