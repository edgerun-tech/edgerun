//! Hardware-backed TLS for the Lifegraph node daemon.
//!
//! The root CA private key lives in secure hardware (TPM, YubiKey) and never leaves.
//! On each startup, an ephemeral leaf certificate is issued by the root CA and used
//! for TLS handshakes. The leaf cert + root CA cert form the TLS chain presented to peers.

use lifegraph_hardware_signing::MeshSigner;
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Root CA
// ---------------------------------------------------------------------------

/// A root CA backed by secure hardware.
///
/// The `root_cert` contains the CA's public identity.
/// The `signer` holds the CA's private key in hardware and performs all signing.
pub struct RootCa {
    root_cert: CertificateDer<'static>,
    signer: Arc<dyn MeshSigner + Send + Sync>,
}

impl RootCa {
    /// Creates a RootCa from a certificate file and a hardware signer.
    ///
    /// Verifies that the certificate's public key matches the signer's NodeID.
    pub fn new(
        cert_path: &std::path::Path,
        signer: Arc<dyn MeshSigner + Send + Sync>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cert = load_cert(cert_path)?;
        verify_cert_matches_node_id(&cert, &*signer)?;
        Ok(Self {
            root_cert: cert,
            signer,
        })
    }

    /// Issues a new ephemeral leaf certificate signed by this root CA.
    ///
    /// Returns (leaf_cert_der, ephemeral_private_key_pkcs8).
    /// The leaf cert is valid for 24 hours and should be regenerated on restart.
    pub fn issue_leaf(
        &self,
        node_id_hex: &str,
    ) -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>), Box<dyn std::error::Error + Send + Sync>> {
        issue_ephemeral_leaf(&self.root_cert, &*self.signer, node_id_hex)
    }

    /// Returns the root CA certificate.
    pub fn root_cert(&self) -> &CertificateDer<'static> {
        &self.root_cert
    }
}

// ---------------------------------------------------------------------------
// Certificate loading
// ---------------------------------------------------------------------------

/// Loads a certificate from a PEM or DER file.
pub fn load_cert(path: &std::path::Path) -> Result<CertificateDer<'static>, Box<dyn std::error::Error + Send + Sync>> {
    let data = std::fs::read(path)
        .map_err(|e| format!("failed to read certificate file {}: {}", path.display(), e))?;

    // Try PEM first
    let mut reader = std::io::Cursor::new(&data);
    let certs: Result<Vec<_>, _> = rustls_pemfile::certs(&mut reader).collect();
    if let Ok(certs) = certs {
        if let Some(cert) = certs.into_iter().next() {
            return Ok(cert);
        }
    }

    // Try DER
    Ok(CertificateDer::from(data))
}

/// Verifies that a certificate's public key matches the node's hardware identity.
fn verify_cert_matches_node_id(
    cert: &CertificateDer<'_>,
    signer: &dyn MeshSigner,
) -> Result<(), String> {
    let node_id = signer.node_id();
    let cert_pubkey = extract_spki_pubkey_bytes(cert)?;

    // NodeID is uncompressed x||y (64 bytes).
    // SPKI public key is 0x04 || x || y (65 bytes).
    if cert_pubkey.len() == 65 && cert_pubkey[0] == 0x04 {
        if cert_pubkey[1..] == node_id.0 {
            return Ok(());
        }
        return Err(format!(
            "certificate public key does not match node identity.\n  cert: {}\n  node: {}",
            lifegraph_core::util::bytes_to_hex(&cert_pubkey),
            lifegraph_core::util::bytes_to_hex(&node_id.0),
        ));
    }

    Err(format!(
        "certificate public key format is unexpected ({} bytes), expected 65-byte uncompressed SEC1",
        cert_pubkey.len()
    ))
}

