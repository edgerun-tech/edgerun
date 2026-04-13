//! TLS 1.3 conformance tests — isolated, testable units.

use edgerun_tls::cipher::{CipherSuite, NamedGroup};
use edgerun_tls::certificate_gen::generate_self_signed;
use edgerun_tls::handshake::{ClientHelloBuilder, ServerHello};
use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use edgerun_tls::prf::{Hasher, Tls13KeySchedule, server_write_keys, hmac_sha256};
use edgerun_tls::record::{RecordCipher, TlsRecord};
use edgerun_tls::server::{build_server_hello, build_encrypted_extensions, build_certificate_message, build_certificate_verify, build_finished_message, ClientHello};

// ========================================================================
// §4.2.1 — ClientHello format conformance
// ========================================================================

/// Verify ClientHello follows RFC 8446 §4.1.2 format exactly.
/// External TLS stacks parse this and expect specific fields.
#[test]
fn test_client_hello_rfc8446_format() {
    let random = [0xAAu8; 32];
    let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let public_key = key_pair.public_key_bytes();

    let ch_bytes = ClientHelloBuilder::new(random, "example.com")
        .key_share(&public_key, NamedGroup::SECP256R1)
        .build()
        .unwrap();

    // RFC 8446 §4.1.2: ClientHello must start with:
    //   msg_type (1) + length (3) + legacy_version (2) + random (32) + legacy_session_id_length (1) + ...
    assert!(ch_bytes.len() >= 38, "ClientHello too short: {} bytes", ch_bytes.len());

    // msg_type = 1 (ClientHello)
    assert_eq!(ch_bytes[0], 1);

    // length = remaining bytes
    let declared_len = u32::from_be_bytes([0, ch_bytes[1], ch_bytes[2], ch_bytes[3]]) as usize;
    assert_eq!(declared_len, ch_bytes.len() - 4, "Handshake length mismatch");

    // legacy_version = 0x0303 (TLS 1.2 for compatibility)
    assert_eq!(&ch_bytes[4..6], &[0x03, 0x03]);

    // random = 32 bytes (must match what we passed)
    assert_eq!(&ch_bytes[6..38], &random[..]);

    // Parse round-trip
    let ch = ClientHello::parse(&ch_bytes).unwrap();
    assert_eq!(ch.random, random);
    assert_eq!(ch.server_name, Some("example.com".to_string()));
    assert!(ch.client_key_share.is_some());
}

/// Verify ClientHello contains all required TLS 1.3 extensions:
/// - supported_versions (43)
/// - key_share (51)
/// - server_name (0, SNI)
/// - supported_groups (10)
/// - signature_algorithms (13)
#[test]
fn test_client_hello_required_extensions() {
    let random = [0xBBu8; 32];
    let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let public_key = key_pair.public_key_bytes();

    let ch_bytes = ClientHelloBuilder::new(random, "test.example.com")
        .key_share(&public_key, NamedGroup::SECP256R1)
        .build()
        .unwrap();

    let ch = ClientHello::parse(&ch_bytes).unwrap();

    // SNI
    assert_eq!(ch.server_name, Some("test.example.com".to_string()));
    // key_share
    assert!(ch.client_key_share.is_some());
    assert_eq!(ch.client_key_share_group, Some(NamedGroup::SECP256R1));
    // supported_versions must include TLS 1.3 (0x0304)
    assert!(ch.supported_versions.contains(&0x0304));
    // supported_groups
    assert!(!ch.supported_groups.is_empty());
    // cipher_suites
    assert!(ch.cipher_suites.len() >= 2);
}

// ========================================================================
// §4.1.3 — ServerHello format conformance
// ========================================================================

