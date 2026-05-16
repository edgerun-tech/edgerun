use super::QuicFrame;
use alloc::vec;

#[test]
fn test_ping_frame() {
    let frame = QuicFrame::Ping;
    let bytes = frame.to_bytes();
    assert_eq!(bytes, vec![0x01]);

    let (parsed, len) = QuicFrame::from_bytes(&bytes).unwrap();
    assert!(matches!(parsed, QuicFrame::Ping));
    assert_eq!(len, 1);
}

#[test]
fn test_padding_frame() {
    let frame = QuicFrame::Padding { length: 10 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes.len(), 10);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn test_stream_frame() {
    let frame = QuicFrame::Stream {
        stream_id: 0,
        offset: 0,
        fin: true,
        data: b"hello".to_vec(),
    };
    let bytes = frame.to_bytes();
    assert!(!bytes.is_empty());

    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::Stream { fin, data, .. } = parsed {
        assert!(fin);
        assert_eq!(data, b"hello");
    } else {
        panic!("Expected Stream frame");
    }
}

#[test]
fn test_crypto_frame() {
    let frame = QuicFrame::Crypto {
        offset: 0,
        data: vec![0x01, 0x02, 0x03],
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x06);
}

#[test]
fn test_crypto_frame_with_tls_handshake_data() {
    // Simulate wrapping TLS ClientHello in QUIC CRYPTO frame
    let tls_client_hello = vec![
        0x01, 0x00, 0x00, 0xac, // handshake type + length
        0x03, 0x03, // TLS version
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // random
        0x00, // session id length
    ];

    let frame = QuicFrame::Crypto {
        offset: 0,
        data: tls_client_hello.clone(),
    };
    let bytes = frame.to_bytes();

    // First byte should be 0x06 (CRYPTO frame type)
    assert_eq!(bytes[0], 0x06);

    // Parse it back
    let (parsed, consumed) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::Crypto { offset, data } = parsed {
        assert_eq!(offset, 0);
        assert_eq!(data, tls_client_hello);
    } else {
        panic!("Expected Crypto frame");
    }
}

#[test]
fn test_crypto_frame_roundtrip() {
    let original_data = vec![0x06, 0x00, 0x40, 0xb0, 0x01, 0x00, 0x00, 0xac, 0x03, 0x03];
    let frame = QuicFrame::Crypto {
        offset: 0,
        data: original_data.clone(),
    };
    let encoded = frame.to_bytes();

    let (parsed, _) = QuicFrame::from_bytes(&encoded).unwrap();
    if let QuicFrame::Crypto { data, .. } = parsed {
        assert_eq!(data, original_data);
    }
}

#[test]
fn test_crypto_exact_encoding() {
    // Build exact bytes we expect: [0x06][offset=0][length=N][TLS data]
    let tls_data = vec![0x01, 0x00, 0x00, 0xac, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00];
    let frame = QuicFrame::Crypto {
        offset: 0,
        data: tls_data.clone(),
    };
    let encoded = frame.to_bytes();

    // First byte should be 0x06 (CRYPTO frame type)
    assert_eq!(
        encoded[0], 0x06,
        "First byte should be 0x06 for CRYPTO frame"
    );

    // Second byte should be varint for offset (0)
    assert_eq!(encoded[1], 0, "Offset should be 0");

    // Third byte should be varint for length
    // For length 10, that's stored as 0x0a
    // Actually in varint, values < 64 are single byte
    // So encoded[2] should be the length
    assert_eq!(encoded[2] as usize, tls_data.len(), "Length should match");

    // Verify roundtrip
    let (parsed, _) = QuicFrame::from_bytes(&encoded).unwrap();
    if let QuicFrame::Crypto { offset, data } = parsed {
        assert_eq!(offset, 0);
        assert_eq!(data, tls_data);
    }
}

#[test]
fn test_max_data_frame() {
    let frame = QuicFrame::MaxData { max_data: 65535 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x10);
}

#[test]
fn test_ack_frame_roundtrip() {
    let frame = QuicFrame::Ack {
        largest_acknowledged: 100,
        ack_delay: 50,
        ack_range_count: 1,
        first_ack_range: 10,
        ack_ranges: vec![(5, 20)],
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x02);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::Ack {
        largest_acknowledged,
        ..
    } = parsed
    {
        assert_eq!(largest_acknowledged, 100);
    } else {
        panic!("Expected ACK frame");
    }
}

#[test]
fn test_reset_stream_frame_roundtrip() {
    let frame = QuicFrame::ResetStream {
        stream_id: 4,
        error_code: 0,
        final_size: 1024,
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x04);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::ResetStream {
        stream_id,
        final_size,
        ..
    } = parsed
    {
        assert_eq!(stream_id, 4);
        assert_eq!(final_size, 1024);
    } else {
        panic!("Expected ResetStream frame");
    }
}

#[test]
fn test_stop_sending_frame_roundtrip() {
    let frame = QuicFrame::StopSending {
        stream_id: 8,
        error_code: 42,
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x05);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::StopSending {
        stream_id,
        error_code,
    } = parsed
    {
        assert_eq!(stream_id, 8);
        assert_eq!(error_code, 42);
    } else {
        panic!("Expected StopSending frame");
    }
}

#[test]
fn test_new_token_frame_roundtrip() {
    let frame = QuicFrame::NewToken {
        token: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x07);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::NewToken { token } = parsed {
        assert_eq!(token, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    } else {
        panic!("Expected NewToken frame");
    }
}

#[test]
fn test_max_stream_data_roundtrip() {
    let frame = QuicFrame::MaxStreamData {
        stream_id: 4,
        max_stream_data: 8192,
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x11);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::MaxStreamData {
        stream_id,
        max_stream_data,
    } = parsed
    {
        assert_eq!(stream_id, 4);
        assert_eq!(max_stream_data, 8192);
    } else {
        panic!("Expected MaxStreamData frame");
    }
}

#[test]
fn test_max_streams_bidi_roundtrip() {
    let frame = QuicFrame::MaxStreamsBidi { max_streams: 200 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x12);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::MaxStreamsBidi { max_streams } = parsed {
        assert_eq!(max_streams, 200);
    } else {
        panic!("Expected MaxStreamsBidi frame");
    }
}

#[test]
fn test_max_streams_uni_roundtrip() {
    let frame = QuicFrame::MaxStreamsUni { max_streams: 100 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x13);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::MaxStreamsUni { max_streams } = parsed {
        assert_eq!(max_streams, 100);
    } else {
        panic!("Expected MaxStreamsUni frame");
    }
}

#[test]
fn test_data_blocked_roundtrip() {
    let frame = QuicFrame::DataBlocked { max_data: 16384 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x14);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::DataBlocked { max_data } = parsed {
        assert_eq!(max_data, 16384);
    } else {
        panic!("Expected DataBlocked frame");
    }
}

#[test]
fn test_stream_data_blocked_roundtrip() {
    let frame = QuicFrame::StreamDataBlocked {
        stream_id: 4,
        max_stream_data: 32768,
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x15);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::StreamDataBlocked {
        stream_id,
        max_stream_data,
    } = parsed
    {
        assert_eq!(stream_id, 4);
        assert_eq!(max_stream_data, 32768);
    } else {
        panic!("Expected StreamDataBlocked frame");
    }
}

#[test]
fn test_streams_blocked_bidi_roundtrip() {
    let frame = QuicFrame::StreamsBlockedBidi { max_streams: 50 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x16);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::StreamsBlockedBidi { max_streams } = parsed {
        assert_eq!(max_streams, 50);
    } else {
        panic!("Expected StreamsBlockedBidi frame");
    }
}

#[test]
fn test_streams_blocked_uni_roundtrip() {
    let frame = QuicFrame::StreamsBlockedUni { max_streams: 25 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x17);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::StreamsBlockedUni { max_streams } = parsed {
        assert_eq!(max_streams, 25);
    } else {
        panic!("Expected StreamsBlockedUni frame");
    }
}

#[test]
fn test_new_connection_id_roundtrip() {
    let frame = QuicFrame::NewConnectionId {
        sequence_number: 1,
        retire_prior_to: 0,
        connection_id: vec![1, 2, 3, 4, 5, 6, 7, 8],
        stateless_reset_token: [0xAA; 16],
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x18);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::NewConnectionId {
        sequence_number,
        retire_prior_to,
        connection_id,
        stateless_reset_token,
    } = parsed
    {
        assert_eq!(sequence_number, 1);
        assert_eq!(retire_prior_to, 0);
        assert_eq!(connection_id, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(stateless_reset_token, [0xAA; 16]);
    } else {
        panic!("Expected NewConnectionId frame");
    }
}

#[test]
fn test_retire_connection_id_roundtrip() {
    let frame = QuicFrame::RetireConnectionId { sequence_number: 3 };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x19);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::RetireConnectionId { sequence_number } = parsed {
        assert_eq!(sequence_number, 3);
    } else {
        panic!("Expected RetireConnectionId frame");
    }
}

#[test]
fn test_path_challenge_roundtrip() {
    let frame = QuicFrame::PathChallenge {
        data: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x1A);
    assert_eq!(bytes.len(), 9);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::PathChallenge { data } = parsed {
        assert_eq!(data, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
    } else {
        panic!("Expected PathChallenge frame");
    }
}

#[test]
fn test_path_response_roundtrip() {
    let frame = QuicFrame::PathResponse {
        data: [0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88],
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x1B);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::PathResponse { data } = parsed {
        assert_eq!(data, [0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88]);
    } else {
        panic!("Expected PathResponse frame");
    }
}

#[test]
fn test_connection_close_application_roundtrip() {
    let frame = QuicFrame::ConnectionCloseApplication {
        error_code: 0x0100,
        reason: b"application shutdown".to_vec(),
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x1D);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::ConnectionCloseApplication { error_code, reason } = parsed {
        assert_eq!(error_code, 0x0100);
        assert_eq!(reason, b"application shutdown");
    } else {
        panic!("Expected ConnectionCloseApplication frame");
    }
}

#[test]
fn test_ack_ecn_frame_roundtrip() {
    let frame = QuicFrame::AckECN {
        largest_acknowledged: 50,
        ack_delay: 10,
        ack_range_count: 0,
        first_ack_range: 5,
        ack_ranges: vec![],
        ect0_count: 30,
        ect1_count: 5,
        ce_count: 0,
    };
    let bytes = frame.to_bytes();
    assert_eq!(bytes[0], 0x03);
    let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
    if let QuicFrame::AckECN {
        ect0_count,
        ect1_count,
        ce_count,
        ..
    } = parsed
    {
        assert_eq!(ect0_count, 30);
        assert_eq!(ect1_count, 5);
        assert_eq!(ce_count, 0);
    } else {
        panic!("Expected AckECN frame");
    }
}

#[test]
fn test_stream_frame_rejects_truncated_declared_data() {
    let bytes = vec![0x0e, 0x00, 0x00, 0x05, b'h', b'i'];
    assert!(QuicFrame::from_bytes(&bytes).is_err());
}

#[test]
fn test_ack_frame_rejects_truncated_ranges() {
    let bytes = vec![
        0x02, // ACK
        0x0a, // largest_acknowledged
        0x00, // ack_delay
        0x01, // ack_range_count
        0x00, // first_ack_range
        0x00, // gap, missing additional range
    ];
    assert!(QuicFrame::from_bytes(&bytes).is_err());
}

#[test]
fn test_ack_ecn_frame_rejects_truncated_ranges() {
    let bytes = vec![
        0x03, // ACK_ECN
        0x0a, // largest_acknowledged
        0x00, // ack_delay
        0x01, // ack_range_count
        0x00, // first_ack_range
        0x00, // gap, missing additional range and ECN counts
    ];
    assert!(QuicFrame::from_bytes(&bytes).is_err());
}

#[test]
fn test_new_token_rejects_truncated_token() {
    let bytes = vec![0x07, 0x04, 0xde, 0xad];
    assert!(QuicFrame::from_bytes(&bytes).is_err());
}

#[test]
fn test_connection_close_rejects_truncated_reason() {
    let bytes = vec![0x1c, 0x00, 0x01, 0x04, b'n', b'o'];
    assert!(QuicFrame::from_bytes(&bytes).is_err());
}

#[test]
fn test_application_close_rejects_truncated_reason() {
    let bytes = vec![0x1d, 0x00, 0x04, b'n', b'o'];
    assert!(QuicFrame::from_bytes(&bytes).is_err());
}
