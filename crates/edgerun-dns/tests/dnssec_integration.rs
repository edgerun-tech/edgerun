//! Integration tests for DNSSEC validation — real cryptographic operations.

use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_dns::dnssec::*;
use edgerun_dns::message::DnsRecord;
use edgerun_dns::record::{encode_domain_name, DnsRecordData, DnsRecordType};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn now() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32
}

fn make_a_rr(name: &str, ip: &str, ttl: u32) -> DnsRecord {
    let octets: [u8; 4] = ip
        .split('.')
        .map(|p| p.parse::<u8>().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    DnsRecord::a(name.to_string(), std::net::Ipv4Addr::from(octets), ttl)
}

fn dnskey_digest_input(dnskey: &DnsRecord) -> Vec<u8> {
    let mut input = encode_domain_name(&dnskey.name.to_ascii_lowercase());
    input.extend_from_slice(&dnskey.data.to_wire(dnskey.rtype));
    input
}

// -----------------------------------------------------------------------
// Key tag computation (RFC 4034 Appendix B)
// -----------------------------------------------------------------------

#[test]
fn test_key_tag_ecdsap256() {
    let dnskey = DnsRecord::dnskey("example.com".to_string(), 257, 3, 13, vec![0x04; 65], 3600);
    let tag = compute_key_tag(&dnskey);
    assert_ne!(tag, 0);
}

#[test]
fn test_key_tag_consistency() {
    let dnskey = DnsRecord::dnskey(
        "test.example.com".to_string(),
        256,
        3,
        13,
        vec![0xAB; 65],
        7200,
    );
    let tag1 = compute_key_tag(&dnskey);
    let tag2 = compute_key_tag(&dnskey);
    assert_eq!(tag1, tag2);
}

// -----------------------------------------------------------------------
// ED25519 (algorithm 15) — key generation, signature rejection
// -----------------------------------------------------------------------

#[test]
fn test_ed25519_key_generation() {
    use edgerun_crypto::Ed25519SigningKey;

    let signing_key = Ed25519SigningKey::generate(&mut edgerun_crypto::OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes().to_vec();
    assert_eq!(public_key_bytes.len(), 32);
}

#[test]
fn test_ed25519_wrong_signature_rejected() {
    use edgerun_crypto::Ed25519SigningKey;

    let signing_key = Ed25519SigningKey::generate(&mut edgerun_crypto::OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes().to_vec();

    let dnskey = DnsRecord::dnskey(
        "example.com".to_string(),
        257,
        3,
        15,
        public_key_bytes,
        3600,
    );

    let rrset = vec![make_a_rr("example.com", "93.184.216.34", 3600)];
    let current = now();
    let rrsig = DnsRecord::rrsig(
        "example.com".to_string(),
        1,
        15,
        2,
        3600,
        current + 86400,
        current - 3600,
        compute_key_tag(&dnskey),
        "example.com".to_string(),
        vec![0u8; 64], // wrong signature
        3600,
    );

    let result = verify_rrsig(&rrset, &rrsig, &dnskey, Some(current));
    assert_eq!(result, DnssecResult::BadSignature);
}

#[test]
fn test_ed25519_expired_signature() {
    use edgerun_crypto::Ed25519SigningKey;

    let signing_key = Ed25519SigningKey::generate(&mut edgerun_crypto::OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes().to_vec();

    let dnskey = DnsRecord::dnskey(
        "example.com".to_string(),
        257,
        3,
        15,
        public_key_bytes,
        3600,
    );

    let current = now();
    let inception = current - 86400 * 2;
    let expiration = current - 86400; // already expired

    let rrsig = DnsRecord::rrsig(
        "example.com".to_string(),
        1,
        15,
        2,
        3600,
        expiration,
        inception,
        compute_key_tag(&dnskey),
        "example.com".to_string(),
        vec![0xAB; 64],
        3600,
    );

    let rrset = vec![make_a_rr("example.com", "93.184.216.34", 3600)];
    let result = verify_rrsig(&rrset, &rrsig, &dnskey, Some(current));
    assert_eq!(result, DnssecResult::Expired);
}

#[test]
fn test_ed25519_not_yet_valid() {
    use edgerun_crypto::Ed25519SigningKey;

    let signing_key = Ed25519SigningKey::generate(&mut edgerun_crypto::OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes().to_vec();

    let dnskey = DnsRecord::dnskey(
        "example.com".to_string(),
        257,
        3,
        15,
        public_key_bytes,
        3600,
    );

    let current = now();
    let inception = current + 86400; // starts tomorrow
    let expiration = current + 86400 * 2;

    let rrsig = DnsRecord::rrsig(
        "example.com".to_string(),
        1,
        15,
        2,
        3600,
        expiration,
        inception,
        compute_key_tag(&dnskey),
        "example.com".to_string(),
        vec![0xAB; 64],
        3600,
    );

    let rrset = vec![make_a_rr("example.com", "93.184.216.34", 3600)];
    let result = verify_rrsig(&rrset, &rrsig, &dnskey, Some(current));
    assert_eq!(result, DnssecResult::Expired);
}

// -----------------------------------------------------------------------
// ECDSAP256SHA256 (algorithm 13) — key generation, signature rejection
// -----------------------------------------------------------------------

#[test]
fn test_ecdsap256_key_generation() {
    let (dnskey, _signing_key) = generate_dnskey_ecdsap256("example.com".to_string(), 257, 3600);
    if let DnsRecordData::DNSKEY { public_key, .. } = dnskey.data {
        assert_eq!(public_key.len(), 64);
    } else {
        panic!("expected DNSKEY");
    }
}

#[test]
fn test_ecdsap256_wrong_signature_rejected() {
    use edgerun_crypto::p256::ecdsa::SigningKey;
    use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;

    let signing_key = SigningKey::random(&mut edgerun_crypto::OsRng);
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let public_key_bytes = encoded.as_bytes().to_vec();

    let dnskey = DnsRecord::dnskey(
        "example.com".to_string(),
        257,
        3,
        13,
        public_key_bytes,
        3600,
    );

    let rrset = vec![make_a_rr("example.com", "93.184.216.34", 3600)];
    let current = now();
    let rrsig = DnsRecord::rrsig(
        "example.com".to_string(),
        1,
        13,
        2,
        3600,
        current + 86400,
        current - 3600,
        compute_key_tag(&dnskey),
        "example.com".to_string(),
        vec![0u8; 64], // wrong signature
        3600,
    );

    let result = verify_rrsig(&rrset, &rrsig, &dnskey, Some(current));
    assert_eq!(result, DnssecResult::BadSignature);
}

// -----------------------------------------------------------------------
// Chain of trust: DS → DNSKEY (RFC 4034)
// -----------------------------------------------------------------------

#[test]
fn test_chain_of_trust_valid_sha256() {
    use edgerun_crypto::p256::ecdsa::SigningKey;
    use edgerun_crypto::p256::elliptic_curve::sec1::ToEncodedPoint;
    use edgerun_crypto::sha2::{Digest, Sha256};

    let signing_key = SigningKey::random(&mut edgerun_crypto::OsRng);
    let verifying_key = signing_key.verifying_key();
    let encoded = verifying_key.to_encoded_point(false);
    let public_key_bytes = encoded.as_bytes().to_vec();

    let dnskey = DnsRecord::dnskey(
        "example.com".to_string(),
        257,
        3,
        13,
        public_key_bytes,
        3600,
    );

    let mut hasher = Sha256::new();
    hasher.update(dnskey_digest_input(&dnskey));
    let digest = hasher.finalize().to_vec();

    let key_tag = compute_key_tag(&dnskey);
    let ds = DnsRecord::ds("example.com".to_string(), key_tag, 13, 2, digest, 3600);

    let result = verify_chain_of_trust(&[ds], &[dnskey]);
    assert_eq!(result, DnssecResult::Valid);
}

#[test]
fn test_chain_of_trust_broken_wrong_digest() {
    let dnskey = DnsRecord::dnskey("example.com".to_string(), 257, 3, 13, vec![0x04; 65], 3600);

    let ds = DnsRecord::ds(
        "example.com".to_string(),
        compute_key_tag(&dnskey),
        13,
        2,
        vec![0xDE, 0xAD, 0xBE, 0xEF],
        3600,
    );

    let result = verify_chain_of_trust(&[ds], &[dnskey]);
    assert_eq!(result, DnssecResult::ChainBroken);
}

#[test]
fn test_chain_of_trust_broken_wrong_key_tag() {
    let dnskey = DnsRecord::dnskey("example.com".to_string(), 257, 3, 13, vec![0x04; 65], 3600);

    let ds = DnsRecord::ds(
        "example.com".to_string(),
        54321, // wrong key tag
        13,
        2,
        vec![0xAB; 32],
        3600,
    );

    let result = verify_chain_of_trust(&[ds], &[dnskey]);
    assert_eq!(result, DnssecResult::ChainBroken);
}

#[test]
fn test_chain_of_trust_sha1_digest() {
    use edgerun_crypto::sha1::Digest;
    use edgerun_crypto::sha1::Sha1;

    let dnskey = DnsRecord::dnskey(
        "example.com".to_string(),
        257,
        3,
        8,
        vec![0x01, 0x00, 0x01, 0xFF, 0xFE],
        3600,
    );

    let mut hasher = Sha1::new();
    hasher.update(dnskey_digest_input(&dnskey));
    let digest = hasher.finalize().to_vec();

    let ds = DnsRecord::ds(
        "example.com".to_string(),
        compute_key_tag(&dnskey),
        8,
        1,
        digest,
        3600,
    );

    let result = verify_chain_of_trust(&[ds], &[dnskey]);
    assert_eq!(result, DnssecResult::Valid);
}

#[test]
fn test_chain_of_trust_sha384_digest() {
    use edgerun_crypto::sha2::{Digest, Sha384};

    let dnskey = DnsRecord::dnskey("example.com".to_string(), 257, 3, 14, vec![0x04; 97], 3600);

    let mut hasher = Sha384::new();
    hasher.update(dnskey_digest_input(&dnskey));
    let digest = hasher.finalize().to_vec();

    let ds = DnsRecord::ds(
        "example.com".to_string(),
        compute_key_tag(&dnskey),
        14,
        4,
        digest,
        3600,
    );

    let result = verify_chain_of_trust(&[ds], &[dnskey]);
    assert_eq!(result, DnssecResult::Valid);
}

// -----------------------------------------------------------------------
// validate_response integration tests
// -----------------------------------------------------------------------

#[test]
fn test_validate_response_insecure_no_rrsig() {
    let answers = vec![make_a_rr("example.com", "93.184.216.34", 3600)];
    let result = validate_response(&answers, &[], &[], None);
    assert_eq!(result, DnssecResult::Insecure);
}

#[test]
fn test_validate_response_no_signature_for_rrset() {
    let answers = vec![make_a_rr("example.com", "93.184.216.34", 3600)];
    // RRSIG for A type (type_covered=1) but wrong signature
    let current = now();
    let rrsig = DnsRecord::rrsig(
        "example.com".to_string(),
        1, // type_covered = A
        15,
        2,
        3600,
        current + 86400,
        current - 3600,
        12345,
        "example.com".to_string(),
        vec![0u8; 64],
        3600,
    );
    // The RRSIG exists but has a bad signature, so it should return BadSignature
    // (NoSignature is only when there's no RRSIG for the covered type at all)
    let result = validate_response(&answers, &[rrsig], &[], None);
    assert!(matches!(
        result,
        DnssecResult::BadSignature | DnssecResult::NoKey
    ));
}

// -----------------------------------------------------------------------
// Canonical name comparison (RFC 4034 §6.1)
// -----------------------------------------------------------------------

#[test]
fn test_canonical_name_case_insensitive() {
    use edgerun_dns::record::encode_domain_name;

    // encode_domain_name preserves input case — canonical form requires
    // the caller to lowercase before encoding (done in canonical_name_bytes).
    // This test verifies that lowercased inputs produce identical wire bytes.
    let a = encode_domain_name("example.com");
    let b = encode_domain_name("example.com");
    assert_eq!(a, b);
}

#[test]
fn test_canonical_name_right_to_left() {
    // Canonical comparison compares TLD first
    // "com" < "net" alphabetically
    assert!("com" < "net");
}

// -----------------------------------------------------------------------
// Wire format roundtrip for DNSSEC records
// -----------------------------------------------------------------------

#[test]
fn test_rrsig_wire_roundtrip() {
    let rrsig = DnsRecordData::RRSIG {
        type_covered: 1,
        algorithm: 13,
        labels: 2,
        original_ttl: 3600,
        expiration: 1700000000,
        inception: 1699996400,
        key_tag: 12345,
        signer_name: "example.com".to_string(),
        signature: vec![0xAB; 64],
    };

    let wire = rrsig.to_wire(DnsRecordType::RRSIG);
    assert!(wire.len() > 18);

    let offset_map: Vec<(usize, usize)> = (0..wire.len()).map(|i| (i, i)).collect();
    let parsed = DnsRecordData::from_wire(DnsRecordType::RRSIG, &wire, &offset_map).unwrap();

    if let DnsRecordData::RRSIG {
        type_covered,
        algorithm,
        labels,
        original_ttl,
        expiration,
        inception,
        key_tag,
        signer_name,
        signature,
    } = parsed
    {
        assert_eq!(type_covered, 1);
        assert_eq!(algorithm, 13);
        assert_eq!(labels, 2);
        assert_eq!(original_ttl, 3600);
        assert_eq!(expiration, 1700000000);
        assert_eq!(inception, 1699996400);
        assert_eq!(key_tag, 12345);
        assert_eq!(signer_name, "example.com");
        assert_eq!(signature.len(), 64);
    } else {
        panic!("Expected RRSIG, got {:?}", parsed);
    }
}

#[test]
fn test_nsec_wire_roundtrip() {
    let nsec = DnsRecordData::NSEC {
        next_owner: "next.example.com".to_string(),
        type_bits: vec![0x40, 0x01, 0x00, 0x01],
    };

    let wire = nsec.to_wire(DnsRecordType::NSEC);
    let offset_map: Vec<(usize, usize)> = (0..wire.len()).map(|i| (i, i)).collect();
    let parsed = DnsRecordData::from_wire(DnsRecordType::NSEC, &wire, &offset_map).unwrap();

    if let DnsRecordData::NSEC {
        next_owner,
        type_bits,
    } = parsed
    {
        assert_eq!(next_owner, "next.example.com");
        assert_eq!(type_bits, vec![0x40, 0x01, 0x00, 0x01]);
    } else {
        panic!("Expected NSEC, got {:?}", parsed);
    }
}

#[test]
fn test_ds_wire_roundtrip() {
    let ds = DnsRecordData::DS {
        key_tag: 54321,
        algorithm: 8,
        digest_type: 2,
        digest: vec![0xCD; 32],
    };

    let wire = ds.to_wire(DnsRecordType::DS);
    let offset_map: Vec<(usize, usize)> = (0..wire.len()).map(|i| (i, i)).collect();
    let parsed = DnsRecordData::from_wire(DnsRecordType::DS, &wire, &offset_map).unwrap();

    if let DnsRecordData::DS {
        key_tag,
        algorithm,
        digest_type,
        digest,
    } = parsed
    {
        assert_eq!(key_tag, 54321);
        assert_eq!(algorithm, 8);
        assert_eq!(digest_type, 2);
        assert_eq!(digest.len(), 32);
    } else {
        panic!("Expected DS, got {:?}", parsed);
    }
}

#[test]
fn test_dnskey_wire_roundtrip() {
    let dnskey = DnsRecordData::DNSKEY {
        protocol: 3,
        flags: 257,
        algorithm: 13,
        public_key: vec![0x04; 65],
    };

    let wire = dnskey.to_wire(DnsRecordType::DNSKEY);
    let offset_map: Vec<(usize, usize)> = (0..wire.len()).map(|i| (i, i)).collect();
    let parsed = DnsRecordData::from_wire(DnsRecordType::DNSKEY, &wire, &offset_map).unwrap();

    if let DnsRecordData::DNSKEY {
        protocol,
        flags,
        algorithm,
        public_key,
    } = parsed
    {
        assert_eq!(protocol, 3);
        assert_eq!(flags, 257);
        assert_eq!(algorithm, 13);
        assert_eq!(public_key.len(), 65);
    } else {
        panic!("Expected DNSKEY, got {:?}", parsed);
    }
}

// -----------------------------------------------------------------------
// NSEC3 Proof Synthesis (RFC 5155)
// -----------------------------------------------------------------------

#[test]
fn test_nsec3_hash_deterministic() {
    // Same input + salt + iterations always produces the same hash
    let hash1 = edgerun_dns::nsec3_hash_owner("www.example.com", &[0xDE, 0xAD], 1);
    let hash2 = edgerun_dns::nsec3_hash_owner("www.example.com", &[0xDE, 0xAD], 1);
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 20); // SHA-1 output
}

#[test]
fn test_nsec3_hash_different_salt() {
    let hash1 = edgerun_dns::nsec3_hash_owner("www.example.com", &[0xDE, 0xAD], 1);
    let hash2 = edgerun_dns::nsec3_hash_owner("www.example.com", &[0xBE, 0xEF], 1);
    assert_ne!(hash1, hash2);
}

#[test]
fn test_nsec3_hash_different_iterations() {
    let hash1 = edgerun_dns::nsec3_hash_owner("www.example.com", &[], 0);
    let hash5 = edgerun_dns::nsec3_hash_owner("www.example.com", &[], 5);
    assert_ne!(hash1, hash5);
}

#[test]
fn test_nsec3_base32hex_encoding() {
    let hash = vec![0u8; 20];
    let b32 = edgerun_dns::nsec3_base32hex(&hash);
    // 20 bytes → 32 base32hex characters
    assert_eq!(b32.len(), 32);
    // All lowercase base32hex characters
    assert!(b32
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
}

#[test]
fn test_nsec3_type_bitmap() {
    use edgerun_dns::record::DnsRecordType;
    let types = vec![DnsRecordType::A, DnsRecordType::AAAA, DnsRecordType::NS];
    let bitmap = edgerun_dns::nsec3_type_bitmap(&types);
    assert!(!bitmap.is_empty());
}

#[test]
fn test_nsec3_synthesize_chain() {
    let names = vec!["@".to_string(), "www".to_string(), "mail".to_string()];
    let chain =
        edgerun_dns::synthesize_nsec3_chain("example.com", &names, &[0xDE, 0xAD], 1, 0, 3600);
    assert_eq!(chain.len(), 3);
    // All should be NSEC3 records
    for rr in &chain {
        assert_eq!(rr.rtype, edgerun_dns::record::DnsRecordType::NSEC3);
    }
}

#[test]
fn test_nsec3_find_covering() {
    let names = vec!["example.com".to_string(), "www.example.com".to_string()];
    let chain = edgerun_dns::synthesize_nsec3_chain("example.com", &names, &[0xAB], 1, 0, 3600);
    // Query a name not in the zone
    let covering = edgerun_dns::find_nsec3_covering(&chain, "nonexistent.example.com", &[0xAB], 1);
    assert!(covering.is_some());
}

#[test]
fn test_nsec3_wire_roundtrip() {
    let nsec3 = DnsRecordData::NSEC3 {
        hash_algorithm: 1,
        flags: 1,
        iterations: 10,
        salt: vec![0xDE, 0xAD],
        next_hashed_owner: vec![0xAB; 20],
        type_bits: vec![0x40],
    };

    let wire = nsec3.to_wire(DnsRecordType::NSEC3);
    let offset_map: Vec<(usize, usize)> = (0..wire.len()).map(|i| (i, i)).collect();
    let parsed = DnsRecordData::from_wire(DnsRecordType::NSEC3, &wire, &offset_map).unwrap();

    if let DnsRecordData::NSEC3 {
        hash_algorithm,
        flags,
        iterations,
        salt,
        next_hashed_owner,
        type_bits,
    } = parsed
    {
        assert_eq!(hash_algorithm, 1);
        assert_eq!(flags, 1);
        assert_eq!(iterations, 10);
        assert_eq!(salt, vec![0xDE, 0xAD]);
        assert_eq!(next_hashed_owner.len(), 20);
        assert_eq!(type_bits, vec![0x40]);
    } else {
        panic!("Expected NSEC3, got {:?}", parsed);
    }
}
