//! TLS 1.3 server handshake message builders.

use alloc::{vec, vec::Vec};
use edgerun_crypto::p256::ecdsa::{signature::SignerMut, Signature, SigningKey};

use super::super::cipher::NamedGroup;
use super::super::prf::{hmac_sha256, hmac_sha384, Hasher};
use super::super::Result;
use edgerun_crypto::CipherSuite;
use edgerun_encoding::byteorder::{push_u16_be, push_u24_be, write_u16_be, write_u24_be};

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
    push_u16_be(&mut msg, 0x0303);
    msg.extend_from_slice(&random);
    msg.push(session_id.len() as u8);
    msg.extend_from_slice(session_id);
    push_u16_be(&mut msg, cipher_suite.to_wire());
    msg.push(0);

    let ext_start = msg.len();
    msg.extend_from_slice(&[0u8; 2]);

    {
        let data = vec![0x03, 0x04];
        push_u16_be(&mut msg, 43);
        push_u16_be(&mut msg, data.len() as u16);
        msg.extend_from_slice(&data);
    }

    {
        let mut data = Vec::new();
        push_u16_be(&mut data, group.to_wire());
        push_u16_be(&mut data, server_key_share.len() as u16);
        data.extend_from_slice(server_key_share);
        push_u16_be(&mut msg, 51);
        push_u16_be(&mut msg, data.len() as u16);
        msg.extend_from_slice(&data);
    }

    let ext_len = (msg.len() - ext_start - 2) as u16;
    write_u16_be(&mut msg, ext_start, ext_len);

    let msg_len = (msg.len() - 4) as u32;
    write_u24_be(&mut msg, 1, msg_len);

    msg
}

/// Build an EncryptedExtensions handshake message (RFC 8446 §4.3.1).
///
/// Wire format: type(1) + length(3) + extensions_length(2) + extensions
///
/// If `alpn_protocol` is Some, includes the ALPN extension with the selected protocol.
pub fn build_encrypted_extensions(alpn_protocol: Option<&[u8]>) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(8); // EncryptedExtensions type
    msg.extend_from_slice(&[0u8; 2]);
    msg.push(0u8); // length high byte placeholder

    let ext_start = msg.len();
    msg.extend_from_slice(&[0u8; 2]); // extensions_length placeholder

    // ALPN extension (ext 16) — if a protocol was selected
    if let Some(proto) = alpn_protocol {
        push_u16_be(&mut msg, 16); // ALPN extension type

        // Build the ALPN extension data: protocol_name_list
        let mut proto_list = Vec::new();
        proto_list.push(proto.len() as u8);
        proto_list.extend_from_slice(proto);

        // extension_data_length = 2 (list length field) + proto_list
        let ext_data_len = 2 + proto_list.len();
        push_u16_be(&mut msg, ext_data_len as u16);

        // protocol_name_list_length
        push_u16_be(&mut msg, proto_list.len() as u16);

        // protocol_name_list
        msg.extend_from_slice(&proto_list);
    }

    let ext_len = (msg.len() - ext_start - 2) as u16;
    write_u16_be(&mut msg, ext_start, ext_len);

    // Fill message length
    let msg_len = (msg.len() - 4) as u32;
    write_u24_be(&mut msg, 1, msg_len);

    msg
}

/// Build a Certificate message (TLS 1.3 format, RFC 8446 §4.4.2).
///
/// Wire format:
///   handshake_type(1) = 11
///   length(3)
///   certificate_request_context_length(1) = 0  (server doesn't request certs)
///   certificate_list_length(3)
///     CertificateEntry:
///       cert_data_length(3)
///       cert_data (= DER certificate)
///       extensions_length(2) = 0
///       extensions (empty)
pub fn build_certificate_message(cert_der: &[u8]) -> Vec<u8> {
    build_certificate_chain_message(&[cert_der])
}

/// Build a Certificate message with a leaf-first certificate chain.
pub fn build_certificate_chain_message(cert_chain_der: &[&[u8]]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(11); // Certificate handshake type

    let cert_list_len: usize = cert_chain_der
        .iter()
        .map(|cert_der| 3 + cert_der.len() + 2)
        .sum();
    // Full message after type+length: context_len(1) + cert_list_len(3) + cert_entry
    let msg_body_len = 1 + 3 + cert_list_len;

    push_u24_be(&mut msg, msg_body_len as u32); // 3-byte message length
    msg.push(0); // certificate_request_context length = 0
    push_u24_be(&mut msg, cert_list_len as u32); // 3-byte certificate_list length

    for cert_der in cert_chain_der {
        push_u24_be(&mut msg, cert_der.len() as u32); // 3-byte cert_data length
        msg.extend_from_slice(cert_der);
        msg.extend_from_slice(&[0u8; 2]); // extensions length = 0
    }

    msg
}

