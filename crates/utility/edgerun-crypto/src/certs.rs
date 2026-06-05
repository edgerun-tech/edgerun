//! Certificate and key encoding helpers.

extern crate alloc;

use alloc::{string::String, vec::Vec};

use crate::{CryptoError, Result};

#[cfg(feature = "rsa")]
use crate::pkcs1::EncodeRsaPublicKey;

pub fn cert_and_key_from_pem(cert_pem: &str, key_pem: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let cert = cert_from_pem(cert_pem).ok_or(CryptoError::InvalidKey)?;
    let key = pem_block(key_pem, "PRIVATE KEY").ok_or(CryptoError::InvalidKey)?;
    Ok((cert, key))
}

pub fn cert_from_pem(pem: &str) -> Option<Vec<u8>> {
    x509_cert_from_pem(pem)
}

pub fn cert_to_pem(der: &[u8]) -> String {
    pem_encode(der)
}

#[cfg(feature = "p256")]
pub fn self_signed_p256_der(key: &crate::P256SigningKey, common_name: &str) -> Vec<u8> {
    self_signed_p256_der_for_names(key, &[common_name])
}

#[cfg(feature = "p256")]
pub fn self_signed_p256_der_for_names(key: &crate::P256SigningKey, names: &[&str]) -> Vec<u8> {
    let common_name = names.first().copied().unwrap_or("localhost");
    let subject = x509_name(common_name);
    let issuer = subject.clone();
    let public_key = key.public_key_sec1();

    let not_before = current_unix_secs().saturating_sub(60);
    let not_after = not_before.saturating_add(10 * 365 * 24 * 60 * 60);

    let serial = random_serial();
    let sig_alg = seq(concat(&[oid(&[
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02,
    ])]));
    let spki_alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]));
    let spki = seq(concat(&[spki_alg, bit_string(&public_key)]));

    let validity = seq(concat(&[utc_time(not_before), utc_time(not_after)]));
    let san_ext = subject_alt_name_extension(names);
    let extensions = tagged(0xa3, seq(concat(&[san_ext])));

    let tbs = seq(concat(&[
        tagged(0xa0, integer(&[2])),
        integer(&serial),
        sig_alg.clone(),
        issuer,
        validity,
        subject,
        spki,
        extensions,
    ]));

    let digest = crate::sha256(&tbs);
    let signature = key
        .sign_prehash_der(&digest)
        .expect("P-256 ECDSA signing should accept SHA-256 prehash");
    seq(concat(&[tbs, sig_alg, bit_string(&signature)]))
}

#[cfg(feature = "p256")]
pub fn self_signed_p256_pem(key: &crate::P256SigningKey, common_name: &str) -> String {
    pem_encode(&self_signed_p256_der(key, common_name))
}

#[cfg(feature = "p256")]
pub fn p256_csr_der_for_names(key: &crate::P256SigningKey, names: &[&str]) -> Vec<u8> {
    let common_name = names.first().copied().unwrap_or("localhost");
    let subject = x509_name(common_name);
    let public_key = key.public_key_sec1();
    let spki_alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]));
    let spki = seq(concat(&[spki_alg, bit_string(&public_key)]));
    let extension_request = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x0e]),
        set(seq(concat(&[subject_alt_name_extension(names)]))),
    ]));
    let attributes = tagged(0xa0, extension_request);
    let certification_request_info = seq(concat(&[integer(&[0]), subject, spki, attributes]));
    let sig_alg = seq(concat(&[oid(&[
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02,
    ])]));
    let digest = crate::sha256(&certification_request_info);
    let signature = key
        .sign_prehash_der(&digest)
        .expect("P-256 ECDSA signing should accept SHA-256 prehash");
    seq(concat(&[
        certification_request_info,
        sig_alg,
        bit_string(&signature),
    ]))
}