/// Extracts the raw public key bytes from a certificate's SubjectPublicKeyInfo.
/// Returns the uncompressed SEC1 point: 0x04 || x || y (65 bytes).
fn extract_spki_pubkey_bytes(cert: &CertificateDer<'_>) -> Result<Vec<u8>, String> {
    // Parse the SubjectPublicKeyInfo from the certificate.
    // The SPKI is inside the TBS Certificate, which is inside the outer SEQUENCE.
    //
    // We use x509-parser for robust parsing.
    use x509_parser::prelude::*;

    let (_, x509) = X509Certificate::from_der(cert.as_ref())
        .map_err(|e| format!("failed to parse certificate: {}", e))?;

    let pubkey = x509.public_key();
    // For EC P-256, raw() returns the full BIT STRING content (0x00 || 0x04 || x || y)
    // or just the point. Let's handle both.
    let raw = pubkey.raw;
    if raw.len() == 65 && raw[0] == 0x04 {
        return Ok(raw.to_vec());
    }
    if raw.len() == 66 && raw[0] == 0x00 && raw[1] == 0x04 {
        return Ok(raw[1..].to_vec());
    }
    // Try the full BIT STRING (with length encoding)
    let spki_bytes = x509.tbs_certificate.subject_pki.subject_public_key.as_ref();
    if spki_bytes.len() == 65 && spki_bytes[0] == 0x04 {
        return Ok(spki_bytes.to_vec());
    }
    if spki_bytes.len() == 66 && spki_bytes[0] == 0x00 && spki_bytes[1] == 0x04 {
        return Ok(spki_bytes[1..].to_vec());
    }

    Err(format!(
        "failed to extract EC public key from certificate (raw={} bytes, spki={} bytes)",
        raw.len(),
        spki_bytes.len()
    ))
}

// ---------------------------------------------------------------------------
// Ephemeral leaf certificate issuance
// ---------------------------------------------------------------------------

/// Issues an ephemeral leaf certificate signed by the root CA's hardware key.
///
/// Strategy:
/// 1. Generate an ephemeral ECDSA P-256 keypair with ring
/// 2. Use rcgen to build a self-signed certificate (which gives us proper ASN.1)
/// 3. Extract the TBSCertificate bytes from the rcgen output
/// 4. Re-sign the TBSCertificate with the root CA's hardware signer
/// 5. Re-assemble the full certificate with the root CA's signature
///
/// This ensures the root CA private key never leaves hardware while still
/// producing a valid X.509 v3 certificate.
fn issue_ephemeral_leaf(
    root_cert: &CertificateDer<'_>,
    root_signer: &dyn MeshSigner,
    node_id_hex: &str,
) -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>), Box<dyn std::error::Error + Send + Sync>> {
    use ring::rand::SystemRandom;
    use sha2::{Digest, Sha256};

    // 1. Generate ephemeral keypair with ring
    let rng = SystemRandom::new();
    let eph_pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        &rng,
    ).map_err(|e| format!("failed to generate ephemeral key: {:?}", e))?;
    let eph_key = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        eph_pkcs8.as_ref(),
        &rng,
    ).map_err(|e| format!("failed to parse ephemeral key: {:?}", e))?;

    // 2. Calculate validity times
    let not_before = SystemTime::now();
    let not_after = not_before + Duration::from_secs(86400); // 24 hours

    // 3. Extract the root CA's subject DN to use as the issuer
    let root_subject = extract_subject_dn(root_cert)?;

    // 4. Build the TBSCertificate with root CA as issuer
    let (tbs_cert_bytes, sig_algo) = build_tbs_certificate(
        &eph_key,
        &root_subject,
        node_id_hex,
        not_before,
        not_after,
    )?;

    // 5. Sign the TBSCertificate with the root CA's hardware signer
    let tbs_hash = Sha256::digest(&tbs_cert_bytes);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&tbs_hash);
    let raw_sig = root_signer.sign_digest(&digest_bytes)
        .map_err(|e| format!("hardware signing failed: {}", e))?;

    // Convert raw r||s to DER-encoded ECDSA signature
    let der_sig = encode_ecdsa_der(&raw_sig);

    // 6. Assemble the full certificate with the root CA's signature
    let leaf_cert_der = assemble_certificate(&tbs_cert_bytes, &sig_algo, &der_sig)?;

    // Return leaf cert + ephemeral private key
    let leaf_cert = CertificateDer::from(leaf_cert_der);
    let priv_key = PrivatePkcs8KeyDer::from(eph_pkcs8.as_ref().to_vec());

    Ok((leaf_cert, priv_key))
}

