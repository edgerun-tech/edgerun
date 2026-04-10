//! X.509 certificate generation using workspace `der` and `p256` primitives.
//!
//! Generates self-signed certificates with ECDSA P-256 keys for TLS servers.
//! Builds DER-encoded X.509 certificates manually — no rcgen needed for the
//! cert itself (rcgen is still re-exported for consumers who want it).

use edgerun_crypto::sha2::{Digest, Sha256};
use edgerun_crypto::p256::ecdsa::{SigningKey, Signature, signature::SignerMut};
use edgerun_crypto::p256::EncodedPoint;
use edgerun_crypto::rand_core::{OsRng, RngCore};
use edgerun_crypto::const_oid::db::{rfc5912, rfc4519, rfc5280};

// Re-export rcgen for consumers who need fine-grained control
pub use edgerun_crypto::rcgen;

/// A self-signed certificate with an associated ECDSA P-256 key pair.
pub struct CertificateAndKey {
    /// DER-encoded X.509 certificate
    pub cert_der: Vec<u8>,
    /// ECDSA P-256 signing key (for CertificateVerify during TLS handshake)
    pub signing_key: SigningKey,
}

/// Generate a self-signed certificate for the given hostname(s).
///
/// Creates an ECDSA P-256 key pair and a self-signed X.509 certificate
/// with the first hostname as CN and all hostnames in the SAN extension.
/// The certificate is valid for 1 year from now.
pub fn generate_self_signed(hostnames: &[&str]) -> CertificateAndKey {
    if hostnames.is_empty() {
        panic!("At least one hostname is required");
    }

    // Generate ECDSA P-256 signing key
    let signing_key = SigningKey::random(&mut OsRng);

    // Build the certificate
    let cert_der = build_cert(&signing_key, hostnames);

    CertificateAndKey { cert_der, signing_key }
}

/// Build a self-signed X.509 v3 certificate with ECDSA P-256.
fn build_cert(signing_key: &SigningKey, hostnames: &[&str]) -> Vec<u8> {
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = EncodedPoint::from(verifying_key).as_bytes().to_vec();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let not_before = format_utctime(now);
    let not_after = format_utctime(now + 365 * 24 * 3600);

    // Serial number (8 random bytes, positive)
    let mut serial = [0u8; 8];
    OsRng.fill_bytes(&mut serial);
    serial[0] &= 0x7F;

    // Build TBS Certificate
    let tbs = build_tbs(&serial, hostnames[0], &not_before, &not_after, &public_key_bytes, hostnames);

    // Sign TBS
    let digest = Sha256::digest(&tbs);
    let mut signer = signing_key.clone();
    let signature: Signature = signer.sign(&digest);
    let sig_bytes = signature.to_bytes();

    // Signature algorithm (OID only for ECDSA)
    let sig_algo = encode_oid(edgerun_crypto::const_oid::db::rfc5912::ECDSA_WITH_SHA_256);

    // Signature as BIT STRING
    let sig_bs = encode_bit_string(&sig_bytes);

    // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signatureValue }
    let mut cert = Vec::new();
    cert.push(0x30);
    encode_len(tbs.len() + sig_algo.len() + sig_bs.len(), &mut cert);
    cert.extend_from_slice(&tbs);
    cert.extend_from_slice(&sig_algo);
    cert.extend_from_slice(&sig_bs);

    cert
}

/// Build TBSCertificate
fn build_tbs(
    serial: &[u8],
    cn: &str,
    not_before: &str,
    not_after: &str,
    public_key: &[u8],
    hostnames: &[&str],
) -> Vec<u8> {
    let mut inner = Vec::new();

    // version [0] EXPLICIT INTEGER 2
    inner.push(0xa0);
    encode_len(3, &mut inner);
    inner.extend_from_slice(&[0x02, 0x01, 0x02]);

    // serialNumber
    inner.push(0x02);
    encode_len(serial.len(), &mut inner);
    inner.extend_from_slice(serial);

    // signature (algorithm OID)
    inner.extend_from_slice(&encode_oid(rfc5912::ECDSA_WITH_SHA_256));

    // issuer
    inner.extend_from_slice(&encode_name(cn));

    // validity
    inner.push(0x30);
    encode_len(2 + not_before.len() + 2 + not_after.len(), &mut inner);
    encode_utctime(not_before, &mut inner);
    encode_utctime(not_after, &mut inner);

    // subject
    inner.extend_from_slice(&encode_name(cn));

    // subjectPublicKeyInfo
    inner.extend_from_slice(&encode_spki(public_key));

    // extensions [3] EXPLICIT
    inner.extend_from_slice(&encode_extensions(hostnames));

    // Wrap in SEQUENCE
    let mut tbs = Vec::new();
    tbs.push(0x30);
    encode_len(inner.len(), &mut tbs);
    tbs.extend_from_slice(&inner);
    tbs
}

