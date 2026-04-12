#[cfg(test)]
use super::*;
mod tests {
    use super::*;

    // -- SHA-256 --

    #[test]
    fn sha256_empty() {
        let h = sha256(b"");
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb924...
        assert_eq!(
            &h[..8],
            &[0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14]
        );
    }

    #[test]
    fn sha256_known() {
        let h = sha256(b"hello");
        // SHA-256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c...
        assert_eq!(
            &h[..8],
            &[0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e]
        );
    }

    #[test]
    fn sha256_output_length() {
        assert_eq!(sha256(b"anything").len(), 32);
    }

    // -- SHA-384 --

    #[test]
    fn sha384_empty() {
        let h = sha384(b"");
        // SHA-384("") = 38b060a751ac96384cd9327eb1b1e36a...
        assert_eq!(
            &h[..8],
            &[0x38, 0xb0, 0x60, 0xa7, 0x51, 0xac, 0x96, 0x38]
        );
    }

    #[test]
    fn sha384_output_length() {
        assert_eq!(sha384(b"hello").len(), 48);
    }

    // -- SHA-512 --

    #[test]
    fn sha512_empty() {
        let h = sha512(b"");
        // SHA-512("") = cf83e1357eefb8bd...
        assert_eq!(
            &h[..8],
            &[0xcf, 0x83, 0xe1, 0x35, 0x7e, 0xef, 0xb8, 0xbd]
        );
    }

    #[test]
    fn sha512_output_length() {
        assert_eq!(sha512(b"hello").len(), 64);
    }

    // -- HMAC-SHA-256 --

    #[test]
    fn hmac_sha256_known() {
        // RFC 4231 test case 1: key = 0x0b repeated 20 times, data = "Hi There"
        let key = vec![0x0b; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            &mac[..8],
            &[0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53]
        );
    }

    #[test]
    fn hmac_sha256_different_key_different_output() {
        let mac1 = hmac_sha256(b"key1", b"data");
        let mac2 = hmac_sha256(b"key2", b"data");
        assert_ne!(mac1, mac2);
    }

    #[test]
    fn hmac_sha256_output_length() {
        assert_eq!(hmac_sha256(b"key", b"data").len(), 32);
    }

    // -- HMAC-SHA-384 --

    #[test]
    fn hmac_sha384_output_length() {
        assert_eq!(hmac_sha384(b"key", b"data").len(), 48);
    }

    #[test]
    fn hmac_sha384_different_key_different_output() {
        let mac1 = hmac_sha384(b"key1", b"data");
        let mac2 = hmac_sha384(b"key2", b"data");
        assert_ne!(mac1, mac2);
    }

    // -- HKDF-SHA-256 --

    #[test]
    fn hkdf_sha256_basic() {
        // Test that HKDF produces deterministic output
        let okm1 = hkdf_sha256(Some(b"salt"), b"ikm", b"info", 32);
        let okm2 = hkdf_sha256(Some(b"salt"), b"ikm", b"info", 32);
        assert_eq!(okm1, okm2);
    }

    #[test]
    fn hkdf_sha256_different_length() {
        let okm16 = hkdf_sha256(None, b"secret", b"context", 16);
        let okm64 = hkdf_sha256(None, b"secret", b"context", 64);
        assert_eq!(okm16.len(), 16);
        assert_eq!(okm64.len(), 64);
        // First 16 bytes should be the same prefix
        assert_eq!(okm16, okm64[..16]);
    }

    #[test]
    fn hkdf_sha256_different_salt_different_output() {
        let okm1 = hkdf_sha256(Some(b"salt1"), b"ikm", b"info", 32);
        let okm2 = hkdf_sha256(Some(b"salt2"), b"ikm", b"info", 32);
        assert_ne!(okm1, okm2);
    }

    #[test]
    fn hkdf_sha256_no_salt() {
        let okm = hkdf_sha256(None, b"ikm", b"info", 32);
        assert_eq!(okm.len(), 32);
    }

    // -- Random P-256 signing key --

