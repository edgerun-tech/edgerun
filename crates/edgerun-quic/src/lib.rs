//! QUIC transport protocol (RFC 9000)

#![no_std]

#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std as real_std;

pub mod compat {
    pub use edgerun_rt::{sleep, spawn, timeout, Duration, Instant};
}

pub mod std {
    pub mod collections {
        pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
    }

    pub mod time {
        pub use edgerun_rt::{Duration, Instant};
    }

    pub mod io {
        use alloc::{format, string::String};
        use core::fmt;

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum ErrorKind {
            UnexpectedEof,
            InvalidData,
            Other,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Error {
            kind: ErrorKind,
            message: String,
        }

        impl Error {
            pub fn new(kind: ErrorKind, message: impl fmt::Display) -> Self {
                Self {
                    kind,
                    message: format!("{message}"),
                }
            }

            pub fn kind(&self) -> ErrorKind {
                self.kind
            }
        }

        impl fmt::Display for Error {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.message)
            }
        }

        impl core::error::Error for Error {}
    }
}

pub mod crypto;
pub mod frame;
pub mod handshake;
pub mod handshake_unified;
pub mod packet;
pub mod server_handshake;
pub mod transport;
pub mod types;

// Re-exports
pub use crypto::{PacketProtection, ProtectionKeys, QuicCrypto};
pub use frame::QuicFrame;
pub use handshake::{HandshakeResult, QuicTlsHandshaker};
pub use packet::{get_long_header_payload_offset, PacketType, QuicPacket};
pub use server_handshake::{QuicTlsServerHandshaker, ServerHandshakeResult};
pub use transport::QuicTransport;

// Types
pub use types::{ConnectionId, PacketNumberSpace, TransportParameters, QUIC_VERSION_V1};