/// Encode SubjectPublicKeyInfo
fn encode_spki(public_key: &[u8]) -> Vec<u8> {
    // AlgorithmIdentifier SEQUENCE
    let mut algo = Vec::new();
    algo.extend_from_slice(&encode_oid(rfc5912::ID_EC_PUBLIC_KEY));
    algo.extend_from_slice(&encode_oid(rfc5912::SECP_256_R_1));

    let mut algo_seq = Vec::new();
    algo_seq.push(0x30);
    encode_len(algo.len(), &mut algo_seq);
    algo_seq.extend_from_slice(&algo);

    // BIT STRING for public key
    let mut bs = Vec::new();
    bs.push(0x03);
    encode_len(1 + public_key.len(), &mut bs);
    bs.push(0x00); // no unused bits
    bs.extend_from_slice(public_key);

    // SPKI SEQUENCE
    let mut spki = Vec::new();
    spki.push(0x30);
    encode_len(algo_seq.len() + bs.len(), &mut spki);
    spki.extend_from_slice(&algo_seq);
    spki.extend_from_slice(&bs);
    spki
}

/// Encode extensions (SAN only)
fn encode_extensions(hostnames: &[&str]) -> Vec<u8> {
    // Build SAN SEQUENCE
    let mut san = Vec::new();
    for h in hostnames {
        san.push(0x82); // dNSName (context [2])
        encode_len(h.len(), &mut san);
        san.extend_from_slice(h.as_bytes());
    }
    let mut san_seq = Vec::new();
    san_seq.push(0x30);
    encode_len(san.len(), &mut san_seq);
    san_seq.extend_from_slice(&san);

    // OCTET STRING wrapping SAN
    let mut octet = Vec::new();
    octet.push(0x04);
    encode_len(san_seq.len(), &mut octet);
    octet.extend_from_slice(&san_seq);

    // Extension SEQUENCE
    let mut ext = Vec::new();
    ext.extend_from_slice(&encode_oid(rfc5280::ID_CE_SUBJECT_ALT_NAME));
    ext.extend_from_slice(&octet);

    let mut ext_seq = Vec::new();
    ext_seq.push(0x30);
    encode_len(ext.len(), &mut ext_seq);
    ext_seq.extend_from_slice(&ext);

    // context [3] EXPLICIT
    let mut out = Vec::new();
    out.push(0xa3);
    encode_len(ext_seq.len(), &mut out);
    out.extend_from_slice(&ext_seq);
    out
}

// ---------- Low-level DER encoding helpers ----------

fn encode_oid(oid: edgerun_crypto::const_oid::ObjectIdentifier) -> Vec<u8> {
    let bytes = oid.as_bytes();
    let mut out = Vec::with_capacity(2 + bytes.len());
    out.push(0x06);
    encode_len(bytes.len(), &mut out);
    out.extend_from_slice(bytes);
    out
}

fn encode_name(cn: &str) -> Vec<u8> {
    let oid_bytes = rfc4519::COMMON_NAME.as_bytes();
    let cn_bytes = cn.as_bytes();

    // OID + value
    let mut av = Vec::new();
    av.push(0x06);
    encode_len(oid_bytes.len(), &mut av);
    av.extend_from_slice(oid_bytes);
    av.push(0x0c); // UTF8String
    encode_len(cn_bytes.len(), &mut av);
    av.extend_from_slice(cn_bytes);

    // SEQUENCE wrapping AV
    let mut seq = Vec::new();
    seq.push(0x30);
    encode_len(av.len(), &mut seq);
    seq.extend_from_slice(&av);

    // SET wrapping RDN
    let mut set = Vec::new();
    set.push(0x31);
    encode_len(seq.len(), &mut set);
    set.extend_from_slice(&seq);

    // SEQUENCE wrapping everything
    let mut name = Vec::new();
    name.push(0x30);
    encode_len(set.len(), &mut name);
    name.extend_from_slice(&set);
    name
}

fn encode_bit_string(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x03);
    encode_len(1 + data.len(), &mut out);
    out.push(0x00); // no unused bits
    out.extend_from_slice(data);
    out
}

fn encode_utctime(s: &str, out: &mut Vec<u8>) {
    out.push(0x17);
    encode_len(s.len(), out);
    out.extend_from_slice(s.as_bytes());
}

fn encode_len(len: usize, out: &mut Vec<u8>) {
    if len < 0x80 {
        out.push(len as u8);
    } else if len < 0x100 {
        out.push(0x81);
        out.push(len as u8);
    } else if len < 0x10000 {
        out.push(0x82);
        out.push((len >> 8) as u8);
        out.push(len as u8);
    } else {
        out.push(0x83);
        out.push((len >> 16) as u8);
        out.push((len >> 8) as u8);
        out.push(len as u8);
    }
}

fn format_utctime(timestamp: u64) -> String {
    let days = timestamp / 86400;
    let rem = timestamp % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    let (y, mo, d) = days_to_ymd(days);
    format!("{:02}{:02}{:02}{:02}{:02}{:02}Z", y % 100, mo, d, h, m, s)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    loop {
        let diy = if is_leap(y) { 366 } else { 365 };
        if days < diy { break; }
        days -= diy;
        y += 1;
    }
    let mdays = [31, if is_leap(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1;
    for &d in &mdays {
        if days < d { break; }
        days -= d;
        mo += 1;
    }
    (y, mo, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