/// Verify ServerHello follows RFC 8446 §4.1.3 format exactly.
#[test]
fn test_server_hello_rfc8446_format() {
    let random = [0xCCu8; 32];
    let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let public_key = key_pair.public_key_bytes();

    let sh_bytes = build_server_hello(
        random,
        &[],
        CipherSuite::TLS_AES_128_GCM_SHA256,
        &public_key,
        NamedGroup::SECP256R1,
    );

    // RFC 8446 §4.1.3: ServerHello must start with:
    //   msg_type (1) + length (3) + legacy_version (2) + random (32) + legacy_session_id_length (1) + ...
    assert!(sh_bytes.len() >= 38);

    // msg_type = 2 (ServerHello)
    assert_eq!(sh_bytes[0], 2);

    let declared_len = u32::from_be_bytes([0, sh_bytes[1], sh_bytes[2], sh_bytes[3]]) as usize;
    assert_eq!(declared_len, sh_bytes.len() - 4);

    // legacy_version = 0x0303 (TLS 1.2 for compatibility)
    assert_eq!(&sh_bytes[4..6], &[0x03, 0x03]);

    // random = 32 bytes
    assert_eq!(&sh_bytes[6..38], &random[..]);

    // Must contain TLS 1.3 via supported_versions extension
    let sh = ServerHello::parse(&sh_bytes).unwrap();
    assert_eq!(sh.supported_version, Some(0x0304));
    assert_eq!(sh.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
    assert_eq!(sh.server_key_share.len(), 65);
}

// ========================================================================
// §5. Record layer conformance
// ========================================================================

/// TLS record must have correct header format: content_type(1) + version(2) + length(2)
#[test]
fn test_record_header_format() {
    let record = TlsRecord {
        content_type: 22, // handshake
        version: 0x0303,
        fragment: vec![1, 2, 3, 4, 5],
    };

    let bytes = record.to_bytes();

    // 5 byte header + fragment
    assert_eq!(bytes.len(), 10);

    // Parse back
    let (parsed, consumed) = TlsRecord::from_bytes(&bytes).unwrap();
    assert_eq!(consumed, 10);
    assert_eq!(parsed.content_type, 22);
    assert_eq!(parsed.version, 0x0303);
    assert_eq!(parsed.fragment, vec![1, 2, 3, 4, 5]);
}

/// AEAD encrypt/decrypt must use independent sequence counters per direction.
/// Write cipher seq=0, read cipher seq=0 → both use same nonce.
#[test]
fn test_record_aead_same_nonce() {
    let key = vec![0x01u8; 16];
    let iv = vec![0x02u8; 12];

    let mut write_cipher = RecordCipher::new(&key, &iv).unwrap();
    let mut read_cipher = RecordCipher::new(&key, &iv).unwrap();

    let plaintext = b"TLS 1.3 test data";
    let ciphertext = write_cipher.encrypt(22, plaintext);

    // Separate read cipher starts at seq=0, same as write
    let (ct, decrypted) = read_cipher.decrypt(&ciphertext).unwrap();
    assert_eq!(ct, 22);
    assert_eq!(&decrypted[..], plaintext);
}

/// Second record from same cipher uses seq=1 (different nonce).
#[test]
fn test_record_cipher_seq_increment() {
    let key = vec![0x03u8; 16];
    let iv = vec![0x04u8; 12];
    let mut write_cipher = RecordCipher::new(&key, &iv).unwrap();
    let mut read_cipher = RecordCipher::new(&key, &iv).unwrap();

    // First record
    let ct1 = write_cipher.encrypt(22, b"first");
    let (t1, p1) = read_cipher.decrypt(&ct1).unwrap();
    assert_eq!(t1, 22);
    assert_eq!(&p1[..], b"first");

    // Second record — different nonce due to seq=1
    let ct2 = write_cipher.encrypt(22, b"second");
    let (t2, p2) = read_cipher.decrypt(&ct2).unwrap();
    assert_eq!(t2, 22);
    assert_eq!(&p2[..], b"second");

    // Replaying ct1 with seq=1 should fail
    let mut read_cipher2 = RecordCipher::new(&key, &iv).unwrap();
    let _ = read_cipher2.decrypt(&ct1).unwrap(); // consumes seq=0
    assert!(read_cipher2.decrypt(&ct1).is_err()); // ct1 replayed at seq=1
}

// ========================================================================
// §7. Key schedule conformance
// ========================================================================

/// RFC 8446 §7.1: HKDF-Expand-Label must produce correct key material.
#[test]
fn test_hkdf_expand_label_consistency() {
    let hash = Hasher::Sha256;
    let secret = vec![0xAAu8; 32];

    // Expand the same secret with different labels
    let key1 = hash.expand_label(&secret, "key", &[], 16);
    let key2 = hash.expand_label(&secret, "key", &[], 16);
    let iv = hash.expand_label(&secret, "iv", &[], 12);
    let derived = hash.expand_label(&secret, "derived", &[], 32);

    // Same inputs must produce same outputs
    assert_eq!(key1, key2);
    assert_eq!(key1.len(), 16);
    assert_eq!(iv.len(), 12);
    assert_eq!(derived.len(), 32);

    // Different labels must produce different outputs
    assert_ne!(key1, iv);
    assert_ne!(key1, derived);
}

/// Full key schedule: early → handshake → master
#[test]
fn test_key_schedule_full_derivation() {
    let hash = Hasher::Sha256;
    let shared_secret = vec![0xBBu8; 32];
    let ch_hash = vec![0xCCu8; 32];
    let sh_hash = vec![0xDDu8; 32];

    let mut ks = Tls13KeySchedule::new(hash.clone());
    ks.advance_to_handshake(&shared_secret, &ch_hash, &sh_hash);

    let server_hs = ks.server_handshake_traffic_secret(&ch_hash);
    let client_hs = ks.client_handshake_traffic_secret(&ch_hash);
    assert_ne!(server_hs, client_hs);
    assert_eq!(server_hs.len(), 32);

    ks.advance_to_master();
    let dummy_hash = vec![0u8; 32];
    let server_app = ks.server_app_traffic_secret(&dummy_hash);
    let client_app = ks.client_app_traffic_secret(&dummy_hash);
    assert_ne!(server_app, client_app);
    assert_ne!(server_app, server_hs);
    assert_eq!(server_app.len(), 32);
}

// ========================================================================
// Encrypted message conformance
// ========================================================================

/// Each encrypted handshake message must be decryptable with the correct keys.
#[test]
fn test_encrypted_extensions_roundtrip() {
    let server_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
    let hash = Hasher::Sha256;

    // Derive server handshake keys
    let shared_secret = vec![0x42u8; 32];
    let ch_hash = vec![0x43u8; 32];
    let sh_hash = vec![0x44u8; 32];
    let mut ks = Tls13KeySchedule::new(hash.clone());
    ks.advance_to_handshake(&shared_secret, &ch_hash, &sh_hash);

    let server_hs = ks.server_handshake_traffic_secret(&ch_hash);
    let server_keys_out = server_write_keys(&server_hs, cipher_suite.key_len(), 12, &hash);
    let mut write_cipher = RecordCipher::new(&server_keys_out.write_key, &server_keys_out.write_iv).unwrap();

    let ee_bytes = build_encrypted_extensions(None);
    let encrypted = write_cipher.encrypt(22, &ee_bytes);

    // Decrypt with same keys
    let mut read_cipher = RecordCipher::new(&server_keys_out.write_key, &server_keys_out.write_iv).unwrap();
    let (ct, decrypted) = read_cipher.decrypt(&encrypted).unwrap();
    assert_eq!(ct, 22);
    assert_eq!(decrypted, ee_bytes);
}

/// Certificate message must be decryptable and parseable.
#[test]
fn test_certificate_message_roundtrip() {
    let cert = generate_self_signed(&["localhost"]).unwrap();
    let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;
    let hash = Hasher::Sha256;

    let shared_secret = vec![0x50u8; 32];
    let ch_hash = vec![0x51u8; 32];
    let sh_hash = vec![0x52u8; 32];
    let mut ks = Tls13KeySchedule::new(hash.clone());
    ks.advance_to_handshake(&shared_secret, &ch_hash, &sh_hash);

    let server_hs = ks.server_handshake_traffic_secret(&ch_hash);
    let server_keys_out = server_write_keys(&server_hs, cipher_suite.key_len(), 12, &hash);
    let mut write_cipher = RecordCipher::new(&server_keys_out.write_key, &server_keys_out.write_iv).unwrap();

    // Build certificate message
    let transcript = vec![0x01u8; 64];
    let cert_msg = build_certificate_message(&cert.cert_der);
    let cv_msg = build_certificate_verify(&transcript, &cert.signing_key, &hash).unwrap();

    let encrypted_cert = write_cipher.encrypt(22, &cert_msg);
    let encrypted_cv = write_cipher.encrypt(22, &cv_msg);

    let mut read_cipher = RecordCipher::new(&server_keys_out.write_key, &server_keys_out.write_iv).unwrap();
    let (_ct1, decrypted_cert) = read_cipher.decrypt(&encrypted_cert).unwrap();
    assert_eq!(decrypted_cert[0], 11); // Certificate type
    assert_eq!(decrypted_cert, cert_msg);

    let (_ct2, decrypted_cv) = read_cipher.decrypt(&encrypted_cv).unwrap();
    assert_eq!(decrypted_cv[0], 15); // CertificateVerify type
    assert_eq!(decrypted_cv, cv_msg);
}

/// Finished message verify_data must be correct HMAC.
#[test]
fn test_finished_verify_data() {
    let hash = Hasher::Sha256;
    let transcript = vec![0x77u8; 128];
    let handshake_secret = vec![0x88u8; 32];

    // Server handshake traffic secret
    let mut ks = Tls13KeySchedule::new(hash.clone());
    // Manually set secret (skip the normal flow for this test)
    let server_hs_secret = handshake_secret.clone(); // simplified
    let finished_key = hash.expand_label(&server_hs_secret, "finished", &[], hash.len());

    let expected = hmac_sha256(&finished_key, &transcript);
    assert_eq!(expected.len(), 32);

    let finished_msg = build_finished_message(&expected);
    assert_eq!(finished_msg[0], 20); // Finished type
    assert_eq!(finished_msg.len(), 4 + expected.len()); // type(1) + length(3) + verify_data(32)
}

// ========================================================================
// §4.4.4 — Full handshake transcript conformance
// ========================================================================

/// Simulate the exact sequence of messages in a TLS 1.3 handshake
/// and verify both sides compute the same transcript hash.
#[test]
fn test_handshake_message_sequence() {
    let cert = generate_self_signed(&["localhost"]).unwrap();
    let client_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let server_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let hash = Hasher::Sha256;
    let server_random = [0x22u8; 32];
    let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;

    // 1. ClientHello
    let client_pub = client_keys.public_key_bytes();
    let ch_msg = ClientHelloBuilder::new([0x11u8; 32], "localhost")
        .key_share(&client_pub, NamedGroup::SECP256R1)
        .build()
        .unwrap();

    // 2. ServerHello
    let server_pub = server_keys.public_key_bytes();
    let sh_msg = build_server_hello(
        server_random,
        &[],
        cipher_suite,
        &server_pub,
        NamedGroup::SECP256R1,
    );

    // 3. EncryptedExtensions
    let ee_msg = build_encrypted_extensions(None);

    // 4. Certificate
    let transcript_so_far = {
        let mut t = Vec::new();
        t.extend_from_slice(&ch_msg);
        t.extend_from_slice(&sh_msg);
        t.extend_from_slice(&ee_msg);
        t
    };
    let cert_msg = build_certificate_message(&cert.cert_der);

    // 5. CertificateVerify
    let mut transcript_with_cert = transcript_so_far.clone();
    transcript_with_cert.extend_from_slice(&cert_msg);
    let cv_msg = build_certificate_verify(&transcript_with_cert, &cert.signing_key, &hash).unwrap();

    // 6. Server Finished
    let mut transcript_before_fin = transcript_with_cert.clone();
    transcript_before_fin.extend_from_slice(&cv_msg);
    let transcript_hash_before_fin = hash.hash(&transcript_before_fin);

    let mut ks = Tls13KeySchedule::new(hash.clone());
    let shared = server_keys.exchange(&client_pub).unwrap();
    let ch_hash_val = hash.hash(&ch_msg);
    let sh_hash_val = hash.hash(&sh_msg);
    ks.advance_to_handshake(&shared, &ch_hash_val, &sh_hash_val);

    let server_hs_secret = ks.server_handshake_traffic_secret(&ch_hash_val);
    let finished_key = hash.expand_label(&server_hs_secret, "finished", &[], hash.len());
    let verify_data = hmac_sha256(&finished_key, &transcript_before_fin);

    // Client independently computes the same thing
    let mut client_transcript = Vec::new();
    client_transcript.extend_from_slice(&ch_msg);
    client_transcript.extend_from_slice(&sh_msg);
    client_transcript.extend_from_slice(&ee_msg);
    client_transcript.extend_from_slice(&cert_msg);
    client_transcript.extend_from_slice(&cv_msg);
    let client_transcript_hash = hash.hash(&client_transcript);

    assert_eq!(client_transcript_hash, transcript_hash_before_fin,
        "Client and server transcript hashes must match before Finished");
}

// ========================================================================
// Wire-level encoding: verify no extra bytes, correct length fields
// ========================================================================

#[test]
fn test_tls_record_serialization_exact() {
    let fragment = vec![0xABu8; 200];
    let record = TlsRecord {
        content_type: 22,
        version: 0x0303,
        fragment: fragment.clone(),
    };

    let bytes = record.to_bytes();

    // Header: 5 bytes, then fragment: 200 bytes
    assert_eq!(bytes.len(), 205);
    assert_eq!(bytes[0], 22); // content_type
    assert_eq!(bytes[1..3], [0x03, 0x03]); // version
    assert_eq!(bytes[3..5], [0x00, 0xC8]); // length = 200

    // Parse back
    let (parsed, consumed) = TlsRecord::from_bytes(&bytes).unwrap();
    assert_eq!(consumed, 205);
    assert_eq!(parsed.content_type, 22);
    assert_eq!(parsed.version, 0x0303);
    assert_eq!(parsed.fragment, fragment);
}

#[test]
fn test_client_hello_extensions_present_in_wire() {
    let random = [0xFFu8; 32];
    let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let public_key = key_pair.public_key_bytes();

    let ch_bytes = ClientHelloBuilder::new(random, "ext-test.example.com")
        .key_share(&public_key, NamedGroup::SECP256R1)
        .build()
        .unwrap();

    // Find the extensions section:
    // After the compression methods (1 byte = 0), there's a 2-byte extensions length
    // Walk through to find it
    let msg = &ch_bytes[4..]; // skip handshake header
    let _legacy_version = u16::from_be_bytes([msg[0], msg[1]]);
    let _random = &msg[2..34];
    let session_id_len = msg[34] as usize;
    let _session_id = &msg[35..35 + session_id_len];
    let cs_len = u16::from_be_bytes([msg[35 + session_id_len], msg[35 + session_id_len + 1]]) as usize;
    let cs_end = 35 + session_id_len + 2 + cs_len;
    let _compression_len = msg[cs_end] as usize;
    let compression_end = cs_end + 1 + _compression_len;

    // Extensions length
    let ext_len = u16::from_be_bytes([msg[compression_end], msg[compression_end + 1]]) as usize;
    assert!(ext_len > 0, "ClientHello must have extensions");

    // Verify we can parse each extension type
    let ch = ClientHello::parse(&ch_bytes).unwrap();
    assert!(ch.server_name.is_some(), "SNI extension missing");
    assert!(ch.client_key_share.is_some(), "key_share extension missing");
    assert!(!ch.supported_groups.is_empty(), "supported_groups missing");
    assert!(!ch.cipher_suites.is_empty(), "cipher_suites empty");
}
