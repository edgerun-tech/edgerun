//! TLS 1.3 server handshake message builders.

use edgerun_crypto::p256::ecdsa::{Signature, SigningKey, signature::SignerMut};
use edgerun_crypto::sha2::{Digest, Sha256, Sha384};

use crate::cipher::{CipherSuite, NamedGroup};
use crate::prf::{Hasher, hmac_sha256, hmac_sha384};
use crate::Result;

/// Build a ServerHello handshake message (RFC 8446 §4.1.3).
pub fn build_server_hello(
    random: [u8; 32],
    session_id: &[u8],
    cipher_suite: CipherSuite,
    server_key_share: &[u8],
    group: NamedGroup,
) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(2);
    msg.extend_from_slice(&[0u8; 3]);
    msg.extend_from_slice(&0x0303u16.to_be_bytes());
    msg.extend_from_slice(&random);
    msg.push(session_id.len() as u8);
    msg.extend_from_slice(session_id);
    msg.extend_from_slice(&cipher_suite.to_wire().to_be_bytes());
    msg.push(0);

    let ext_start = msg.len();
    msg.extend_from_slice(&[0u8; 2]);

    {
        let data = vec![0x03, 0x04];
        msg.extend_from_slice(&43u16.to_be_bytes());
        msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
        msg.extend_from_slice(&data);
    }

    {
        let mut data = Vec::new();
        data.extend_from_slice(&group.to_wire().to_be_bytes());
        data.extend_from_slice(&(server_key_share.len() as u16).to_be_bytes());
        data.extend_from_slice(server_key_share);
        msg.extend_from_slice(&51u16.to_be_bytes());
        msg.extend_from_slice(&(data.len() as u16).to_be_bytes());
        msg.extend_from_slice(&data);
    }

    let ext_len = (msg.len() - ext_start - 2) as u16;
    msg[ext_start..ext_start + 2].copy_from_slice(&ext_len.to_be_bytes());

    let msg_len = (msg.len() - 4) as u32;
    msg[1..4].copy_from_slice(&msg_len.to_be_bytes()[1..]);

    msg
}

/// Build an EncryptedExtensions handshake message.
pub fn build_encrypted_extensions() -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(8);
    msg.extend_from_slice(&[0u8; 3]);
    msg.extend_from_slice(&[0u8; 2]);
    msg
}

/// Build a Certificate message (TLS 1.3 format).
pub fn build_certificate_message(cert_der: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(11);

    let cert_list_len = 3 + cert_der.len();
    let total_len = 1 + 3 + cert_list_len;

    msg.extend_from_slice(&(total_len as u32).to_be_bytes()[1..]);
    msg.push(0);
    msg.extend_from_slice(&(cert_list_len as u32).to_be_bytes()[1..]);
    msg.extend_from_slice(cert_der);
    msg.extend_from_slice(&[0u8; 2]);

    msg
}

/// Build a CertificateVerify message (RFC 8446 §4.4.3).
pub fn build_certificate_verify(
    transcript: &[u8],
    signing_key: &SigningKey,
    hasher: &Hasher,
) -> Result<Vec<u8>> {
    let context = b"TLS 1.3, server CertificateVerify";
    let mut padded = vec![0x20u8; 64];
    padded.extend_from_slice(context);
    padded.push(0x00);
    padded.extend_from_slice(transcript);

    let digest = match hasher {
        Hasher::Sha256 => Sha256::digest(&padded).to_vec(),
        Hasher::Sha384 => Sha384::digest(&padded).to_vec(),
    };

    let mut signer = signing_key.clone();
    let signature: Signature = <SigningKey as SignerMut<Signature>>::sign(&mut signer, &digest);
    let sig_der_bytes = signature.to_bytes().to_vec();

    let mut msg = Vec::new();
    msg.push(15);

    let inner_len = 2 + 2 + sig_der_bytes.len();
    msg.extend_from_slice(&((4 + inner_len) as u32).to_be_bytes()[1..]);

    msg.extend_from_slice(&0x0403u16.to_be_bytes());
    msg.extend_from_slice(&(sig_der_bytes.len() as u16).to_be_bytes());
    msg.extend_from_slice(&sig_der_bytes);

    Ok(msg)
}

