//! EncryptedEnvelope — E2E encryption with ECDH P-256 key exchange.
//!
//! ALL private data MUST be:
//! - encrypted using ECDH P-256 key agreement + AEAD payload encryption
//! - ephemeral key per message (forward secrecy)
//! - recipient-bound with per-recipient wrapped message keys
//! - signature-protected
//!
//! Encryption model:
//!   1. Generate ephemeral ECDH key pair
//!   2. Generate random message key K_msg
//!   3. For each recipient: K_wrapped = AEAD(ECDH(shared_secret), K_msg)
//!   4. Ciphertext = AEAD(K_msg, plaintext)
//!   5. Sign: sender || recipients || ephemeral_pub || nonce || ciphertext
//!
//! Domain separation: "edgerun:v0:hash:encrypted-envelope"

use crate::prelude::v1::*;
use edgerun_crypto::p256::ecdh::EphemeralSecret;
use edgerun_crypto::p256::elliptic_curve::sec1::{EncodedPoint, ToEncodedPoint};
use edgerun_crypto::p256::PublicKey;
use edgerun_crypto::aes_gcm;
use edgerun_crypto::{Aead, AesGcmCipher, KeyInit, OsRng};
use edgerun_proto::edgerun::v0::common::{
    CipherSuite, EncryptedEnvelope, RecipientKey,
};
use prost::Message;

/// Domain string for encrypted envelope signature canonicalization.
pub const ENCRYPTED_ENVELOPE_DOMAIN: &[u8] = b"edgerun:v0:hash:encrypted-envelope";

/// Validate an EncryptedEnvelope meets protocol invariants.
pub fn validate_encrypted_envelope(env: &EncryptedEnvelope) -> Result<(), &'static str> {
    if env.recipients.is_empty() {
        return Err("NO_RECIPIENT");
    }

    for rk in &env.recipients {
        if rk.identity.is_empty() {
            return Err("EMPTY_RECIPIENT_IDENTITY");
        }
        if rk.encrypted_key.is_empty() {
            return Err("EMPTY_ENCRYPTED_KEY");
        }
    }

    if env.ephemeral_pubkey.is_empty() {
        return Err("EMPTY_EPHEMERAL_PUBKEY");
    }

    if env.ciphertext.is_empty() {
        return Err("EMPTY_CIPHERTEXT");
    }

    if env.nonce.is_empty() {
        return Err("EMPTY_NONCE");
    }

    let expected_nonce_len = match CipherSuite::from_i32(env.cipher) {
        Some(CipherSuite::Xchacha20Poly1305) => 24,
        Some(CipherSuite::Aes256Gcm) => 12,
        _ => return Err("INVALID_CIPHER"),
    };

    if env.nonce.len() != expected_nonce_len {
        return Err("WRONG_NONCE_LENGTH");
    }

    if env.sender.is_empty() {
        return Err("EMPTY_SENDER");
    }

    Ok(())
}

/// Check if a cipher suite value is valid for encrypted envelopes.
pub fn is_valid_cipher(cipher: i32) -> bool {
    matches!(
        CipherSuite::from_i32(cipher),
        Some(CipherSuite::Xchacha20Poly1305) | Some(CipherSuite::Aes256Gcm)
    )
}

/// Build canonical bytes for signature:
/// sender || recipients (each: identity || encrypted_key || key_nonce) || ephemeral_pub || nonce || ciphertext
pub fn canonical_envelope_bytes(env: &EncryptedEnvelope) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&env.sender);
    for rk in &env.recipients {
        out.extend_from_slice(&rk.identity);
        out.extend_from_slice(&rk.encrypted_key);
        out.extend_from_slice(&rk.key_nonce);
    }
    out.extend_from_slice(&env.ephemeral_pubkey);
    out.extend_from_slice(&env.nonce);
    out.extend_from_slice(&env.ciphertext);
    out
}

/// ECDH key exchange: compute shared secret from ephemeral secret + recipient public key.
fn ecdh_shared_secret(
    ephemeral_secret: &EphemeralSecret,
    recipient_pubkey: &[u8],
) -> Result<Vec<u8>, &'static str> {
    let pk = PublicKey::from_sec1_bytes(recipient_pubkey)
        .map_err(|_| "INVALID_RECIPIENT_PUBKEY")?;
    let shared = ephemeral_secret.diffie_hellman(&pk);
    Ok(shared.raw_secret_bytes().to_vec())
}

