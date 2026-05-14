use edgerun_crypto as crypto;

#[cfg(feature = "aes")]
#[test]
fn aes128_matches_nist_vector() {
    let key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f,
    ];
    let plaintext = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];
    let ciphertext = [
        0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5,
        0x5a,
    ];
    let cipher = crypto::aes::Aes128::new(&key);

    assert_eq!(cipher.encrypt_block(&plaintext), ciphertext);
    assert_eq!(cipher.decrypt_block(&ciphertext), plaintext);
}

#[cfg(feature = "aes")]
#[test]
fn aes256_matches_nist_vector() {
    let key = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
        0x1e, 0x1f,
    ];
    let plaintext = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];
    let ciphertext = [
        0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49, 0x60,
        0x89,
    ];

    assert_eq!(
        crypto::aes::Aes256::new(&key).encrypt_block(&plaintext),
        ciphertext
    );
}

#[cfg(feature = "des")]
#[test]
fn method2_padding_roundtrips() {
    let padded = crypto::des::iso9797_method2_pad(b"abc", 8);
    assert_eq!(padded, vec![b'a', b'b', b'c', 0x80, 0, 0, 0, 0]);
    assert_eq!(crypto::des::iso9797_method2_unpad(&padded).unwrap(), b"abc");
}

#[cfg(feature = "des")]
#[test]
fn tdes2_cbc_roundtrips() {
    let key = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        0x01,
    ];
    let cipher = crypto::des::Tdes2::new(&key);
    let iv = [0u8; 8];
    let plaintext = b"12345678ABCDEFGH";
    let encrypted = cipher.cbc_encrypt(&iv, plaintext).unwrap();
    assert_ne!(encrypted, plaintext);
    assert_eq!(cipher.cbc_decrypt(&iv, &encrypted).unwrap(), plaintext);
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::new();
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[test]
fn sha_known_vectors() {
    assert_eq!(
        hex(&crypto::sha::sha1(b"abc")),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(
        hex(&crypto::sha::sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        hex(&crypto::sha::sha384(b"abc")),
        "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded163\
         1a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
            .replace(' ', "")
    );
    assert_eq!(
        hex(&crypto::sha::sha512(b"abc")),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea2\
         0a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd\
         454d4423643ce80e2a9ac94fa54ca49f"
            .replace(' ', "")
    );
}

fn fixed_source(out: &mut [u8]) -> crypto::error::Result<()> {
    out.fill(0xA5);
    Ok(())
}

#[test]
fn registered_source_takes_priority() {
    crypto::rng::register_random_source(fixed_source);
    let mut bytes = [0u8; 16];
    crypto::rng::fill_random(&mut bytes).unwrap();
    crypto::rng::unregister_random_source();
    assert_eq!(bytes, [0xA5; 16]);
}

#[test]
fn os_rng_fills_bytes() {
    let mut rng = crypto::rng::OsRng;
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    assert!(bytes.iter().any(|b| *b != 0));
}

#[cfg(feature = "aead")]
#[test]
fn aes128_gcm_matches_nist_vector() {
    use crypto::aead::{Aead, KeyInit};

    let key = [0u8; 16];
    let nonce = crypto::aead::Nonce::from([0u8; 12]);
    let plaintext = [0u8; 16];
    let expected_ciphertext = [
        0x03, 0x88, 0xda, 0xce, 0x60, 0xb6, 0xa3, 0x92, 0xf3, 0x28, 0xc2, 0xb9, 0x71, 0xb2, 0xfe,
        0x78, 0xab, 0x6e, 0x47, 0xd4, 0x2c, 0xec, 0x13, 0xbd, 0xf5, 0x3a, 0x67, 0xb2, 0x12, 0x57,
        0xbd, 0xdf,
    ];
    let cipher = crypto::aead::Aes128Gcm::new_from_slice(&key).unwrap();
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_slice()).unwrap();
    assert_eq!(ciphertext, expected_ciphertext);
    assert_eq!(
        cipher.decrypt(&nonce, ciphertext.as_slice()).unwrap(),
        plaintext
    );
}

#[cfg(feature = "aead")]
#[test]
fn rejects_modified_tag() {
    use crypto::aead::{Aead, KeyInit};

    let key = [7u8; 32];
    let nonce = crypto::aead::Nonce::from([9u8; 12]);
    let cipher = crypto::aead::Aes256Gcm::new_from_slice(&key).unwrap();
    let mut ciphertext = cipher.encrypt(&nonce, b"message".as_slice()).unwrap();
    let last = ciphertext.len() - 1;
    ciphertext[last] ^= 1;
    assert!(cipher.decrypt(&nonce, ciphertext.as_slice()).is_err());
}

#[cfg(feature = "aead")]
#[test]
fn aes256_gcm_sealed_bytes_roundtrip() {
    let key = [7u8; 32];
    let aad = b"edgerun:test:seal";
    let sealed =
        crypto::sealing::seal_aes256_gcm_with_nonce(&key, aad, b"trust container bytes", [9u8; 12])
            .unwrap();
    let encoded = sealed.to_bytes();
    let decoded = crypto::sealing::SealedBytes::from_bytes(&encoded).unwrap();

    assert_eq!(
        crypto::sealing::unseal_aes256_gcm(&key, aad, &decoded).unwrap(),
        b"trust container bytes"
    );
}

#[cfg(feature = "aead")]
#[test]
fn aes256_gcm_rejects_tampering() {
    let key = [7u8; 32];
    let mut sealed =
        crypto::sealing::seal_aes256_gcm_with_nonce(&key, b"aad", b"sealed payload", [9u8; 12])
            .unwrap();
    sealed.ciphertext[0] ^= 1;

    assert_eq!(
        crypto::sealing::unseal_aes256_gcm(&key, b"aad", &sealed),
        Err(crypto::error::CryptoError::DecryptionFailed)
    );
}

#[cfg(feature = "ed25519")]
#[test]
fn ed25519_signing_api_roundtrips_through_verification_api() {
    let key = crypto::Ed25519SigningKey::from_bytes(&[11u8; 32]);
    let public_key = crypto::signing::ed25519_public_key(&key);
    let signature = crypto::signing::ed25519_sign(&key, b"edge signed");

    crypto::verification::ed25519_verify(&public_key, b"edge signed", &signature).unwrap();
}

#[cfg(feature = "x25519")]
#[test]
fn shared_secret_agrees_from_both_sides() {
    let alice = crypto::x25519::StaticSecret::from([1u8; 32]);
    let bob = crypto::x25519::StaticSecret::from([2u8; 32]);
    let alice_public = crypto::x25519::PublicKey::from(&alice);
    let bob_public = crypto::x25519::PublicKey::from(&bob);
    assert_eq!(
        alice.diffie_hellman(&bob_public).to_bytes(),
        bob.diffie_hellman(&alice_public).to_bytes()
    );
}

#[cfg(feature = "x25519")]
#[test]
fn x25519_matches_rfc7748_vector() {
    let scalar = [
        0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66,
        0x45, 0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9,
        0x2c, 0x2a,
    ];
    let expected = [
        0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7,
        0x5a, 0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b,
        0x4e, 0x6a,
    ];
    let mut point = [0u8; 32];
    point[0] = 9;
    assert_eq!(crypto::x25519::x25519(scalar, point), expected);
}
