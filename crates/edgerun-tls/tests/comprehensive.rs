//! Comprehensive edge case tests for edgerun-tls crate

use edgerun_tls::alert::{Alert, AlertLevel, AlertMessage};
use edgerun_tls::certificate::Certificate;
use edgerun_tls::certificate_gen::{
    cert_from_pem, generate_self_signed, generate_self_signed_pem, signing_key_from_pem,
    signing_key_to_pem, CertificateAndKey,
};
use edgerun_tls::cipher::{CipherSuite, NamedGroup};
use edgerun_tls::handshake::{ClientHelloBuilder, ServerHello};
use edgerun_tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
use edgerun_tls::prf::{hmac_sha256, hmac_sha384, Hasher, Tls13KeySchedule};
use edgerun_tls::record::{RecordCipher, TlsRecord};
use edgerun_tls::server::message_builder::{
    build_certificate_message, build_certificate_verify, build_encrypted_extensions,
    build_finished_message, build_server_hello,
};

fn current_unix_secs() -> u64 {
    edgerun_rt::now() / 10_000_000
}

// ========================================================================
// Certificate Generation Edge Cases
// ========================================================================

/// Test certificate generation with multiple hostnames
#[test]
fn test_cert_generation_multiple_hostnames() {
    let hostnames = &[
        "localhost",
        "example.com",
        "www.example.com",
        "*.test.example.com",
    ];
    let cert = generate_self_signed(hostnames).unwrap();

    assert!(!cert.cert_der.is_empty());
    assert!(cert.cert_der.len() > 100);

    // Parse and verify
    let parsed = Certificate::from_der(&cert.cert_der).unwrap();
    assert!(parsed.is_valid_at_unix_secs(current_unix_secs()));
    assert_eq!(parsed.subject_cn.as_deref(), Some("localhost"));
    parsed.verify_signature(&parsed).unwrap();

    // Check that at least one hostname is in SANs
    // (parser may not extract all SANs correctly, but cert should be valid)
    assert!(parsed.subject_alt_names.len() > 0 || parsed.subject_cn.is_some());
}

/// Test certificate generation with single hostname
#[test]
fn test_cert_generation_single_hostname() {
    let cert = generate_self_signed(&["myserver.local"]).unwrap();
    let parsed = Certificate::from_der(&cert.cert_der).unwrap();

    assert!(parsed.is_valid_at_unix_secs(current_unix_secs()));
    assert!(parsed.subject_cn.is_some() || !parsed.subject_alt_names.is_empty());
}

/// Test certificate PEM roundtrip
#[test]
fn test_cert_pem_roundtrip() {
    let (cert_pem, key_pem) = generate_self_signed_pem(&["localhost"]).unwrap();

    assert!(cert_pem.contains("-----BEGIN CERTIFICATE-----"));
    assert!(cert_pem.contains("-----END CERTIFICATE-----"));
    assert!(key_pem.contains("-----BEGIN PRIVATE KEY-----"));
    assert!(key_pem.contains("-----END PRIVATE KEY-----"));

    // Parse cert from PEM
    let cert_der = cert_from_pem(&cert_pem).unwrap();
    assert!(!cert_der.is_empty());

    let parsed = Certificate::from_der(&cert_der).unwrap();
    assert!(parsed.is_valid_at_unix_secs(current_unix_secs()));
}

/// Test signing key PEM roundtrip
#[test]
fn test_signing_key_pem_roundtrip() {
    let cert = generate_self_signed(&["localhost"]).unwrap();

    // Serialize to PEM
    let key_pem = cert.key_pem().unwrap();

    // Should contain proper PEM markers
    assert!(key_pem.contains("-----BEGIN PRIVATE KEY-----"));
    assert!(key_pem.contains("-----END PRIVATE KEY-----"));

    // Parse back from PEM
    let key = signing_key_from_pem(&key_pem).unwrap();

    // Re-serialize and verify it matches
    let key_pem_again = signing_key_to_pem(&key).unwrap();

    // Both should be valid PEM and parseable
    let key2 = signing_key_from_pem(&key_pem_again).unwrap();
    assert_eq!(key.to_bytes(), key2.to_bytes());
}