/// Derive a 256-bit key from ECDH shared secret using HKDF-SHA256.
fn derive_key(shared_secret: &[u8], info: &[u8]) -> [u8; 32] {
    let key = edgerun_crypto::hkdf_sha256(None, shared_secret, info, 32);
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&key);
    arr
}

/// Encrypt a payload for multiple recipients using ECDH + AEAD.
///
/// Returns an EncryptedEnvelope with:
/// - ephemeral ECDH public key
/// - per-recipient wrapped message keys
/// - AEAD-encrypted ciphertext
pub fn encrypt_envelope(
    sender_pubkey: &[u8],
    sender_sign_key: &dyn Fn(&[u8]) -> Vec<u8>,
    recipient_pubkeys: &[Vec<u8>],
    plaintext: &[u8],
    cipher: CipherSuite,
) -> Result<EncryptedEnvelope, &'static str> {
    if recipient_pubkeys.is_empty() {
        return Err("NO_RECIPIENT");
    }

    // Step 1: Generate ephemeral ECDH key pair
    let ephemeral_secret = EphemeralSecret::random(&mut edgerun_crypto::OsRng);
    let ephemeral_pub = ephemeral_secret.public_key();
    let ephemeral_pubkey_bytes = ephemeral_pub.to_encoded_point(false).as_bytes().to_vec();

    // Step 2: Generate random message key
    let mut k_msg = [0u8; 32];
    edgerun_crypto::fill_random(&mut k_msg).map_err(|_| "RNG_FAILURE")?;

    // Step 3: For each recipient, compute ECDH shared secret + wrap message key
    let mut recipient_keys = Vec::new();
    for recipient_pub in recipient_pubkeys {
        let shared = ecdh_shared_secret(&ephemeral_secret, recipient_pub)?;
        let k_recipient = derive_key(&shared, b"edgerun:v0:ecdh:recipient-key");

        // Wrap K_msg with AES-GCM using K_recipient
        let cipher = AesGcmCipher::new_from_slice(&k_recipient)
            .map_err(|_| "AES_KEY_INIT")?;
        let mut key_nonce = [0u8; 12];
        edgerun_crypto::fill_random(&mut key_nonce).map_err(|_| "RNG_FAILURE")?;
        let nonce = aes_gcm::Nonce::from_slice(&key_nonce);
        let encrypted_key = cipher
            .encrypt(nonce, k_msg.as_ref())
            .map_err(|_| "KEY_WRAP")?;

        recipient_keys.push(RecipientKey {
            identity: recipient_pub.clone(),
            encrypted_key,
            key_nonce: nonce.to_vec(),
        });
    }

    // Step 4: Encrypt payload with K_msg
    let payload_cipher = AesGcmCipher::new_from_slice(&k_msg)
        .map_err(|_| "AES_KEY_INIT")?;
    let mut nonce_bytes = [0u8; 12];
    edgerun_crypto::fill_random(&mut nonce_bytes).map_err(|_| "RNG_FAILURE")?;
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    let ciphertext = payload_cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| "ENCRYPT_PAYLOAD")?;

    // Step 5: Build envelope and sign
    let env = EncryptedEnvelope {
        version: 1,
        sender: sender_pubkey.to_vec(),
        recipients: recipient_keys,
        ephemeral_pubkey: ephemeral_pubkey_bytes,
        cipher: cipher as i32,
        nonce: nonce.to_vec(),
        ciphertext,
        tag: Vec::new(),
        signature: Vec::new(),
    };

    let canonical = canonical_envelope_bytes(&env);
    let signature = sender_sign_key(&canonical);

    Ok(EncryptedEnvelope { signature, ..env })
}