/// Generate a self-signed X.509 v3 certificate for an RSA-1024 key.
///
/// The resulting DER-encoded certificate is suitable for use as a Tor
/// link certificate (type 2).
#[cfg(feature = "rsa")]
pub fn self_signed_rsa_der(key: &crate::rsa::RsaPrivateKey, common_name: &str) -> Vec<u8> {
    use crate::rsa::traits::PublicKeyParts;

    let subject = x509_name(common_name);
    let issuer = subject.clone();

    // Build SubjectPublicKeyInfo for RSA
    let rsa_encryption_oid = oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01]); // 1.2.840.113549.1.1.1
    let null_param = alloc::vec![0x05, 0x00];
    let spki_alg = seq(concat(&[rsa_encryption_oid, null_param.clone()]));

    let pubkey: &crate::rsa::RsaPublicKey = key.as_ref();
    let n_bytes = pubkey.n().to_bytes_be();
    let e_bytes = pubkey.e().to_bytes_be();
    let rsa_pubkey = seq(concat(&[integer(&n_bytes), integer(&e_bytes)]));
    let spki = seq(concat(&[spki_alg, bit_string(&rsa_pubkey)]));

    let serial = rsa_serial();
    let not_before = rsa_timestamp().saturating_sub(60);
    let not_after = not_before.saturating_add(10 * 365 * 24 * 60 * 60);

    // sha256WithRSAEncryption OID = 1.2.840.113549.1.1.11
    let sig_alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b]),
        null_param.clone(),
    ]));

    let validity = seq(concat(&[utc_time(not_before), utc_time(not_after)]));

    let tbs = seq(concat(&[
        tagged(0xa0, integer(&[2])),
        integer(&serial),
        sig_alg.clone(),
        issuer,
        validity,
        subject,
        spki,
    ]));

    use crate::rsa::signature::{RandomizedSigner, SignatureEncoding as RsaSignatureEncoding};
    use crate::rsa::pkcs1v15::SigningKey as RsaPkcs1v15SigningKey;
    let signing_key = RsaPkcs1v15SigningKey::<crate::sha2::Sha256>::new(key.clone());
    let mut rng = crate::rsa_rng::RsaRng;
    let signature = signing_key.sign_with_rng(&mut rng, &tbs);
    let sig_bytes: Vec<u8> = signature.to_bytes().to_vec();

    seq(concat(&[tbs, sig_alg, bit_string(&sig_bytes)]))
}

/// Return the SHA-256 of the PKCS#1 DER-encoded RSA public key.
///
/// This is what Tor uses for the identity digest field
/// (`tor_x509_cert_get_id_digests` returns SHA-256 of the RSA public key
/// SubjectPublicKeyInfo / PKCS#1 `RSAPublicKey` DER, not the full cert DER).
#[cfg(feature = "rsa")]
pub fn rsa_public_key_id_digest(key: &crate::rsa::RsaPrivateKey) -> [u8; 32] {
    let pub_key = key.to_public_key();
    let doc = pub_key.to_pkcs1_der().expect("RSA PKCS#1 DER encoding");
    crate::sha256(doc.as_bytes())
}

/// Given a DER-encoded RSA X.509 certificate, return the SHA-256 of its
/// embedded RSA public key in PKCS#1 DER form.
///
/// Equivalent to Tor's `tor_x509_cert_get_id_digests` for a peer certificate.
#[cfg(feature = "rsa")]
pub fn extract_rsa_cert_id_digest(cert_der: &[u8]) -> Result<[u8; 32]> {
    let pkcs1 = pkcs1_rsa_pubkey_from_cert(cert_der)?;
    Ok(crate::sha256(&pkcs1))
}