// ---------------------------------------------------------------------------
// TBSCertificate building
// ---------------------------------------------------------------------------

/// Builds a TBSCertificate with the given ephemeral public key and root CA as issuer.
/// Returns (tbs_cert_der, signature_algorithm_der).
fn build_tbs_certificate(
    eph_key: &ring::signature::EcdsaKeyPair,
    root_subject: &[u8],
    node_id_hex: &str,
    not_before: SystemTime,
    not_after: SystemTime,
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    use ring::signature::KeyPair;

    let eph_pubkey = eph_key.public_key().as_ref();

    // Generate a random serial number (20 bytes)
    let serial: [u8; 20] = rand_serial();

    // Build the subject DN
    let subject_dn = build_leaf_subject_dn(node_id_hex)?;

    // Build validity
    let not_before_bytes = time_to_utctime(not_before)?;
    let not_after_bytes = time_to_utctime(not_after)?;

    // Build SubjectPublicKeyInfo for the ephemeral key
    let spki = build_spki(eph_pubkey)?;

    // Build extensions
    let extensions = build_leaf_extensions()?;

    // ecdsa-with-SHA256 OID: 1.2.840.10045.4.3.2
    let sig_algo = encode_algorithm_identifier_ecdsa_sha256();

    let mut tbs_content = Vec::with_capacity(250);

    // version [0] EXPLICIT INTEGER {2}
    tbs_content.push(0xa0); // context-specific [0]
    tbs_content.push(0x03); // length
    tbs_content.push(0x02); // INTEGER
    tbs_content.push(0x01); // length
    tbs_content.push(0x02); // value: 2 (v3)

    // serialNumber
    tbs_content.push(0x02); // INTEGER
    let serial_der = encode_der_integer_positive(&serial);
    tbs_content.extend_from_slice(&serial_der);

    // signature AlgorithmIdentifier
    tbs_content.extend_from_slice(&sig_algo);

    // issuer (root CA subject)
    tbs_content.push(0x30); // SEQUENCE
    tbs_content.push(root_subject.len() as u8);
    tbs_content.extend_from_slice(root_subject);

    // validity
    tbs_content.push(0x30); // SEQUENCE
    tbs_content.push((2 + not_before_bytes.len() + 2 + not_after_bytes.len()) as u8);
    tbs_content.push(0x17); // UTCTime
    tbs_content.push(not_before_bytes.len() as u8);
    tbs_content.extend_from_slice(&not_before_bytes);
    tbs_content.push(0x17); // UTCTime
    tbs_content.push(not_after_bytes.len() as u8);
    tbs_content.extend_from_slice(&not_after_bytes);

    // subject (leaf subject DN)
    tbs_content.push(0x30); // SEQUENCE
    tbs_content.push(subject_dn.len() as u8);
    tbs_content.extend_from_slice(&subject_dn);

    // subjectPublicKeyInfo
    tbs_content.extend_from_slice(&spki);

    // extensions [3] EXPLICIT
    tbs_content.push(0xa3); // context-specific [3]
    tbs_content.push(extensions.len() as u8);
    tbs_content.extend_from_slice(&extensions);

    // Write TBSCertificate SEQUENCE
    let mut tbs = Vec::with_capacity(tbs_content.len() + 2);
    tbs.push(0x30); // SEQUENCE
    tbs.push(tbs_content.len() as u8);
    tbs.extend_from_slice(&tbs_content);

    Ok((tbs, sig_algo))
}

/// Assembles a full X.509 certificate from TBSCertificate, signature algorithm, and signature.
fn assemble_certificate(
    tbs_cert: &[u8],
    sig_algo: &[u8],
    signature: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut cert = Vec::with_capacity(tbs_cert.len() + sig_algo.len() + signature.len() + 16);

    // Calculate outer SEQUENCE length
    let inner_len = tbs_cert.len() + sig_algo.len() + 2 + signature.len() + 1; // +1 for BIT STRING unused bits byte

    cert.push(0x30); // SEQUENCE
    if inner_len < 128 {
        cert.push(inner_len as u8);
    } else {
        cert.push(0x81);
        cert.push(inner_len as u8);
    }

    // TBSCertificate
    cert.extend_from_slice(tbs_cert);

    // signatureAlgorithm
    cert.extend_from_slice(sig_algo);

    // signatureValue BIT STRING
    cert.push(0x03); // BIT STRING
    cert.push((signature.len() + 1) as u8);
    cert.push(0x00); // unused bits
    cert.extend_from_slice(signature);

    Ok(cert)
}