/// Decrypt an EncryptedEnvelope using the recipient's private key.
pub fn decrypt_envelope(
    env: &EncryptedEnvelope,
    recipient_privkey: &EphemeralSecret,
) -> Result<Vec<u8>, &'static str> {
    validate_encrypted_envelope(env)?;

    // Find our recipientKey entry
    let our_entry = env.recipients.iter().find(|rk| {
        // Match by checking if the identity in the envelope corresponds to our public key
        let our_pub = recipient_privkey.public_key();
        let our_bytes = our_pub.to_encoded_point(false).as_bytes().to_vec();
        rk.identity == our_bytes
    });

    let our_entry = our_entry.ok_or("NOT_A_RECIPIENT")?;

    // Compute ECDH shared secret
    let shared = ecdh_shared_secret(recipient_privkey, &env.ephemeral_pubkey)?;
    let k_recipient = derive_key(&shared, b"edgerun:v0:ecdh:recipient-key");

    // Unwrap message key
    let cipher = AesGcmCipher::new_from_slice(&k_recipient)
        .map_err(|_| "AES_KEY_INIT")?;
    let nonce = aes_gcm::Nonce::from_slice(&our_entry.key_nonce);
    let k_msg = cipher
        .decrypt(nonce, our_entry.encrypted_key.as_ref())
        .map_err(|_| "KEY_UNWRAP")?;

    if k_msg.len() != 32 {
        return Err("WRONG_KEY_LENGTH");
    }

    // Decrypt payload
    let payload_cipher = AesGcmCipher::new_from_slice(&k_msg)
        .map_err(|_| "AES_KEY_INIT")?;
    let nonce = aes_gcm::Nonce::from_slice(&env.nonce);
    let plaintext = payload_cipher
        .decrypt(nonce, env.ciphertext.as_ref())
        .map_err(|_| "DECRYPT_PAYLOAD")?;

    Ok(plaintext)
}