/// Build a Finished message (RFC 8446 §4.4.4).
pub fn build_finished_message(verify_data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(20);
    msg.extend_from_slice(&((verify_data.len()) as u32).to_be_bytes()[1..]);
    msg.extend_from_slice(verify_data);
    msg
}

/// Compute the Finished verify_data for the server side.
pub fn compute_server_finished_verify_data(
    server_hs_secret: &[u8],
    transcript_hash: &[u8],
    hasher: &Hasher,
) -> Vec<u8> {
    let finished_key = hasher.expand_label(server_hs_secret, "finished", &[], hasher.len());
    match hasher {
        Hasher::Sha256 => hmac_sha256(&finished_key, transcript_hash),
        Hasher::Sha384 => hmac_sha384(&finished_key, transcript_hash),
    }
}

/// Compute the expected Finished verify_data for the client side.
pub fn compute_client_finished_verify_data(
    client_hs_secret: &[u8],
    transcript_hash: &[u8],
    hasher: &Hasher,
) -> Vec<u8> {
    let finished_key = hasher.expand_label(client_hs_secret, "finished", &[], hasher.len());
    match hasher {
        Hasher::Sha256 => hmac_sha256(&finished_key, transcript_hash),
        Hasher::Sha384 => hmac_sha384(&finished_key, transcript_hash),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::certificate_gen::generate_self_signed;
    use crate::handshake::ClientHelloBuilder;
    use crate::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
    use crate::prf::{Tls13KeySchedule, server_write_keys};
    use crate::record::RecordCipher;

    #[test]
    fn test_server_hello_roundtrip() {
        let random = [0xCDu8; 32];
        let key_pair = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let public_key = key_pair.public_key_bytes();

        let sh_bytes = build_server_hello(
            random,
            &[],
            CipherSuite::TLS_AES_128_GCM_SHA256,
            &public_key,
            NamedGroup::SECP256R1,
        );

        assert_eq!(sh_bytes[0], 2);
        let sh = crate::handshake::ServerHello::parse(&sh_bytes).unwrap();
        assert_eq!(sh.random, random);
        assert_eq!(sh.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
        assert_eq!(sh.server_key_share.len(), 65);
        assert_eq!(sh.supported_version, Some(0x0304));
    }

    #[test]
    fn test_encrypted_extensions_build() {
        let ee_bytes = build_encrypted_extensions();
        assert_eq!(ee_bytes[0], 8);
        assert_eq!(ee_bytes.len(), 6);
    }

    #[test]
    fn test_certificate_message_build() {
        let cert = generate_self_signed(&["localhost"]);
        let cert_msg = build_certificate_message(&cert.cert_der);
        assert_eq!(cert_msg[0], 11);
        assert!(cert_msg.len() > cert.cert_der.len());
    }

    #[test]
    fn test_certificate_verify_build() {
        let cert = generate_self_signed(&["localhost"]);
        let transcript = vec![0x01u8; 64];

        let cv_bytes = build_certificate_verify(&transcript, &cert.signing_key, &Hasher::Sha256).unwrap();
        assert_eq!(cv_bytes[0], 15);
        assert!(cv_bytes.len() > 4);
        assert_eq!(&cv_bytes[4..6], &[0x04, 0x03]);
    }

    #[test]
    fn test_finished_message_build() {
        let verify_data = vec![0xDEu8; 32];
        let finished = build_finished_message(&verify_data);
        assert_eq!(finished[0], 20);
        assert_eq!(finished.len(), 4 + verify_data.len());
    }

    #[test]
    fn test_compute_server_finished_verify_data() {
        let server_hs_secret = vec![0x42u8; 32];
        let transcript_hash = vec![0x55u8; 32];
        let hasher = Hasher::Sha256;

        let verify_data = compute_server_finished_verify_data(&server_hs_secret, &transcript_hash, &hasher);
        assert_eq!(verify_data.len(), 32);

        let verify_data2 = compute_server_finished_verify_data(&server_hs_secret, &transcript_hash, &hasher);
        assert_eq!(verify_data, verify_data2);

        let different_secret = vec![0xFFu8; 32];
        let verify_data3 = compute_server_finished_verify_data(&different_secret, &transcript_hash, &hasher);
        assert_ne!(verify_data, verify_data3);
    }

    #[test]
    fn test_server_hello_encrypt_decrypt_roundtrip() {
        let cert = generate_self_signed(&["localhost"]);
        let client_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let server_keys = EcdhKeyPair::generate(KeyExchangeGroup::SECP256R1).unwrap();
        let client_random = [0xCCu8; 32];
        let cipher_suite = CipherSuite::TLS_AES_128_GCM_SHA256;

        let client_pub = client_keys.public_key_bytes();
        let ch_bytes = ClientHelloBuilder::new(client_random, "localhost")
            .key_share(&client_pub, NamedGroup::SECP256R1)
            .build()
            .unwrap();

        let server_pub = server_keys.public_key_bytes();
        let sh_bytes = build_server_hello(
            [0xDDu8; 32],
            &[],
            cipher_suite,
            &server_pub,
            NamedGroup::SECP256R1,
        );

        let shared = server_keys.exchange(&client_pub).unwrap();
        let hash = Hasher::Sha256;
        let ch_hash = hash.hash(&ch_bytes);
        let sh_hash = hash.hash(&sh_bytes);

        let mut ks = Tls13KeySchedule::new(hash.clone());
        ks.advance_to_handshake(&shared, &ch_hash, &sh_hash);

        let server_hs_secret = ks.server_handshake_traffic_secret(&ch_hash);
        let server_write = server_write_keys(&server_hs_secret, cipher_suite.key_len(), 12, &hash);

        let mut write_cipher = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();

        let ee_bytes = build_encrypted_extensions();
        let ee_encrypted = write_cipher.encrypt(22, &ee_bytes);

        let mut read_cipher_ee = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let (ct, plaintext) = read_cipher_ee.decrypt(&ee_encrypted).unwrap();
        assert_eq!(ct, 22);
        assert_eq!(plaintext, ee_bytes);

        let cert_msg = build_certificate_message(&cert.cert_der);
        let cert_encrypted = write_cipher.encrypt(22, &cert_msg);

        let mut read_cipher_cert = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = read_cipher_cert.decrypt(&ee_encrypted).unwrap();
        let (_ct2, cert_decrypted) = read_cipher_cert.decrypt(&cert_encrypted).unwrap();
        assert_eq!(cert_decrypted, cert_msg);

        let transcript = vec![0xEEu8; 64];
        let cv_bytes = build_certificate_verify(&transcript, &cert.signing_key, &hash).unwrap();
        let cv_encrypted = write_cipher.encrypt(22, &cv_bytes);

        let mut read_cipher_cv = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = read_cipher_cv.decrypt(&ee_encrypted).unwrap();
        let _ = read_cipher_cv.decrypt(&cert_encrypted).unwrap();
        let (_ct3, cv_decrypted) = read_cipher_cv.decrypt(&cv_encrypted).unwrap();
        assert_eq!(cv_decrypted, cv_bytes);

        let transcript_hash = hash.hash(&transcript);
        let verify_data = compute_server_finished_verify_data(&server_hs_secret, &transcript_hash, &hash);
        let finished_msg = build_finished_message(&verify_data);
        let finished_encrypted = write_cipher.encrypt(22, &finished_msg);

        let mut read_cipher_fin = RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = read_cipher_fin.decrypt(&ee_encrypted).unwrap();
        let _ = read_cipher_fin.decrypt(&cert_encrypted).unwrap();
        let _ = read_cipher_fin.decrypt(&cv_encrypted).unwrap();
        let (_ct4, finished_decrypted) = read_cipher_fin.decrypt(&finished_encrypted).unwrap();
        assert_eq!(finished_decrypted, finished_msg);
    }
}