    #[test]
    fn random_p256_signing_key_generates() {
        let key = random_p256_signing_key();
        // Should produce a valid verifying key
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false); // uncompressed
        assert_eq!(encoded.as_bytes().len(), 65); // 1 byte prefix + 32 + 32
    }

    #[test]
    fn random_p256_signing_key_unique() {
        let k1 = random_p256_signing_key();
        let k2 = random_p256_signing_key();
        // Two random keys should almost certainly differ
        assert_ne!(
            k1.to_bytes().as_slice(),
            k2.to_bytes().as_slice()
        );
    }

    #[test]
    fn random_p256_signing_key_can_sign() {
        use signature::Signer;
        let key = random_p256_signing_key();
        let msg = b"test message";
        let sig: p256::ecdsa::Signature = key.sign(msg);
        assert!(sig.to_bytes().len() > 0);
    }

    // -- AES-256-GCM encrypt/decrypt --

    #[test]
    fn aes256_gcm_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"hello world";
        let (nonce, ct) = aes256_gcm_encrypt(&key, plaintext);
        let pt = aes256_gcm_decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn aes256_gcm_empty_plaintext() {
        let key = [0x00u8; 32];
        let (nonce, ct) = aes256_gcm_encrypt(&key, b"");
        let pt = aes256_gcm_decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(pt, b"");
    }

    #[test]
    fn aes256_gcm_wrong_key_fails() {
        let key = [0x42u8; 32];
        let wrong_key = [0x99u8; 32];
        let (nonce, ct) = aes256_gcm_encrypt(&key, b"secret");
        let result = aes256_gcm_decrypt(&wrong_key, &nonce, &ct);
        assert!(result.is_err());
    }

    #[test]
    fn aes256_gcm_tampered_ciphertext_fails() {
        let key = [0x42u8; 32];
        let (nonce, mut ct) = aes256_gcm_encrypt(&key, b"secret");
        ct[0] ^= 0xFF; // flip bits
        let result = aes256_gcm_decrypt(&key, &nonce, &ct);
        assert!(result.is_err());
    }

    #[test]
    fn aes256_gcm_tampered_nonce_fails() {
        let key = [0x42u8; 32];
        let (mut nonce, ct) = aes256_gcm_encrypt(&key, b"secret");
        nonce[0] ^= 0xFF;
        let result = aes256_gcm_decrypt(&key, &nonce, &ct);
        assert!(result.is_err());
    }

    #[test]
    fn aes256_gcm_large_plaintext() {
        let key = [0xABu8; 32];
        let plaintext = vec![0xCDu8; 10000];
        let (nonce, ct) = aes256_gcm_encrypt(&key, &plaintext);
        let pt = aes256_gcm_decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    // -- AesGcmCipher unified enum --

    #[test]
    fn aes_gcm_cipher_aes128_from_slice() {
        let key = [0x00u8; 16];
        let cipher = AesGcmCipher::new_from_slice(&key).unwrap();
        let nonce = [0x01u8; 12];
        let pt = b"aes128 test";
        let ct = cipher.encrypt(&nonce, pt).unwrap();
        let dec = cipher.decrypt(&nonce, &ct).unwrap();
        assert_eq!(dec, pt);
    }

    #[test]
    fn aes_gcm_cipher_aes256_from_slice() {
        let key = [0x00u8; 32];
        let cipher = AesGcmCipher::new_from_slice(&key).unwrap();
        let nonce = [0x02u8; 12];
        let pt = b"aes256 test";
        let ct = cipher.encrypt(&nonce, pt).unwrap();
        let dec = cipher.decrypt(&nonce, &ct).unwrap();
        assert_eq!(dec, pt);
    }

    #[test]
    fn aes_gcm_cipher_invalid_key_length() {
        assert!(AesGcmCipher::new_from_slice(&[0u8; 8]).is_err());
        assert!(AesGcmCipher::new_from_slice(&[0u8; 24]).is_err());
    }

    #[test]
    fn aes_gcm_cipher_wrong_key() {
        let key1 = [0xAAu8; 32];
        let key2 = [0xBBu8; 32];
        let c1 = AesGcmCipher::new_from_slice(&key1).unwrap();
        let c2 = AesGcmCipher::new_from_slice(&key2).unwrap();
        let nonce = [0xCCu8; 12];
        let ct = c1.encrypt(&nonce, b"secret").unwrap();
        assert!(c2.decrypt(&nonce, &ct).is_err());
    }

    #[test]
    fn aes_gcm_cipher_encrypt_in_place_detached() {
        let key = [0x55u8; 32];
        let cipher = AesGcmCipher::new_from_slice(&key).unwrap();
        let nonce = [0x66u8; 12];
        let aad = b"additional data";
        let mut buffer = b"hello in-place".to_vec();
        let tag = cipher.encrypt_in_place_detached(&nonce, aad, &mut buffer).unwrap();
        cipher.decrypt_in_place_detached(&nonce, aad, &mut buffer, &tag).unwrap();
        assert_eq!(buffer, b"hello in-place");
    }

    #[test]
    fn aes_gcm_cipher_decrypt_detached_tampered_tag_fails() {
        let key = [0x77u8; 32];
        let cipher = AesGcmCipher::new_from_slice(&key).unwrap();
        let nonce = [0x88u8; 12];
        let aad = b"aad";
        let mut buffer = b"tamper test".to_vec();
        let mut tag = cipher.encrypt_in_place_detached(&nonce, aad, &mut buffer).unwrap();
        // Tamper with the tag (first byte)
        tag[0] ^= 0xFF;
        assert!(cipher.decrypt_in_place_detached(&nonce, aad, &mut buffer, &tag).is_err());
    }
}