/// Check if raw bytes look like a valid EncryptedEnvelope protobuf.
pub fn looks_like_encrypted_envelope(data: &[u8]) -> bool {
    if data.len() < 10 {
        return false;
    }
    EncryptedEnvelope::decode(data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::p256::ecdsa::SigningKey;
    use edgerun_crypto::signature::Signer;

    fn valid_envelope() -> EncryptedEnvelope {
        EncryptedEnvelope {
            version: 1,
            sender: vec![1u8; 65],
            recipients: vec![RecipientKey {
                identity: vec![2u8; 65],
                encrypted_key: vec![3u8; 48],
                key_nonce: vec![4u8; 12],
            }],
            ephemeral_pubkey: vec![5u8; 65],
            cipher: CipherSuite::Aes256Gcm as i32,
            nonce: vec![0xCAu8; 12],
            ciphertext: vec![0xBBu8; 100],
            tag: Vec::new(),
            signature: vec![6u8; 64],
        }
    }

    #[test]
    fn valid_envelope_passes() {
        let env = valid_envelope();
        assert!(validate_encrypted_envelope(&env).is_ok());
    }

    #[test]
    fn empty_recipients_rejected() {
        let mut env = valid_envelope();
        env.recipients.clear();
        assert_eq!(validate_encrypted_envelope(&env), Err("NO_RECIPIENT"));
    }

    #[test]
    fn empty_ephemeral_pubkey_rejected() {
        let mut env = valid_envelope();
        env.ephemeral_pubkey.clear();
        assert_eq!(validate_encrypted_envelope(&env), Err("EMPTY_EPHEMERAL_PUBKEY"));
    }

    #[test]
    fn unspecified_cipher_rejected() {
        let mut env = valid_envelope();
        env.cipher = CipherSuite::Unspecified as i32;
        assert_eq!(validate_encrypted_envelope(&env), Err("INVALID_CIPHER"));
    }

    #[test]
    fn empty_nonce_rejected() {
        let mut env = valid_envelope();
        env.nonce.clear();
        assert_eq!(validate_encrypted_envelope(&env), Err("EMPTY_NONCE"));
    }

    #[test]
    fn wrong_nonce_length_rejected() {
        let mut env = valid_envelope();
        env.nonce = vec![0u8; 24]; // AES-GCM needs 12
        assert_eq!(validate_encrypted_envelope(&env), Err("WRONG_NONCE_LENGTH"));
    }

    #[test]
    fn empty_ciphertext_rejected() {
        let mut env = valid_envelope();
        env.ciphertext.clear();
        assert_eq!(validate_encrypted_envelope(&env), Err("EMPTY_CIPHERTEXT"));
    }

    #[test]
    fn empty_sender_rejected() {
        let mut env = valid_envelope();
        env.sender.clear();
        assert_eq!(validate_encrypted_envelope(&env), Err("EMPTY_SENDER"));
    }

    #[test]
    fn xchacha20_accepts_24_byte_nonce() {
        let mut env = valid_envelope();
        env.cipher = CipherSuite::Xchacha20Poly1305 as i32;
        env.nonce = vec![0u8; 24];
        assert!(validate_encrypted_envelope(&env).is_ok());
    }

    #[test]
    fn canonical_bytes_include_all_fields() {
        let env = valid_envelope();
        let canonical = canonical_envelope_bytes(&env);
        // sender (65) + 1 recipient (65+48+12) + ephemeral (65) + nonce (12) + ciphertext (100)
        assert_eq!(canonical.len(), 65 + 125 + 65 + 12 + 100);
    }

    #[test]
    fn canonical_is_deterministic() {
        let original = valid_envelope();
        let bytes1 = canonical_envelope_bytes(&original);
        let bytes2 = canonical_envelope_bytes(&original);
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn looks_like_envelope_accepts_valid() {
        let env = valid_envelope();
        let data = prost::Message::encode_to_vec(&env);
        assert!(looks_like_encrypted_envelope(&data));
    }

    #[test]
    fn looks_like_envelope_rejects_garbage() {
        assert!(!looks_like_encrypted_envelope(b"hello plaintext"));
        assert!(!looks_like_encrypted_envelope(&[]));
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        // Generate recipient key pair
        let recipient_sk = EphemeralSecret::random(&mut edgerun_crypto::OsRng);
        let recipient_pk = recipient_sk.public_key();
        let recipient_pk_bytes = recipient_pk.to_encoded_point(false).as_bytes().to_vec();

        // Generate sender key pair for signing
        let sender_sk = SigningKey::random(&mut edgerun_crypto::OsRng);
        let sender_pk = sender_sk.verifying_key();
        let sender_pk_bytes = sender_pk.to_encoded_point(false).as_bytes().to_vec();

        // Signer closure
        use edgerun_crypto::signature::Signer;
        use edgerun_crypto::ecdsa::Signature;
        let signer = |data: &[u8]| -> Vec<u8> {
            let sig: Signature = sender_sk.sign(data);
            sig.to_bytes().to_vec()
        };

        // Encrypt
        let plaintext = b"hello secret world";
        let envelope = encrypt_envelope(
            &sender_pk_bytes,
            &signer,
            &[recipient_pk_bytes.clone()],
            plaintext,
            CipherSuite::Aes256Gcm,
        )
        .unwrap();

        assert!(!envelope.ciphertext.is_empty());
        assert_eq!(envelope.recipients.len(), 1);

        // Decrypt
        let decrypted = decrypt_envelope(&envelope, &recipient_sk).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let recipient_sk = EphemeralSecret::random(&mut edgerun_crypto::OsRng);
        let recipient_pk = recipient_sk.public_key();
        let recipient_pk_bytes = recipient_pk.to_encoded_point(false).as_bytes().to_vec();

        let sender_sk = SigningKey::random(&mut edgerun_crypto::OsRng);
        let sender_pk = sender_sk.verifying_key();
        let sender_pk_bytes = sender_pk.to_encoded_point(false).as_bytes().to_vec();

        use edgerun_crypto::signature::Signer;
        let signer = |data: &[u8]| -> Vec<u8> {
            <SigningKey as Signer<Vec<u8>>>::sign(&sender_sk, data).to_vec()
        };

        let envelope = encrypt_envelope(
            &sender_pk_bytes,
            &signer,
            &[recipient_pk_bytes],
            b"secret",
            CipherSuite::Aes256Gcm,
        )
        .unwrap();

        // Wrong recipient key
        let wrong_sk = EphemeralSecret::random(&mut edgerun_crypto::OsRng);
        let result = decrypt_envelope(&envelope, &wrong_sk);
        assert!(result.is_err());
    }
}