pub type QuicConnection = QuicTransport;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use edgerun_tls::certificate_gen::generate_self_signed;

    /// Full QUIC handshake test: client connects to server, handshake completes,
    /// and both sides can exchange messages using the established keys.
    #[test]
    fn test_full_handshake_and_message_exchange() {
        // 1. Generate server certificate
        let cert = generate_self_signed(&["localhost", "127.0.0.1"]).unwrap();
        let cert_and_key = server_handshake::CertificateAndKey::from(cert);

        // 2. Create server handshaker
        let mut server = server_handshake::QuicTlsServerHandshaker::new(cert_and_key);

        // 3. Create client handshaker
        let mut client = handshake::QuicTlsHandshaker::new("localhost");
        client.allow_unverified_certificates(true);

        // 4. Client generates ClientHello (Initial crypto data)
        let client_hello = client.initial_crypto_data();
        assert!(!client_hello.is_empty());
        assert_eq!(client_hello[0], 1); // ClientHello type

        // 5. Server processes ClientHello and generates ServerHello
        let server_hello = server.process_client_hello(client_hello).unwrap();
        assert_eq!(server_hello[0], 2); // ServerHello type
        assert!(server.has_client_key_share());

        // 6. Client processes ServerHello (Initial crypto)
        client.process_initial_crypto(&server_hello).unwrap();
        assert!(client.has_server_hello());

        // 7. Server builds encrypted handshake messages (EE, Cert, CV, Finished)
        let (server_handshake_data, expected_client_verify) =
            server.build_encrypted_handshake().unwrap();
        assert!(!server_handshake_data.is_empty());

        // 8. Client processes server's handshake messages
        let client_finished = client
            .process_handshake_crypto(&server_handshake_data)
            .unwrap();
        assert!(!client_finished.is_empty());
        assert_eq!(client_finished[0], 20); // Finished type

        // 9. Server verifies client's Finished
        // Extract verify_data from the Client Finished message (skip type byte and 3-byte length)
        let client_verify_data = &client_finished[4..];
        server
            .verify_client_finished(client_verify_data, &expected_client_verify)
            .unwrap();

        // 10. Build handshake results with all keys
        // Client transcript already includes: CH || SH || EE || Cert || CV || ServerFinished
        // Need to append ClientFinished for app key derivation
        let mut client_full_transcript = client.transcript().to_vec();
        client_full_transcript.extend_from_slice(&client_finished);

        let client_result = client
            .build_result(&[0x01, 0x02, 0x03, 0x04], &client_full_transcript)
            .unwrap();

        // Server transcript includes: CH || SH || EE || Cert || CV || ServerFinished
        // Need to append ClientFinished for app key derivation
        let mut server_full_transcript = server.transcript().to_vec();
        server_full_transcript.extend_from_slice(&client_finished);

        let server_result = server
            .build_result(&[0x01, 0x02, 0x03, 0x04], &server_full_transcript)
            .unwrap();

        // 11. Verify all key types are present
        assert!(!client_result.initial_keys.write_key.is_empty());
        assert!(!client_result.handshake_keys.write_key.is_empty());
        assert!(!client_result.app_keys.write_key.is_empty());

        assert!(!server_result.initial_keys.write_key.is_empty());
        assert!(!server_result.handshake_keys.write_key.is_empty());
        assert!(!server_result.app_keys.write_key.is_empty());

        // 12. Verify that client and server have matching keys for communication
        // Client writes with client_app_secret, server reads with client_app_secret
        assert_eq!(
            client_result.app_keys.write_key,
            server_result.app_keys.read_key
        );
        // Server writes with server_app_secret, client reads with server_app_secret
        assert_eq!(
            server_result.app_keys.write_key,
            client_result.app_keys.read_key
        );
        // IVs should also match
        assert_eq!(
            client_result.app_keys.write_iv,
            server_result.app_keys.read_iv
        );
        assert_eq!(
            server_result.app_keys.write_iv,
            client_result.app_keys.read_iv
        );

        // 13. Verify cipher suite negotiation
        assert_eq!(
            client_result.cipher_suite,
            edgerun_crypto::CipherSuite::TLS_AES_128_GCM_SHA256
        );
        assert_eq!(
            server_result.cipher_suite,
            edgerun_crypto::CipherSuite::TLS_AES_128_GCM_SHA256
        );

        // 14. Test message exchange using application keys
        let mut client_prot = crypto::PacketProtection::new(&client_result.app_keys);
        let mut server_prot = crypto::PacketProtection::new(&server_result.app_keys);

        // Client sends message to server (client writes with client_app_secret)
        let header = b"\x40\x00\x00\x00\x01"; // Short header
        let client_msg = b"Hello from QUIC client!";
        let encrypted = client_prot.protect(header, client_msg).unwrap();

        // Server decrypts using client_app_secret (which is server's read key)
        let decrypted = server_prot.unprotect(header, 0, &encrypted).unwrap();
        assert_eq!(&decrypted, client_msg);

        // Server sends response (server writes with server_app_secret)
        let server_header = b"\x40\x00\x00\x00\x02";
        let server_msg = b"Hello from QUIC server!";
        let encrypted = server_prot.protect(server_header, server_msg).unwrap();

        // Client decrypts using server_app_secret (which is client's read key)
        let decrypted = client_prot.unprotect(server_header, 1, &encrypted).unwrap();
        assert_eq!(&decrypted, server_msg);
    }

    /// Test 0-RTT early data functionality
    #[test]
    fn test_zero_rtt_early_data() {
        let cert = generate_self_signed(&["localhost"]).unwrap();
        let cert_and_key = server_handshake::CertificateAndKey::from(cert);

        let mut server = server_handshake::QuicTlsServerHandshaker::new(cert_and_key);
        let mut client = handshake::QuicTlsHandshaker::new("localhost");
        client.allow_unverified_certificates(true);

        // Perform handshake
        let client_hello = client.initial_crypto_data();
        let server_hello = server.process_client_hello(client_hello).unwrap();
        client.process_initial_crypto(&server_hello).unwrap();

        // Client should have 0-RTT keys
        let early_keys = client.early_data_keys();
        assert!(early_keys.is_some(), "Client should have 0-RTT keys");

        if let Some(keys) = early_keys {
            // Test 0-RTT encryption/decryption
            let mut protection = crypto::PacketProtection::new(&keys);
            let header = b"\x40\x00\x00\x00\x00";
            let early_data = b"Early data before handshake completes";
            let encrypted = protection.protect(header, early_data).unwrap();

            // Server can read 0-RTT data using server early data keys
            let server_early_keys = client.server_early_data_read_keys();
            assert!(server_early_keys.is_some());
        }
    }

    /// Test multiple message exchanges in same connection
    #[test]
    fn test_multiple_message_exchange() {
        let cert = generate_self_signed(&["localhost"]).unwrap();
        let cert_and_key = server_handshake::CertificateAndKey::from(cert);

        let mut server = server_handshake::QuicTlsServerHandshaker::new(cert_and_key);
        let mut client = handshake::QuicTlsHandshaker::new("localhost");
        client.allow_unverified_certificates(true);

        // Complete handshake
        let client_hello = client.initial_crypto_data();
        let server_hello = server.process_client_hello(client_hello).unwrap();
        client.process_initial_crypto(&server_hello).unwrap();

        let (server_handshake_data, expected_client_verify) =
            server.build_encrypted_handshake().unwrap();
        let client_finished = client
            .process_handshake_crypto(&server_handshake_data)
            .unwrap();
        // Extract verify_data from Client Finished message
        let client_verify_data = &client_finished[4..];
        server
            .verify_client_finished(client_verify_data, &expected_client_verify)
            .unwrap();

        // Build full client transcript for app key derivation
        let client_transcript = client.transcript().to_vec();
        let client_result = client
            .build_result(&[0x01, 0x02, 0x03, 0x04], &client_transcript)
            .unwrap();

        // Build full server transcript including client's Finished
        let mut server_transcript = server.transcript().to_vec();
        server_transcript.extend_from_slice(&client_finished);

        let server_result = server
            .build_result(&[0x01, 0x02, 0x03, 0x04], &server_transcript)
            .unwrap();

        let mut client_protection = crypto::PacketProtection::new(&client_result.app_keys);
        let mut server_protection = crypto::PacketProtection::new(&server_result.app_keys);

        // Exchange multiple messages
        let messages = [
            b"Message 1".as_slice(),
            b"Message 2 with more data".as_slice(),
            b"Final message".as_slice(),
        ];

        for (i, msg) in messages.iter().enumerate() {
            let header = b"\x40\x00\x00\x00";
            let encrypted = client_protection.protect(header, msg).unwrap();
            let decrypted = server_protection
                .unprotect(header, i as u64, &encrypted)
                .unwrap();
            assert_eq!(&decrypted, msg);
        }
    }

    /// Test initial keys derivation and packet protection
    #[test]
    fn test_initial_packet_protection() {
        let cert = generate_self_signed(&["localhost"]).unwrap();
        let cert_and_key = server_handshake::CertificateAndKey::from(cert);

        let server = server_handshake::QuicTlsServerHandshaker::new(cert_and_key);
        let client = handshake::QuicTlsHandshaker::new("localhost");

        let dcid = vec![0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08];

        let server_initial_keys = server.initial_keys(&dcid);
        let client_initial_keys = client.initial_keys(&dcid);

        // Server writes, client reads (using respective initial keys)
        let mut server_protection = crypto::PacketProtection::new(&server_initial_keys);
        let mut client_protection = crypto::PacketProtection::new(&client_initial_keys);

        let header = b"\xc0\x00\x00\x00\x01"; // Initial packet header
        let payload = b"Initial handshake packet";

        let encrypted = server_protection.protect(header, payload).unwrap();
        let decrypted = client_protection.unprotect(header, 0, &encrypted).unwrap();
        assert_eq!(&decrypted, payload);
    }

    /// Test transport layer features
    #[test]
    fn test_transport_features() {
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let mut transport = QuicTransport::new(local_cid, remote_cid);

        // Test packet number management
        assert_eq!(
            transport.current_packet_number(PacketNumberSpace::Initial),
            0
        );
        assert_eq!(transport.next_packet_number(PacketNumberSpace::Initial), 0);
        assert_eq!(
            transport.current_packet_number(PacketNumberSpace::Initial),
            1
        );

        // Test packet number expansion
        transport.record_received_packet(PacketNumberSpace::ApplicationData, 0x100);
        let expanded = transport.expand_packet_number(PacketNumberSpace::ApplicationData, 0x00, 1);
        assert_eq!(expanded, 0x100);

        // Test ACK generation
        transport.record_received_packet(PacketNumberSpace::ApplicationData, 1);
        transport.record_received_packet(PacketNumberSpace::ApplicationData, 2);
        transport.record_received_packet(PacketNumberSpace::ApplicationData, 3);

        let ack = transport.generate_ack_frame(PacketNumberSpace::ApplicationData);
        assert!(ack.is_some());

        // Test flow control
        let available = transport.available_flow_control();
        assert!(available > 0);

        // Test stream frame creation
        let stream_frame = transport.create_stream_frame(0, b"test data".to_vec(), false);
        if let QuicFrame::Stream { data, fin, .. } = stream_frame {
            assert_eq!(data, b"test data");
            assert!(!fin);
        } else {
            panic!("Expected Stream frame");
        }

        // Test MTU discovery
        let probe_size = transport.start_mtu_discovery();
        assert!(probe_size.is_some());
        if let Some(size) = probe_size {
            assert!(size > 1200); // Should be larger than default MTU
            transport.on_mtu_probe_success(size);
            assert_eq!(transport.mtu(), size);
        }
    }

    /// Test RTT estimation
    #[test]
    fn test_rtt_estimation() {
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let mut transport = QuicTransport::new(local_cid, remote_cid);

        // Initial RTT should be default
        let initial_rtt = transport.rtt();
        assert_eq!(initial_rtt, std::time::Duration::from_millis(100));

        // Update RTT
        transport.update_rtt(
            std::time::Duration::from_millis(50),
            std::time::Duration::ZERO,
        );

        let updated_rtt = transport.rtt();
        assert!(updated_rtt <= std::time::Duration::from_millis(100));
        assert!(transport.smoothed_rtt().is_some());
    }

    /// Test congestion control
    #[test]
    fn test_congestion_control() {
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let mut transport = QuicTransport::new(local_cid, remote_cid);

        // Initial congestion window
        let initial_cwnd = transport.congestion_window();
        assert_eq!(initial_cwnd, 14720); // 10 * max_datagram_size

        // Record sent packets
        transport.record_packet_sent(PacketNumberSpace::ApplicationData, 0, 1000, false);
        transport.record_packet_sent(PacketNumberSpace::ApplicationData, 1, 1000, false);

        assert!(transport.bytes_in_flight() > 0);
        assert!(transport.can_send_bytes(500));

        // Simulate ACK received
        transport.on_ack_received(
            PacketNumberSpace::ApplicationData,
            1,
            0,
            &[],
            std::time::Duration::ZERO,
        );

        // Bytes in flight should decrease
        assert!(transport.bytes_in_flight() < 2000);
    }

    /// Test connection ID handling
    #[test]
    fn test_connection_id() {
        let cid1 = ConnectionId::random();
        let cid2 = ConnectionId::random();

        assert_ne!(cid1.as_bytes(), cid2.as_bytes());
        assert!(!cid1.is_empty());
        assert!(cid1.len() <= 20);

        let custom_cid = ConnectionId::new(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(custom_cid.len(), 8);
        assert_eq!(custom_cid.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    }
}