// ---------------------------------------------------------------------------
// ASN.1 encoding helpers
// ---------------------------------------------------------------------------

/// Encodes a positive integer as a DER INTEGER.
fn encode_der_integer_positive(bytes: &[u8]) -> Vec<u8> {
    // Skip leading zeros
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len().saturating_sub(1));
    let trimmed = &bytes[start..];

    // Add leading zero if high bit is set
    let needs_zero = trimmed[0] & 0x80 != 0;
    let len = trimmed.len() + if needs_zero { 1 } else { 0 };

    let mut result = Vec::with_capacity(2 + len);
    result.push(0x02); // INTEGER
    result.push(len as u8);
    if needs_zero {
        result.push(0x00);
    }
    result.extend_from_slice(trimmed);
    result
}

/// Encodes an OID as a DER OID.
fn encode_der_oid(oid: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(2 + oid.len());
    result.push(0x06); // OID
    result.push(oid.len() as u8);
    result.extend_from_slice(oid);
    result
}

/// ecdsa-with-SHA256 OID: 1.2.840.10045.4.3.2
const OID_ECDSA_SHA256: [u8; 8] = [0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];

/// ecPublicKey OID: 1.2.840.10045.2.1
const OID_EC_PUBLIC_KEY: [u8; 7] = [0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];

/// prime256v1 OID: 1.2.840.10045.3.1.7
const OID_PRIME256V1: [u8; 8] = [0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];

/// subjectAltName OID: 2.5.29.17
const OID_SUBJECT_ALT_NAME: [u8; 3] = [0x55, 0x1d, 0x11];

/// Encodes the ecdsa-with-SHA256 AlgorithmIdentifier.
fn encode_algorithm_identifier_ecdsa_sha256() -> Vec<u8> {
    let oid = encode_der_oid(&OID_ECDSA_SHA256);
    let mut result = Vec::with_capacity(oid.len() + 2);
    result.push(0x30); // SEQUENCE
    result.push(oid.len() as u8);
    result.extend_from_slice(&oid);
    result
}

/// Builds a SubjectPublicKeyInfo for an EC P-256 public key.
fn build_spki(pubkey: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let algo_oid = encode_der_oid(&OID_EC_PUBLIC_KEY);
    let curve_oid = encode_der_oid(&OID_PRIME256V1);
    let algo_seq = {
        let mut s = Vec::with_capacity(algo_oid.len() + curve_oid.len() + 2);
        s.push(0x30); // SEQUENCE
        s.push((algo_oid.len() + curve_oid.len()) as u8);
        s.extend_from_slice(&algo_oid);
        s.extend_from_slice(&curve_oid);
        s
    };

    // BIT STRING containing the public key
    let mut bit_string = Vec::with_capacity(pubkey.len() + 3);
    bit_string.push(0x03); // BIT STRING
    bit_string.push((pubkey.len() + 1) as u8);
    bit_string.push(0x00); // unused bits
    bit_string.extend_from_slice(pubkey);

    // SubjectPublicKeyInfo SEQUENCE
    let mut spki = Vec::with_capacity(algo_seq.len() + bit_string.len() + 2);
    spki.push(0x30); // SEQUENCE
    spki.push((algo_seq.len() + bit_string.len()) as u8);
    spki.extend_from_slice(&algo_seq);
    spki.extend_from_slice(&bit_string);

    Ok(spki)
}