/// Build a CertificateVerify message (RFC 8446 §4.4.3).
///
/// The signature is computed over:
///   0x20 * 64 || context_string || 0x00 || Hash(transcript)
/// where `transcript` is the concatenation of all prior handshake messages.
pub fn build_certificate_verify(
    transcript: &[u8],
    signing_key: &SigningKey,
    hasher: &Hasher,
) -> Result<Vec<u8>> {
    let context = b"TLS 1.3, server CertificateVerify";

    // Per RFC 8446 §4.4.3, the signed input is:
    //   0x20 * 64 || context_string || 0x00 || Hash(transcript)
    // The ecdsa::SigningKey::sign method hashes its input internally,
    // so we pass the raw padded data, NOT a pre-computed hash.
    let transcript_hash = hasher.hash(transcript);

    let mut padded = vec![0x20u8; 64];
    padded.extend_from_slice(context);
    padded.push(0x00);
    padded.extend_from_slice(&transcript_hash);

    let mut signer = signing_key.clone();
    let signature: Signature = <SigningKey as SignerMut<Signature>>::sign(&mut signer, &padded);
    // RFC 8446 §4.4.3 requires DER-encoded signatures
    // `to_bytes()` returns raw r||s (64 bytes), but CertificateVerify needs DER encoding
    let sig_der_bytes = signature.to_der().as_ref().to_vec();

    let mut msg = Vec::new();
    msg.push(15);

    // Handshake message length is body size only (RFC 8446 §4)
    let inner_len = 2 + 2 + sig_der_bytes.len(); // algorithm(2) + sig_len(2) + signature
    push_u24_be(&mut msg, inner_len as u32); // 3-byte body length

    push_u16_be(&mut msg, 0x0403);
    push_u16_be(&mut msg, sig_der_bytes.len() as u16);
    msg.extend_from_slice(&sig_der_bytes);

    Ok(msg)
}

/// Build a Finished message (RFC 8446 §4.4.4).
pub fn build_finished_message(verify_data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.push(20);
    push_u24_be(&mut msg, verify_data.len() as u32);
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
    use crate::tls::certificate_gen::generate_self_signed;
    use crate::tls::handshake::ClientHelloBuilder;
    use crate::tls::key_exchange::{EcdhKeyPair, KeyExchangeGroup};
    use crate::tls::prf::{server_write_keys, Tls13KeySchedule};
    use crate::tls::record::RecordCipher;

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
        let sh = crate::tls::handshake::ServerHello::parse(&sh_bytes).unwrap();
        assert_eq!(sh.random, random);
        assert_eq!(sh.cipher_suite, CipherSuite::TLS_AES_128_GCM_SHA256);
        assert_eq!(sh.server_key_share.len(), 65);
        assert_eq!(sh.supported_version, Some(0x0304));
    }

    #[test]
    fn test_encrypted_extensions_build() {
        let ee_bytes = build_encrypted_extensions(None);
        assert_eq!(ee_bytes[0], 8);
        assert_eq!(ee_bytes.len(), 6);
    }

    #[test]
    fn test_certificate_message_build() {
        let cert = generate_self_signed(&["localhost"]).unwrap();
        let cert_msg = build_certificate_message(&cert.cert_der);
        assert_eq!(cert_msg[0], 11);
        assert!(cert_msg.len() > cert.cert_der.len());
    }

    #[test]
    fn test_certificate_verify_build() {
        let cert = generate_self_signed(&["localhost"]).unwrap();
        let transcript = vec![0x01u8; 64];

        let cv_bytes =
            build_certificate_verify(&transcript, &*cert.signing_key, &Hasher::Sha256).unwrap();
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

        let verify_data =
            compute_server_finished_verify_data(&server_hs_secret, &transcript_hash, &hasher);
        assert_eq!(verify_data.len(), 32);

        let verify_data2 =
            compute_server_finished_verify_data(&server_hs_secret, &transcript_hash, &hasher);
        assert_eq!(verify_data, verify_data2);

        let different_secret = vec![0xFFu8; 32];
        let verify_data3 =
            compute_server_finished_verify_data(&different_secret, &transcript_hash, &hasher);
        assert_ne!(verify_data, verify_data3);
    }

    #[test]
    fn test_server_hello_encrypt_decrypt_roundtrip() {
        let cert = generate_self_signed(&["localhost"]).unwrap();
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

        let mut write_cipher =
            RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();

        let ee_bytes = build_encrypted_extensions(None);
        let ee_encrypted = write_cipher.encrypt(22, &ee_bytes);

        let mut read_cipher_ee =
            RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let (ct, plaintext) = read_cipher_ee.decrypt(&ee_encrypted).unwrap();
        assert_eq!(ct, 22);
        assert_eq!(plaintext, ee_bytes);

        let cert_msg = build_certificate_message(&cert.cert_der);
        let cert_encrypted = write_cipher.encrypt(22, &cert_msg);

        let mut read_cipher_cert =
            RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = read_cipher_cert.decrypt(&ee_encrypted).unwrap();
        let (_ct2, cert_decrypted) = read_cipher_cert.decrypt(&cert_encrypted).unwrap();
        assert_eq!(cert_decrypted, cert_msg);

        let transcript = vec![0xEEu8; 64];
        let cv_bytes = build_certificate_verify(&transcript, &*cert.signing_key, &hash).unwrap();
        let cv_encrypted = write_cipher.encrypt(22, &cv_bytes);

        let mut read_cipher_cv =
            RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = read_cipher_cv.decrypt(&ee_encrypted).unwrap();
        let _ = read_cipher_cv.decrypt(&cert_encrypted).unwrap();
        let (_ct3, cv_decrypted) = read_cipher_cv.decrypt(&cv_encrypted).unwrap();
        assert_eq!(cv_decrypted, cv_bytes);

        let transcript_hash = hash.hash(&transcript);
        let verify_data =
            compute_server_finished_verify_data(&server_hs_secret, &transcript_hash, &hash);
        let finished_msg = build_finished_message(&verify_data);
        let finished_encrypted = write_cipher.encrypt(22, &finished_msg);

        let mut read_cipher_fin =
            RecordCipher::new(&server_write.write_key, &server_write.write_iv).unwrap();
        let _ = read_cipher_fin.decrypt(&ee_encrypted).unwrap();
        let _ = read_cipher_fin.decrypt(&cert_encrypted).unwrap();
        let _ = read_cipher_fin.decrypt(&cv_encrypted).unwrap();
        let (_ct4, finished_decrypted) = read_cipher_fin.decrypt(&finished_encrypted).unwrap();
        assert_eq!(finished_decrypted, finished_msg);
    }
}