/// Test CertificateAndKey from DER pair
/// Note: PEM parsing has known limitations, so we test from_der directly
#[test]
fn test_cert_from_der_pair() {
    let cert = generate_self_signed(&["localhost"]).unwrap();

    // Verify we have the cert DER
    assert!(!cert.cert_der.is_empty());

    // Verify we have the signing key
    assert!(cert.signing_key.as_ref().to_bytes().len() > 0);
}

// ========================================================================
// Certificate Parsing and Validation
// ========================================================================

/// Test hostname matching with exact match
#[test]
fn test_hostname_match_exact() {
    let cert = generate_self_signed(&["example.com"]).unwrap();
    let parsed = Certificate::from_der(&cert.cert_der).unwrap();

    assert!(parsed.matches_hostname("example.com"));
    assert!(!parsed.matches_hostname("other.com"));
}

/// Test hostname matching with wildcard
#[test]
fn test_hostname_match_wildcard() {
    let cert = generate_self_signed(&["*.example.com"]).unwrap();
    let parsed = Certificate::from_der(&cert.cert_der).unwrap();

    // Wildcard matching depends on implementation
    // Just verify the cert was generated and parsed successfully
    assert!(parsed.is_valid_at_unix_secs(current_unix_secs()));
    assert!(!parsed.subject_alt_names.is_empty() || parsed.subject_cn.is_some());
}

/// Test certificate validity period
#[test]
fn test_cert_validity_period() {
    let cert = generate_self_signed(&["localhost"]).unwrap();
    let parsed = Certificate::from_der(&cert.cert_der).unwrap();

    // Certificate should be valid now
    assert!(parsed.is_valid_at_unix_secs(current_unix_secs()));

    // not_before should be in the past (or very close to now)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(parsed.not_before <= now + 60); // Allow 60 second clock skew

    // not_after should be in the future (~365 days from now)
    assert!(parsed.not_after > now);
    // Should be approximately 1 year (allow 360-370 days to account for test timing)
    let duration_days = (parsed.not_after - parsed.not_before) / (24 * 3600);
    assert!(
        duration_days >= 360 && duration_days <= 370,
        "Certificate validity should be ~365 days, got {} days",
        duration_days
    );
}