/// Builds the Subject DN for a leaf certificate.
fn build_leaf_subject_dn(node_id_hex: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // CN=lifegraph-<short-id>, O=Lifegraph
    let cn_value = format!("lifegraph-{}", &node_id_hex[..12.min(node_id_hex.len())]);
    let org_value = "Lifegraph";

    // CN: OID 2.5.4.3
    let cn_oid: [u8; 3] = [0x55, 0x04, 0x03];
    let cn_oid_enc = encode_der_oid(&cn_oid);
    let cn_utf8 = cn_value.as_bytes();
    let cn_set = {
        let mut s = Vec::with_capacity(cn_oid_enc.len() + cn_utf8.len() + 6);
        // OID
        s.extend_from_slice(&cn_oid_enc);
        // UTF8String
        s.push(0x0c);
        s.push(cn_utf8.len() as u8);
        s.extend_from_slice(cn_utf8);
        // Wrap in SET
        let mut wrapped = Vec::with_capacity(s.len() + 2);
        wrapped.push(0x31); // SET
        wrapped.push(s.len() as u8);
        wrapped.extend_from_slice(&s);
        wrapped
    };

    // O: OID 2.5.4.10
    let o_oid: [u8; 3] = [0x55, 0x04, 0x0a];
    let o_oid_enc = encode_der_oid(&o_oid);
    let o_utf8 = org_value.as_bytes();
    let o_set = {
        let mut s = Vec::with_capacity(o_oid_enc.len() + o_utf8.len() + 6);
        s.extend_from_slice(&o_oid_enc);
        s.push(0x0c);
        s.push(o_utf8.len() as u8);
        s.extend_from_slice(o_utf8);
        let mut wrapped = Vec::with_capacity(s.len() + 2);
        wrapped.push(0x31); // SET
        wrapped.push(s.len() as u8);
        wrapped.extend_from_slice(&s);
        wrapped
    };

    let mut dn = Vec::with_capacity(cn_set.len() + o_set.len());
    dn.extend_from_slice(&cn_set);
    dn.extend_from_slice(&o_set);

    Ok(dn)
}

/// Builds leaf certificate extensions: subjectAltName (DNS:<node-id>)
fn build_leaf_extensions() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Extensions ::= SEQUENCE { SEQUENCE { OID, OCTET STRING { SEQUENCE { [2] { IA5String } } } } }
    // subjectAltName: dNSName = lifegraph-<id>

    let oid = encode_der_oid(&OID_SUBJECT_ALT_NAME);
    let dns_name = "lifegraph-node.local".as_bytes();

    // [2] IMPLICIT IA5String (dNSName tag is 0x82)
    let mut san_content = Vec::with_capacity(dns_name.len() + 2);
    san_content.push(0x82); // dNSName [2]
    san_content.push(dns_name.len() as u8);
    san_content.extend_from_slice(dns_name);

    // OCTET STRING wrapping the SEQUENCE
    let mut octet_inner = Vec::with_capacity(san_content.len() + 2);
    octet_inner.push(0x30); // SEQUENCE
    octet_inner.push(san_content.len() as u8);
    octet_inner.extend_from_slice(&san_content);

    let octet_string = {
        let mut o = Vec::with_capacity(octet_inner.len() + 2);
        o.push(0x04); // OCTET STRING
        o.push(octet_inner.len() as u8);
        o.extend_from_slice(&octet_inner);
        o
    };

    let mut ext_seq = Vec::with_capacity(oid.len() + octet_string.len() + 2);
    ext_seq.push(0x30); // SEQUENCE
    ext_seq.push((oid.len() + octet_string.len()) as u8);
    ext_seq.extend_from_slice(&oid);
    ext_seq.extend_from_slice(&octet_string);

    // Wrap in outer SEQUENCE for extensions
    let mut extensions = Vec::with_capacity(ext_seq.len() + 2);
    extensions.push(0x30); // SEQUENCE
    extensions.push(ext_seq.len() as u8);
    extensions.extend_from_slice(&ext_seq);

    Ok(extensions)
}