#[cfg(feature = "rsa")]
fn pkcs1_rsa_pubkey_from_cert(cert_der: &[u8]) -> Result<Vec<u8>> {
    fn der_len(data: &[u8], pos: &mut usize) -> core::result::Result<usize, ()> {
        if *pos >= data.len() {
            return Err(());
        }
        let b = data[*pos];
        *pos += 1;
        if b < 0x80 {
            return Ok(b as usize);
        }
        let n = (b & 0x7f) as usize;
        if n > 4 || *pos + n > data.len() {
            return Err(());
        }
        let mut v = 0usize;
        for _ in 0..n {
            v = (v << 8) | data[*pos] as usize;
            *pos += 1;
        }
        Ok(v)
    }

    fn skip_tlv(data: &[u8], pos: &mut usize, expected: u8) -> core::result::Result<(), ()> {
        if *pos >= data.len() {
            return Err(());
        }
        let tag = data[*pos];
        *pos += 1;
        let l = der_len(data, pos)?;
        if *pos + l > data.len() {
            return Err(());
        }
        if tag != expected {
            return Err(());
        }
        *pos += l;
        Ok(())
    }

    fn expect_tag(data: &[u8], pos: &mut usize, expected: u8) -> core::result::Result<usize, ()> {
        if *pos >= data.len() {
            return Err(());
        }
        let tag = data[*pos];
        *pos += 1;
        let l = der_len(data, pos)?;
        if *pos + l > data.len() {
            return Err(());
        }
        if tag != expected {
            return Err(());
        }
        Ok(l)
    }

    let mut pos = 0;

    // Outer SEQUENCE (Certificate)
    let _outer_len = expect_tag(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;

    // tbsCertificate SEQUENCE
    let tbs_len = expect_tag(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;
    let tbs_end = pos + tbs_len;

    // Skip [0] EXPLICIT (v3 version) if present
    if pos < tbs_end && cert_der[pos] == 0xa0 {
        skip_tlv(cert_der, &mut pos, 0xa0).map_err(|_| CryptoError::InternalError)?;
    }

    // Skip INTEGER (serialNumber)
    skip_tlv(cert_der, &mut pos, 0x02).map_err(|_| CryptoError::InternalError)?;
    // Skip SEQUENCE (signature algorithm)
    skip_tlv(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;
    // Skip SEQUENCE (issuer)
    skip_tlv(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;
    // Skip SEQUENCE (validity)
    skip_tlv(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;
    // Skip SEQUENCE (subject)
    skip_tlv(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;

    // Now at SubjectPublicKeyInfo
    let _spki_len = expect_tag(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;

    // Skip AlgorithmIdentifier SEQUENCE inside SPKI
    skip_tlv(cert_der, &mut pos, 0x30).map_err(|_| CryptoError::InternalError)?;

    // Read BIT STRING containing the PKCS#1 RSAPublicKey
    let bs_len = expect_tag(cert_der, &mut pos, 0x03).map_err(|_| CryptoError::InternalError)?;
    if pos >= cert_der.len() || cert_der[pos] != 0 {
        return Err(CryptoError::InternalError);
    }
    // Skip unused-bits byte, return the PKCS#1 RSAPublicKey DER
    Ok(cert_der[pos + 1..pos + bs_len].to_vec())
}

/// Extract the SIGNED_WITH_KEY Ed25519 identity key from a Tor type-4 cert.
///
/// The cert structure is (Trunnel format):
///   version(1) | cert_type(1) | exp(4) | cert_key_type(1) |
///   certified_key(32) | n_ext(1) |
///   ext_len(2) | ext_type(1) | ext_flags(1) | ext_body(ext_len) |
///   signature(64)
///
/// For the identity-type cert, the first extension is SIGNED_WITH_KEY (type 4)
/// and its body is the 32-byte Ed25519 identity key.
pub fn extract_ed25519_cert_id_key(cert: &[u8]) -> Result<[u8; 32]> {
    if cert.len() < 44 + 32 {
        return Err(CryptoError::InternalError);
    }
    let n_ext = cert[39];
    if n_ext < 1 {
        return Err(CryptoError::InternalError);
    }
    let ext_len = u16::from_be_bytes([cert[40], cert[41]]);
    if ext_len != 32 {
        return Err(CryptoError::InternalError);
    }
    if cert[42] != 4 {
        // CERTEXT_SIGNED_WITH_KEY
        return Err(CryptoError::InternalError);
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&cert[44..76]);
    Ok(key)
}

/// Fixed serial for RSA certs (deterministic for testing).
fn rsa_serial() -> Vec<u8> {
    alloc::vec![0x01]
}

/// Plausible timestamp for ephemeral RSA certs.
fn rsa_timestamp() -> u64 {
    1_704_067_200
}

/// Build an X.509 Name (a SEQUENCE of SETs of AttributeTypeAndValue) for
/// a single CommonName attribute.
#[cfg(any(feature = "p256", feature = "rsa"))]
fn x509_name(common_name: &str) -> Vec<u8> {
    seq(concat(&[set(seq(concat(&[
        oid(&[0x55, 0x04, 0x03]),
        utf8_string(common_name),
    ])))]))
}

#[cfg(feature = "p256")]
pub fn p256_key_from_pem(pem: &str) -> Option<crate::P256SigningKey> {
    let der = pem_block(pem, "PRIVATE KEY").or_else(|| pem_block(pem, "EC PRIVATE KEY"))?;
    p256_key_from_der(&der)
}

#[cfg(feature = "p256")]
pub fn p256_key_from_der(der: &[u8]) -> Option<crate::P256SigningKey> {
    if der.len() == 32 {
        let bytes: [u8; 32] = der.try_into().ok()?;
        return crate::P256SigningKey::from_bytes(&bytes).ok();
    }

    let mut i = 0;
    while i + 34 <= der.len() {
        if der[i] == 0x04 && der[i + 1] == 0x20 {
            let bytes = &der[i + 2..i + 34];
            if let Ok(bytes) = <[u8; 32]>::try_from(bytes) {
                if let Ok(key) = crate::P256SigningKey::from_bytes(&bytes) {
                    return Some(key);
                }
            }
        }
        i += 1;
    }
    None
}

#[cfg(feature = "p256")]
pub fn p256_key_to_pem(key: &crate::P256SigningKey) -> String {
    pem_encode_labeled("PRIVATE KEY", &p256_key_to_der(key))
}

#[cfg(feature = "p256")]
pub fn p256_key_to_der(key: &crate::P256SigningKey) -> Vec<u8> {
    p256_private_key_info_der(key)
}

pub fn pem_encode(data: &[u8]) -> String {
    pem_encode_labeled("CERTIFICATE", data)
}

pub fn x509_cert_from_pem(pem: &str) -> Option<Vec<u8>> {
    pem_block(pem, "CERTIFICATE")
}

fn der_len(len: usize) -> Vec<u8> {
    if len < 128 {
        return alloc::vec![len as u8];
    }
    let mut tmp = [0u8; core::mem::size_of::<usize>()];
    let mut value = len;
    let mut pos = tmp.len();
    while value > 0 {
        pos -= 1;
        tmp[pos] = value as u8;
        value >>= 8;
    }
    let len_len = tmp.len() - pos;
    let mut out = Vec::with_capacity(1 + len_len);
    out.push(0x80 | len_len as u8);
    out.extend_from_slice(&tmp[pos..]);
    out
}

fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + value.len() + 5);
    out.push(tag);
    out.extend_from_slice(&der_len(value.len()));
    out.extend_from_slice(value);
    out
}

fn seq(value: Vec<u8>) -> Vec<u8> {
    tlv(0x30, &value)
}

fn set(value: Vec<u8>) -> Vec<u8> {
    tlv(0x31, &value)
}

fn tagged(tag: u8, value: Vec<u8>) -> Vec<u8> {
    tlv(tag, &value)
}

fn integer(bytes: &[u8]) -> Vec<u8> {
    let mut start = 0;
    while start + 1 < bytes.len() && bytes[start] == 0 {
        start += 1;
    }
    let mut value = Vec::from(&bytes[start..]);
    if value.is_empty() {
        value.push(0);
    }
    if value[0] & 0x80 != 0 {
        value.insert(0, 0);
    }
    tlv(0x02, &value)
}

fn oid(encoded_body: &[u8]) -> Vec<u8> {
    tlv(0x06, encoded_body)
}

fn utf8_string(value: &str) -> Vec<u8> {
    tlv(0x0c, value.as_bytes())
}

fn ia5_string_tagged(tag: u8, value: &str) -> Vec<u8> {
    tlv(tag, value.as_bytes())
}

fn bit_string(value: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(value.len() + 1);
    body.push(0);
    body.extend_from_slice(value);
    tlv(0x03, &body)
}

fn octet_string(value: &[u8]) -> Vec<u8> {
    tlv(0x04, value)
}

fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
    let len = parts.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(len);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

#[cfg(feature = "p256")]
fn subject_alt_name_extension(names: &[&str]) -> Vec<u8> {
    let dns_names = if names.is_empty() {
        alloc::vec!["localhost"]
    } else {
        names.to_vec()
    };
    let mut general_names = Vec::new();
    for name in dns_names {
        general_names.extend_from_slice(&ia5_string_tagged(0x82, name));
    }
    let san_der = seq(general_names);
    seq(concat(&[oid(&[0x55, 0x1d, 0x11]), octet_string(&san_der)]))
}

#[cfg(feature = "p256")]
fn random_serial() -> [u8; 16] {
    let mut serial = [0u8; 16];
    let _ = crate::fill_random(&mut serial);
    serial[0] &= 0x7f;
    if serial.iter().all(|b| *b == 0) {
        serial[15] = 1;
    }
    serial
}

#[cfg(feature = "p256")]
fn current_unix_secs() -> u64 {
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(1_704_067_200)
    }
    #[cfg(not(feature = "std"))]
    {
        1_704_067_200
    }
}

fn utc_time(unix_secs: u64) -> Vec<u8> {
    let days = (unix_secs / 86_400) as i64;
    let seconds = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    let year = (year % 100) as u64;
    let mut value = String::new();
    push_two_digits(&mut value, year);
    push_two_digits(&mut value, month as u64);
    push_two_digits(&mut value, day as u64);
    push_two_digits(&mut value, hour);
    push_two_digits(&mut value, minute);
    push_two_digits(&mut value, second);
    value.push('Z');
    tlv(0x17, value.as_bytes())
}

fn push_two_digits(out: &mut String, value: u64) {
    out.push((b'0' + ((value / 10) % 10) as u8) as char);
    out.push((b'0' + (value % 10) as u8) as char);
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}

#[cfg(feature = "p256")]
fn p256_private_key_info_der(key: &crate::P256SigningKey) -> Vec<u8> {
    let private_key = key.to_bytes();
    let public_key = key.public_key_sec1();
    let ec_private_key = seq(concat(&[
        integer(&[1]),
        octet_string(&private_key),
        tagged(0xa1, bit_string(&public_key)),
    ]));
    let alg = seq(concat(&[
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        oid(&[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]));
    seq(concat(&[integer(&[0]), alg, octet_string(&ec_private_key)]))
}

fn pem_encode_labeled(label: &str, data: &[u8]) -> String {
    let encoded = base64_encode(data);
    let mut out = String::new();
    out.push_str("-----BEGIN ");
    out.push_str(label);
    out.push_str("-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        for byte in chunk {
            out.push(*byte as char);
        }
        out.push('\n');
    }
    out.push_str("-----END ");
    out.push_str(label);
    out.push_str("-----");
    out
}

fn pem_block(pem: &str, label: &str) -> Option<Vec<u8>> {
    let mut begin = String::from("-----BEGIN ");
    begin.push_str(label);
    begin.push_str("-----");
    let mut end = String::from("-----END ");
    end.push_str(label);
    end.push_str("-----");

    let start = pem.find(&begin)? + begin.len();
    let rest = &pem[start..];
    let stop = rest.find(&end)?;
    let body = &rest[..stop];
    let mut compact = String::new();
    for ch in body.chars() {
        if !ch.is_whitespace() {
            compact.push(ch);
        }
    }
    base64_decode(&compact)
}

fn base64_encode(data: &[u8]) -> String {
    edgerun_encoding::base64::standard_encode(data)
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    edgerun_encoding::base64::standard_decode(input).ok()
}