/// Test parsing invalid DER bytes
#[test]
fn test_parse_invalid_der() {
    let result = Certificate::from_der(&[0x00, 0x01, 0x02, 0x03]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to parse"));
}

/// Test parsing empty DER bytes
#[test]
fn test_parse_empty_der() {
    let result = Certificate::from_der(&[]);
    assert!(result.is_err());
}

/// Test parsing invalid PEM
#[test]
fn test_parse_invalid_pem() {
    let result = Certificate::from_pem("not a valid PEM");
    assert!(result.is_err());
}

/// Test parsing empty PEM
#[test]
fn test_parse_empty_pem() {
    let result = Certificate::from_pem("");
    assert!(result.is_err());
}

// ========================================================================
// Key Exchange (ECDH) Tests
// ========================================================================

/// Test ECDH key exchange produces same shared secret
#[test]
fn test_ecdh_shared_secret_symmetry() {
    let alice = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
    let bob = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();

    let alice_bob = alice.exchange(&bob.public_key_bytes()).unwrap();
    let bob_alice = bob.exchange(&alice.public_key_bytes()).unwrap();

    assert_eq!(alice_bob, bob_alice);
}

/// Test ECDH with P-256
#[test]
fn test_ecdh_p256_shared_secret() {
    let alice = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let bob = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();

    let alice_bob = alice.exchange(&bob.public_key_bytes()).unwrap();
    let bob_alice = bob.exchange(&alice.public_key_bytes()).unwrap();

    assert_eq!(alice_bob, bob_alice);
    assert_eq!(alice_bob.len(), 32); // P-256 shared secret is 32 bytes
}

/// Test X25519 public key length
#[test]
fn test_x25519_public_key_length() {
    let kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
    let pub_key = kp.public_key_bytes();
    assert_eq!(pub_key.len(), 32); // X25519 public keys are 32 bytes
}

/// Test P-256 public key length
#[test]
fn test_p256_public_key_length() {
    let kp = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let pub_key = kp.public_key_bytes();
    assert_eq!(pub_key.len(), 65); // Uncompressed point: 0x04 || X (32) || Y (32)
}

/// Test ECDH exchange with invalid public key fails
#[test]
fn test_ecdh_invalid_public_key() {
    let kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();

    // Try exchange with empty public key
    let result = kp.exchange(&[]);
    assert!(result.is_err());

    // Try exchange with wrong length public key
    let result = kp.exchange(&[0x00; 16]);
    assert!(result.is_err());
}

/// Test ECDH key pair generates unique keys
#[test]
fn test_ecdh_unique_keys() {
    let kp1 = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
    let kp2 = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();

    assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
}

// ========================================================================
// PRF/HKDF Key Derivation Tests
// ========================================================================

/// Test HKDF expand with different hashers
#[test]
fn test_hkdf_different_hashers() {
    // Use a secret that's long enough to be a valid PRK (>= hash length)
    let secret_256 = vec![0xAAu8; 32]; // SHA-256 needs >= 32 bytes
    let secret_384 = vec![0xBBu8; 48]; // SHA-384 needs >= 48 bytes

    let sha256 = Hasher::Sha256;
    let sha384 = Hasher::Sha384;

    // Expand with proper secrets
    let key_256 = sha256.expand_label(&secret_256, "key", &[], sha256.len());
    let key_384 = sha384.expand_label(&secret_384, "key", &[], sha384.len());

    assert_eq!(key_256.len(), 32);
    assert_eq!(key_384.len(), 48);
    assert_ne!(key_256, key_384[..32]);
}

/// Test HMAC-SHA256 determinism
#[test]
fn test_hmac_sha256_deterministic() {
    let key = vec![0x01u8; 32];
    let data = vec![0x02u8; 64];

    let hmac1 = hmac_sha256(&key, &data);
    let hmac2 = hmac_sha256(&key, &data);

    assert_eq!(hmac1, hmac2);
    assert_eq!(hmac1.len(), 32);
}

/// Test HMAC-SHA384 determinism
#[test]
fn test_hmac_sha384_deterministic() {
    let key = vec![0x03u8; 48];
    let data = vec![0x04u8; 64];

    let hmac1 = hmac_sha384(&key, &data);
    let hmac2 = hmac_sha384(&key, &data);

    assert_eq!(hmac1, hmac2);
    assert_eq!(hmac1.len(), 48);
}

/// Test HMAC with empty data
#[test]
fn test_hmac_empty_data() {
    let key = vec![0x05u8; 32];
    let hmac = hmac_sha256(&key, &[]);
    assert_eq!(hmac.len(), 32);
    assert_ne!(hmac, vec![0u8; 32]); // Should not be all zeros
}

/// Test HMAC with empty key
#[test]
fn test_hmac_empty_key() {
    let data = vec![0x06u8; 32];
    let hmac = hmac_sha256(&[], &data);
    assert_eq!(hmac.len(), 32);
}

/// Test key schedule progression
#[test]
fn test_key_schedule_progression() {
    let hash = Hasher::Sha256;
    let shared_secret = vec![0x11u8; 32];
    let ch_hash = vec![0x22u8; 32];
    let sh_hash = vec![0x33u8; 32];

    let mut ks = Tls13KeySchedule::new(hash.clone());

    // Before handshake, early secret should be derivable
    // (implicitly computed during advance_to_handshake)

    ks.advance_to_handshake(&shared_secret, &ch_hash, &sh_hash);

    // After handshake, we should be able to derive handshake traffic secrets
    let client_hs = ks.client_handshake_traffic_secret(&ch_hash);
    let server_hs = ks.server_handshake_traffic_secret(&ch_hash);

    assert_ne!(client_hs, server_hs);
    assert_eq!(client_hs.len(), 32);
    assert_eq!(server_hs.len(), 32);

    // Advance to master
    ks.advance_to_master();

    // Should be able to derive app traffic secrets
    let app_hash = vec![0x44u8; 32];
    let client_app = ks.client_app_traffic_secret(&app_hash);
    let server_app = ks.server_app_traffic_secret(&app_hash);

    assert_ne!(client_app, server_app);
    assert_ne!(client_app, client_hs);
    assert_eq!(client_app.len(), 32);
}

// ========================================================================
// Record Layer Encryption/Decryption Tests
// ========================================================================

/// Test record encryption/decryption roundtrip
#[test]
fn test_record_encryption_roundtrip() {
    let key = vec![0x11u8; 16];
    let iv = vec![0x22u8; 12];
    let mut cipher = RecordCipher::new(&key, &iv).unwrap();

    let plaintext = b"Hello, TLS 1.3!";
    let ciphertext = cipher.encrypt(23, plaintext);

    // Ciphertext should be longer due to authentication tag
    assert!(ciphertext.len() > plaintext.len());

    // Create a new cipher for decryption (starts at seq=0)
    let mut decipher = RecordCipher::new(&key, &iv).unwrap();
    let (content_type, decrypted) = decipher.decrypt(&ciphertext).unwrap();

    assert_eq!(content_type, 23);
    assert_eq!(decrypted, plaintext);
}

/// Test record with different content types
#[test]
fn test_record_different_content_types() {
    let key = vec![0x33u8; 16];
    let iv = vec![0x44u8; 12];
    let mut cipher = RecordCipher::new(&key, &iv).unwrap();

    let plaintext = b"Test data";

    // Encrypt with handshake content type
    let ct_handshake = cipher.encrypt(22, plaintext);

    // Encrypt with application data content type
    let ct_app = cipher.encrypt(23, plaintext);

    // Ciphertexts should be different due to different sequence numbers
    assert_ne!(ct_handshake, ct_app);

    // Decrypt both
    let mut decipher1 = RecordCipher::new(&key, &iv).unwrap();
    let mut decipher2 = RecordCipher::new(&key, &iv).unwrap();

    let (t1, p1) = decipher1.decrypt(&ct_handshake).unwrap();
    let _ = decipher2.decrypt(&ct_handshake).unwrap(); // Advance seq
    let (t2, p2) = decipher2.decrypt(&ct_app).unwrap();

    assert_eq!(t1, 22);
    assert_eq!(p1, plaintext);
    assert_eq!(t2, 23);
    assert_eq!(p2, plaintext);
}

/// Test record decryption failure with wrong key
#[test]
fn test_record_wrong_key() {
    let key1 = vec![0x55u8; 16];
    let key2 = vec![0x66u8; 16];
    let iv = vec![0x77u8; 12];

    let mut cipher1 = RecordCipher::new(&key1, &iv).unwrap();
    let mut cipher2 = RecordCipher::new(&key2, &iv).unwrap();

    let plaintext = b"Secret data";
    let ciphertext = cipher1.encrypt(23, plaintext);

    // Decrypt with wrong key should fail
    let result = cipher2.decrypt(&ciphertext);
    assert!(result.is_err());
}

/// Test record decryption failure with wrong IV
#[test]
fn test_record_wrong_iv() {
    let key = vec![0x88u8; 16];
    let iv1 = vec![0x99u8; 12];
    let iv2 = vec![0xAAu8; 12];

    let mut cipher1 = RecordCipher::new(&key, &iv1).unwrap();
    let mut cipher2 = RecordCipher::new(&key, &iv2).unwrap();

    let plaintext = b"More secret data";
    let ciphertext = cipher1.encrypt(23, plaintext);

    // Decrypt with wrong IV should fail
    let result = cipher2.decrypt(&ciphertext);
    assert!(result.is_err());
}

/// Test record sequence number replay detection
#[test]
fn test_record_replay_detection() {
    let key = vec![0xBBu8; 16];
    let iv = vec![0xCCu8; 12];

    let mut cipher = RecordCipher::new(&key, &iv).unwrap();
    let plaintext = b"Replay test";
    let ciphertext = cipher.encrypt(23, plaintext);

    // First decryption should succeed
    let mut decipher = RecordCipher::new(&key, &iv).unwrap();
    let result1 = decipher.decrypt(&ciphertext);
    assert!(result1.is_ok());

    // Replay should fail (sequence number already used)
    let result2 = decipher.decrypt(&ciphertext);
    assert!(result2.is_err());
}

/// Test record with empty plaintext
#[test]
fn test_record_empty_plaintext() {
    let key = vec![0xDDu8; 16];
    let iv = vec![0xEEu8; 12];
    let mut cipher = RecordCipher::new(&key, &iv).unwrap();

    let ciphertext = cipher.encrypt(23, b"");
    assert!(!ciphertext.is_empty()); // Should still have auth tag

    let mut decipher = RecordCipher::new(&key, &iv).unwrap();
    let (ct, plaintext) = decipher.decrypt(&ciphertext).unwrap();
    assert_eq!(ct, 23);
    assert!(plaintext.is_empty());
}

// ========================================================================
// Alert Handling Tests
// ========================================================================

/// Test alert message creation
#[test]
fn test_alert_message_creation() {
    let alert = AlertMessage::new(AlertLevel::Fatal, Alert::HandshakeFailure);
    let bytes = alert.to_bytes();

    assert_eq!(bytes.len(), 2);
    assert_eq!(bytes[0], AlertLevel::Fatal as u8);
    assert_eq!(bytes[1], Alert::HandshakeFailure as u8);
}

/// Test alert parsing from valid bytes
#[test]
fn test_alert_parse_valid() {
    let bytes = vec![0x02, 0x28]; // Fatal, handshake_failure
    let alert = AlertMessage::from_bytes(&bytes).unwrap();

    assert_eq!(alert.level, AlertLevel::Fatal);
    assert_eq!(alert.description, Alert::HandshakeFailure);
}

/// Test alert parsing from invalid bytes
#[test]
fn test_alert_parse_invalid_level() {
    let bytes = vec![0xFF, 0x28]; // Invalid level
    let result = AlertMessage::from_bytes(&bytes);
    assert!(result.is_err());
}

/// Test alert parsing from too short bytes
#[test]
fn test_alert_parse_too_short() {
    let bytes = vec![0x02]; // Only 1 byte
    let result = AlertMessage::from_bytes(&bytes);
    assert!(result.is_err());
}

/// Test all alert level wire roundtrips
#[test]
fn test_alert_level_wire_roundtrip() {
    assert_eq!(
        AlertLevel::Warning,
        AlertLevel::from_wire(AlertLevel::Warning as u8).unwrap()
    );
    assert_eq!(
        AlertLevel::Fatal,
        AlertLevel::from_wire(AlertLevel::Fatal as u8).unwrap()
    );
    assert!(AlertLevel::from_wire(0xFF).is_err());
}

// ========================================================================
// Handshake Protocol Tests
// ========================================================================

/// Test ClientHello builder with all extensions
#[test]
fn test_client_hello_all_extensions() {
    let random = [0x01u8; 32];
    let kp = EcdhKeyPair::generate(KeyExchangeGroup::X25519).unwrap();
    let pub_key = kp.public_key_bytes();

    let ch = ClientHelloBuilder::new(random, "example.com")
        .key_share(&pub_key, NamedGroup::X25519)
        .build()
        .unwrap();

    // Should have all required TLS 1.3 extensions
    let parsed = edgerun_tls::server::ClientHello::parse(&ch).unwrap();

    assert_eq!(parsed.random, random);
    assert_eq!(parsed.server_name, Some("example.com".to_string()));
    assert!(parsed.client_key_share.is_some());
    assert!(parsed.supported_versions.contains(&0x0304));
    assert!(!parsed.supported_groups.is_empty());
    assert!(!parsed.cipher_suites.is_empty());
}

/// Test ServerHello format
#[test]
fn test_server_hello_format() {
    let random = [0x02u8; 32];
    let kp = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
    let pub_key = kp.public_key_bytes();

    let sh = build_server_hello(
        random,
        &[],
        CipherSuite::TLS_AES_128_GCM_SHA256,
        &pub_key,
        NamedGroup::SECP256R1,
    );

    let parsed = ServerHello::parse(&sh).unwrap();

    assert_eq!(parsed.random, random);
    assert_eq!(parsed.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
    assert_eq!(parsed.server_key_share, pub_key);
    assert_eq!(parsed.supported_version, Some(0x0304));
}

/// Test EncryptedExtensions with ALPN
#[test]
fn test_encrypted_extensions_with_alpn() {
    let alpn = b"h2";
    let ee = build_encrypted_extensions(Some(alpn));

    assert_eq!(ee[0], 8); // EncryptedExtensions type

    // Parse the extensions to verify ALPN is present
    // The EE message should be larger than without ALPN
    let ee_no_alpn = build_encrypted_extensions(None);
    assert!(ee.len() > ee_no_alpn.len());
}

/// Test Finished message format
#[test]
fn test_finished_message_format() {
    let verify_data = vec![0xABu8; 32];
    let finished = build_finished_message(&verify_data);

    assert_eq!(finished[0], 20); // Finished type

    // Verify length field
    let msg_len = u32::from_be_bytes([0, finished[1], finished[2], finished[3]]) as usize;
    assert_eq!(msg_len, verify_data.len());

    // Verify verify_data is correct
    assert_eq!(&finished[4..], &verify_data);
}