/// Converts a SystemTime to ASN.1 UTCTime bytes.
fn time_to_utctime(time: SystemTime) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use std::time::UNIX_EPOCH;
    let dur = time.duration_since(UNIX_EPOCH)?;
    let secs = dur.as_secs();

    // Convert to year, month, day, hour, minute, second
    // Simplified calculation (good enough for 2000-2099)
    let days = secs / 86400;
    let remaining_secs = secs % 86400;
    let hour = remaining_secs / 3600;
    let minute = (remaining_secs % 3600) / 60;
    let second = remaining_secs % 60;

    // Days since 1970-01-01 to Y-M-D (simplified)
    let mut y = 1970u32;
    let mut d = days;
    loop {
        let days_in_year = if is_leap_year(y) { 366 } else { 365 };
        if d < days_in_year as u64 {
            break;
        }
        d -= days_in_year as u64;
        y += 1;
    }

    let month_days = if is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0usize;
    for &days in &month_days {
        if d < days as u64 {
            break;
        }
        d -= days as u64;
        m += 1;
    }
    let day = d + 1;

    let year_short = (y % 100) as u8;
    let s = format!(
        "{:02}{:02}{:02}{:02}{:02}{:02}Z",
        year_short, m + 1, day, hour, minute, second
    );

    Ok(s.into_bytes())
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Generates a random 20-byte serial number.
fn rand_serial() -> [u8; 20] {
    use ring::rand::{SecureRandom, SystemRandom};
    let rng = SystemRandom::new();
    let mut serial = [0u8; 20];
    rng.fill(&mut serial).unwrap();
    // Ensure positive (clear high bit)
    serial[0] &= 0x7f;
    serial
}

// ---------------------------------------------------------------------------
// DER ECDSA signature encoding
// ---------------------------------------------------------------------------

/// Encodes a raw ECDSA P-256 signature (r||s, 64 bytes) as a DER-encoded signature.
fn encode_ecdsa_der(raw: &[u8; 64]) -> Vec<u8> {
    let r = &raw[..32];
    let s = &raw[32..];

    let r_der = encode_der_integer_positive(r);
    let s_der = encode_der_integer_positive(s);

    let mut result = Vec::with_capacity(2 + r_der.len() + s_der.len());
    result.push(0x30); // SEQUENCE
    result.push((r_der.len() + s_der.len()) as u8);
    result.extend_from_slice(&r_der);
    result.extend_from_slice(&s_der);
    result
}

// ---------------------------------------------------------------------------
// TLS server config builder
// ---------------------------------------------------------------------------

/// Builds a rustls `ServerConfig` with hardware-issued ephemeral leaf certificate.
///
/// The root CA certificate is loaded from `root_cert_path` and must match the
/// hardware signer's public key. An ephemeral leaf certificate is issued by
/// the root CA and used for TLS.
pub fn build_tls_server_config(
    root_cert_path: &std::path::Path,
    signer: Arc<dyn MeshSigner + Send + Sync>,
) -> Result<rustls::ServerConfig, Box<dyn std::error::Error + Send + Sync>> {
    let root_ca = RootCa::new(root_cert_path, signer)?;

    // Issue an ephemeral leaf certificate
    let node_id_hex = lifegraph_core::util::bytes_to_hex(&root_ca.signer.node_id().0);
    let (leaf_cert, eph_key) = root_ca.issue_leaf(&node_id_hex)?;

    let root_cert = root_ca.root_cert().clone();

    // Build the TLS config with the leaf cert + root cert chain
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf_cert, root_cert],
            rustls::pki_types::PrivateKeyDer::Pkcs8(eph_key),
        )?;

    tracing::info!(
        root_subject = ?extract_cert_subject(&root_ca.root_cert()),
        "TLS server config built with hardware-issued ephemeral leaf certificate"
    );

    Ok(config)
}

/// Extracts the subject DN from a certificate as a human-readable string.
fn extract_cert_subject(cert: &CertificateDer<'_>) -> String {
    use x509_parser::prelude::*;
    if let Ok((_, x509)) = X509Certificate::from_der(cert.as_ref()) {
        x509.subject().to_string()
    } else {
        "<unknown>".into()
    }
}

/// Extracts the subject DN from a certificate as raw DER bytes.
fn extract_subject_dn(cert: &CertificateDer<'_>) -> Result<Vec<u8>, String> {
    use x509_parser::prelude::*;
    let (_, x509) = X509Certificate::from_der(cert.as_ref())
        .map_err(|e| format!("failed to parse certificate: {}", e))?;
    Ok(x509.subject().as_raw().to_vec())
}
