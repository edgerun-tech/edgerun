//! Minimal X.509 certificate parsing for the TLS certificate fields we use.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use edgerun_encoding::byteorder::{read_u16_be, read_u24_be};

const OID_EC_PUBLIC_KEY: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
const OID_RSA_ENCRYPTION: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
const OID_ED25519: &[u8] = &[0x2B, 0x65, 0x70];
const OID_ECDSA_SHA256: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
const OID_ECDSA_SHA384: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x03];
const OID_ECDSA_SHA512: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x04];
const OID_SHA256_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
const OID_SHA384_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0C];
const OID_SHA512_WITH_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0D];
const OID_RSASSA_PSS: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0A];

/// Parsed X.509 certificate
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Raw DER bytes
    pub der: Vec<u8>,
    /// Subject Common Name (if present)
    pub subject_cn: Option<String>,
    /// Issuer Common Name (if present)
    pub issuer_cn: Option<String>,
    /// Validity: not before (Unix timestamp)
    pub not_before: u64,
    /// Validity: not after (Unix timestamp)
    pub not_after: u64,
    /// Subject public key bytes (uncompressed point for EC keys)
    pub subject_public_key: Vec<u8>,
    /// Subject public key algorithm OID
    pub subject_public_key_algorithm: Vec<u8>,
    /// Subject Alternative Names (DNS entries)
    pub subject_alt_names: Vec<String>,
    /// Issuer DER bytes (for chain validation)
    pub issuer_der: Vec<u8>,
    /// Subject DER bytes (for chain validation)
    pub subject_der: Vec<u8>,
    /// Signature algorithm OID
    pub signature_algorithm: Vec<u8>,
    /// Signature value bytes
    pub signature_value: Vec<u8>,
    /// TBSCertificate DER bytes (what gets signed)
    pub tbs_certificate_der: Vec<u8>,
}

impl Certificate {
    /// Parse a single certificate from DER bytes
    pub fn from_der(data: &[u8]) -> Result<Self, String> {
        parse_certificate_der(data)
    }

    /// Parse a certificate from PEM format
    pub fn from_pem(pem: &str) -> Result<Self, String> {
        let der = edgerun_crypto::x509_cert_from_pem(pem)
            .ok_or_else(|| "Failed to parse PEM certificate".to_string())?;
        Self::from_der(&der)
    }

    /// Parse multiple certificates from a TLS Certificate message payload.
    /// TLS 1.3 format (RFC 8446 §4.4.2):
    ///   context_len(1) + certificate_list_length(3) + CertificateEntry*
    ///   CertificateEntry: cert_data_len(3) + cert_data + ext_len(2) + ext
    pub fn parse_list(data: &[u8]) -> Result<Vec<Self>, String> {
        if data.is_empty() {
            return Err("Empty certificate list".into());
        }
        let context_len = data[0] as usize;
        if data.len() < 1 + context_len + 3 {
            return Err("Certificate list truncated".into());
        }
        let cert_list_len = read_u24_be(data, 1 + context_len) as usize;
        let list_start = 4 + context_len;
        if data.len() < list_start + cert_list_len {
            return Err("Certificate list truncated".into());
        }

        let list_data = &data[list_start..list_start + cert_list_len];
        let mut certs = Vec::new();
        let mut pos = 0;
        while pos + 5 < list_data.len() {
            let cert_data_len = read_u24_be(list_data, pos) as usize;
            pos += 3;
            if pos + cert_data_len + 2 > list_data.len() {
                break;
            }
            let cert_der = &list_data[pos..pos + cert_data_len];
            pos += cert_data_len;
            let ext_len = read_u16_be(list_data, pos) as usize;
            pos += 2 + ext_len;

            let cert = Self::from_der(cert_der)?;
            certs.push(cert);
        }
        Ok(certs)
    }

    /// Check if the certificate is currently valid
    pub fn is_valid_now(&self) -> bool {
        let Some(now) = unix_now_secs() else {
            return true;
        };
        now >= self.not_before && now <= self.not_after
    }

    /// Check if certificate matches the expected hostname.
    /// Per RFC 2818, SAN takes precedence over CN.
    pub fn matches_hostname(&self, hostname: &str) -> bool {
        if !self.subject_alt_names.is_empty() {
            for san_name in &self.subject_alt_names {
                if Self::hostname_matches(san_name, hostname) {
                    return true;
                }
            }
            return false;
        }
        if let Some(cn) = &self.subject_cn {
            if Self::hostname_matches(cn, hostname) {
                return true;
            }
        }
        false
    }

    /// Verify signature on the certificate against an issuer certificate.
    ///
    /// This performs actual cryptographic signature verification for supported
    /// issuer key/signature combinations: P-256 ECDSA, RSA PKCS#1 v1.5,
    /// RSA-PSS with SHA-256 defaults, and Ed25519.
    pub fn verify_signature(&self, issuer: &Certificate) -> Result<(), String> {
        if let Some(ref issuer_cn) = self.issuer_cn {
            if let Some(ref issuer_subject_cn) = issuer.subject_cn {
                if issuer_cn != issuer_subject_cn {
                    return Err(format!(
                        "Issuer CN mismatch: expected '{}', got '{}'",
                        issuer_cn, issuer_subject_cn
                    ));
                }
            } else {
                return Err("Issuer certificate has no subject CN".into());
            }
        } else {
            return Err("Certificate has no issuer CN".into());
        }

        if self.signature_value.is_empty() {
            return Err("Certificate signature value is empty".into());
        }

        verify_certificate_signature_with_issuer(self, issuer)
    }

    /// Hostname matching with wildcard support (RFC 2818)
    fn hostname_matches(cert_name: &str, hostname: &str) -> bool {
        if cert_name.eq_ignore_ascii_case(hostname) {
            return true;
        }
        // Wildcard: *.example.com matches anything.example.com
        if cert_name.starts_with("*.") && hostname.len() > cert_name.len() - 1 {
            let wildcard_suffix = &cert_name[1..];
            let left_label_len = hostname.len().saturating_sub(wildcard_suffix.len());
            if hostname.ends_with(wildcard_suffix) && !hostname[..left_label_len].contains('.') {
                return true;
            }
        }
        false
    }
}

fn verify_certificate_signature_with_issuer(
    cert: &Certificate,
    issuer: &Certificate,
) -> Result<(), String> {
    match issuer.subject_public_key_algorithm.as_slice() {
        OID_EC_PUBLIC_KEY => verify_ecdsa_certificate_signature(cert, issuer),
        OID_RSA_ENCRYPTION => verify_rsa_certificate_signature(cert, issuer),
        OID_ED25519 => verify_ed25519_certificate_signature(cert, issuer),
        alg => Err(format!("Unsupported issuer public key algorithm: {alg:?}")),
    }
}

fn verify_ecdsa_certificate_signature(
    cert: &Certificate,
    issuer: &Certificate,
) -> Result<(), String> {
    let (hash, hasher_name) = match cert.signature_algorithm.as_slice() {
        OID_ECDSA_SHA256 => {
            use edgerun_crypto::sha::Digest;
            let mut hasher = edgerun_crypto::sha::Sha256::new();
            hasher.update(&cert.tbs_certificate_der);
            (hasher.finalize().to_vec(), "SHA-256")
        }
        OID_ECDSA_SHA384 => {
            use edgerun_crypto::sha::Digest;
            let mut hasher = edgerun_crypto::sha::Sha384::new();
            hasher.update(&cert.tbs_certificate_der);
            (hasher.finalize().to_vec(), "SHA-384")
        }
        OID_ECDSA_SHA512 => {
            use edgerun_crypto::sha::Digest;
            let mut hasher = edgerun_crypto::sha::Sha512::new();
            hasher.update(&cert.tbs_certificate_der);
            (hasher.finalize().to_vec(), "SHA-512")
        }
        alg => return Err(format!("Unsupported ECDSA signature algorithm: {alg:?}")),
    };

    let verifying_key = edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(
        issuer.subject_public_key.as_slice(),
    )
    .map_err(|e| format!("Failed to parse issuer ECDSA public key: {e}"))?;
    let signature = edgerun_crypto::p256::ecdsa::Signature::from_der(&cert.signature_value)
        .map_err(|e| format!("Failed to parse ECDSA signature: {e}"))?;

    use edgerun_crypto::signature::hazmat::PrehashVerifier;
    verifying_key
        .verify_prehash(&hash, &signature)
        .map_err(|e| format!("ECDSA signature verification failed ({hasher_name} over P-256): {e}"))
}

fn verify_rsa_certificate_signature(
    cert: &Certificate,
    issuer: &Certificate,
) -> Result<(), String> {
    use edgerun_crypto::rsa::pkcs1::DecodeRsaPublicKey;

    let public_key = edgerun_crypto::rsa::RsaPublicKey::from_pkcs1_der(&issuer.subject_public_key)
        .map_err(|e| format!("Failed to parse issuer RSA public key: {e}"))?;

    match cert.signature_algorithm.as_slice() {
        OID_SHA256_WITH_RSA => verify_rsa_pkcs1_sha256(
            &public_key,
            &cert.tbs_certificate_der,
            &cert.signature_value,
        ),
        OID_SHA384_WITH_RSA => verify_rsa_pkcs1_sha384(
            &public_key,
            &cert.tbs_certificate_der,
            &cert.signature_value,
        ),
        OID_SHA512_WITH_RSA => verify_rsa_pkcs1_sha512(
            &public_key,
            &cert.tbs_certificate_der,
            &cert.signature_value,
        ),
        OID_RSASSA_PSS => verify_rsa_pss_sha256(
            &public_key,
            &cert.tbs_certificate_der,
            &cert.signature_value,
        ),
        alg => Err(format!(
            "Unsupported RSA certificate signature algorithm: {alg:?}"
        )),
    }
}

fn verify_rsa_pkcs1_sha256(
    public_key: &edgerun_crypto::rsa::RsaPublicKey,
    message: &[u8],
    signature: &[u8],
) -> Result<(), String> {
    let verifying_key = edgerun_crypto::rsa::pkcs1v15::VerifyingKey::<
        edgerun_crypto::rsa::sha2::Sha256,
    >::new(public_key.clone());
    let signature = edgerun_crypto::rsa::pkcs1v15::Signature::try_from(signature)
        .map_err(|e| format!("Failed to parse RSA PKCS#1 signature: {e}"))?;
    use edgerun_crypto::rsa::signature::Verifier;
    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("RSA PKCS#1 SHA-256 certificate signature verification failed: {e}"))
}

fn verify_rsa_pkcs1_sha384(
    public_key: &edgerun_crypto::rsa::RsaPublicKey,
    message: &[u8],
    signature: &[u8],
) -> Result<(), String> {
    let verifying_key = edgerun_crypto::rsa::pkcs1v15::VerifyingKey::<
        edgerun_crypto::rsa::sha2::Sha384,
    >::new(public_key.clone());
    let signature = edgerun_crypto::rsa::pkcs1v15::Signature::try_from(signature)
        .map_err(|e| format!("Failed to parse RSA PKCS#1 signature: {e}"))?;
    use edgerun_crypto::rsa::signature::Verifier;
    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("RSA PKCS#1 SHA-384 certificate signature verification failed: {e}"))
}

fn verify_rsa_pkcs1_sha512(
    public_key: &edgerun_crypto::rsa::RsaPublicKey,
    message: &[u8],
    signature: &[u8],
) -> Result<(), String> {
    let verifying_key = edgerun_crypto::rsa::pkcs1v15::VerifyingKey::<
        edgerun_crypto::rsa::sha2::Sha512,
    >::new(public_key.clone());
    let signature = edgerun_crypto::rsa::pkcs1v15::Signature::try_from(signature)
        .map_err(|e| format!("Failed to parse RSA PKCS#1 signature: {e}"))?;
    use edgerun_crypto::rsa::signature::Verifier;
    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("RSA PKCS#1 SHA-512 certificate signature verification failed: {e}"))
}

fn verify_rsa_pss_sha256(
    public_key: &edgerun_crypto::rsa::RsaPublicKey,
    message: &[u8],
    signature: &[u8],
) -> Result<(), String> {
    let verifying_key =
        edgerun_crypto::rsa::pss::VerifyingKey::<edgerun_crypto::rsa::sha2::Sha256>::new(
            public_key.clone(),
        );
    let signature = edgerun_crypto::rsa::pss::Signature::try_from(signature)
        .map_err(|e| format!("Failed to parse RSA-PSS signature: {e}"))?;
    use edgerun_crypto::rsa::signature::Verifier;
    verifying_key
        .verify(message, &signature)
        .map_err(|e| format!("RSA-PSS SHA-256 certificate signature verification failed: {e}"))
}

fn verify_ed25519_certificate_signature(
    cert: &Certificate,
    issuer: &Certificate,
) -> Result<(), String> {
    if cert.signature_algorithm.as_slice() != OID_ED25519 {
        return Err(format!(
            "Unsupported Ed25519 certificate signature algorithm: {:?}",
            cert.signature_algorithm
        ));
    }
    let public_key: [u8; 32] = issuer
        .subject_public_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid Ed25519 public key length".to_string())?;
    let verifying_key = edgerun_crypto::ed25519_dalek::VerifyingKey::from_bytes(&public_key)
        .map_err(|e| format!("Failed to parse issuer Ed25519 public key: {e}"))?;
    let signature = edgerun_crypto::ed25519_dalek::Signature::from_slice(&cert.signature_value)
        .map_err(|e| format!("Failed to parse Ed25519 signature: {e}"))?;
    use edgerun_crypto::ed25519_dalek::Verifier;
    verifying_key
        .verify(&cert.tbs_certificate_der, &signature)
        .map_err(|e| format!("Ed25519 certificate signature verification failed: {e}"))
}

#[derive(Clone, Copy)]
struct DerNode<'a> {
    tag: u8,
    full: &'a [u8],
    value: &'a [u8],
}

struct DerReader<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> DerReader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn next(&mut self) -> Result<Option<DerNode<'a>>, String> {
        if self.pos == self.input.len() {
            return Ok(None);
        }
        let start = self.pos;
        if self.input.len().saturating_sub(self.pos) < 2 {
            return Err("Failed to parse X.509 certificate: DER object truncated".into());
        }
        let tag = self.input[self.pos];
        self.pos += 1;
        let len_first = self.input[self.pos];
        self.pos += 1;
        let len = if len_first & 0x80 == 0 {
            len_first as usize
        } else {
            let len_len = (len_first & 0x7f) as usize;
            if len_len == 0 || len_len > core::mem::size_of::<usize>() {
                return Err("Failed to parse X.509 certificate: invalid DER length".into());
            }
            if self.input.len().saturating_sub(self.pos) < len_len {
                return Err("Failed to parse X.509 certificate: DER length truncated".into());
            }
            let mut len = 0usize;
            for byte in &self.input[self.pos..self.pos + len_len] {
                len = (len << 8) | (*byte as usize);
            }
            self.pos += len_len;
            len
        };
        let value_start = self.pos;
        let end = value_start
            .checked_add(len)
            .ok_or_else(|| "Failed to parse X.509 certificate: DER length overflow".to_string())?;
        if end > self.input.len() {
            return Err("Failed to parse X.509 certificate: DER value truncated".into());
        }
        self.pos = end;
        Ok(Some(DerNode {
            tag,
            full: &self.input[start..end],
            value: &self.input[value_start..end],
        }))
    }

    fn expect(&mut self, tag: u8, what: &str) -> Result<DerNode<'a>, String> {
        let node = self
            .next()?
            .ok_or_else(|| format!("Failed to parse X.509 certificate: missing {what}"))?;
        if node.tag != tag {
            return Err(format!(
                "Failed to parse X.509 certificate: expected {what}"
            ));
        }
        Ok(node)
    }
}

fn parse_certificate_der(data: &[u8]) -> Result<Certificate, String> {
    let cert = only_node(data, 0x30, "certificate")?;
    let mut cert_reader = DerReader::new(cert.value);
    let tbs = cert_reader.expect(0x30, "TBSCertificate")?;
    let sig_alg = cert_reader.expect(0x30, "signatureAlgorithm")?;
    let signature = cert_reader.expect(0x03, "signatureValue")?;
    if cert_reader.next()?.is_some() {
        return Err("Failed to parse X.509 certificate: trailing certificate data".into());
    }

    let signature_algorithm = first_oid(sig_alg.value).unwrap_or_default();
    let signature_value = bit_string_bytes(signature.value)?.to_vec();

    let mut tbs_reader = DerReader::new(tbs.value);
    let first = tbs_reader
        .next()?
        .ok_or_else(|| "Failed to parse X.509 certificate: empty TBSCertificate".to_string())?;
    if first.tag != 0xa0 {
        parse_tbs_after_version(
            data,
            tbs,
            first,
            &mut tbs_reader,
            signature_algorithm,
            signature_value,
        )
    } else {
        let serial = tbs_reader
            .next()?
            .ok_or_else(|| "Failed to parse X.509 certificate: missing serial".to_string())?;
        parse_tbs_after_serial(
            data,
            tbs,
            serial,
            &mut tbs_reader,
            signature_algorithm,
            signature_value,
        )
    }
}

fn parse_tbs_after_version(
    data: &[u8],
    tbs: DerNode<'_>,
    serial: DerNode<'_>,
    tbs_reader: &mut DerReader<'_>,
    signature_algorithm: Vec<u8>,
    signature_value: Vec<u8>,
) -> Result<Certificate, String> {
    parse_tbs_after_serial(
        data,
        tbs,
        serial,
        tbs_reader,
        signature_algorithm,
        signature_value,
    )
}

fn parse_tbs_after_serial(
    data: &[u8],
    tbs: DerNode<'_>,
    _serial: DerNode<'_>,
    tbs_reader: &mut DerReader<'_>,
    signature_algorithm: Vec<u8>,
    signature_value: Vec<u8>,
) -> Result<Certificate, String> {
    let _tbs_sig_alg = tbs_reader.expect(0x30, "TBS signature")?;
    let issuer = tbs_reader.expect(0x30, "issuer")?;
    let validity = tbs_reader.expect(0x30, "validity")?;
    let subject = tbs_reader.expect(0x30, "subject")?;
    let spki = tbs_reader.expect(0x30, "subjectPublicKeyInfo")?;

    let issuer_cn = extract_cn(issuer.value);
    let subject_cn = extract_cn(subject.value);
    let (not_before, not_after) = parse_validity(validity.value)?;
    let (subject_public_key_algorithm, subject_public_key) = parse_spki_public_key(spki.value)?;

    let mut subject_alt_names = Vec::new();
    while let Some(node) = tbs_reader.next()? {
        if node.tag == 0xa3 {
            subject_alt_names = parse_extensions(node.value);
        }
    }

    Ok(Certificate {
        der: data.to_vec(),
        subject_cn,
        issuer_cn,
        not_before,
        not_after,
        subject_public_key,
        subject_public_key_algorithm,
        subject_alt_names,
        issuer_der: issuer.full.to_vec(),
        subject_der: subject.full.to_vec(),
        signature_algorithm,
        signature_value,
        tbs_certificate_der: tbs.full.to_vec(),
    })
}

fn only_node<'a>(data: &'a [u8], tag: u8, what: &str) -> Result<DerNode<'a>, String> {
    let mut reader = DerReader::new(data);
    let node = reader.expect(tag, what)?;
    if reader.next()?.is_some() {
        return Err(format!(
            "Failed to parse X.509 certificate: trailing {what} data"
        ));
    }
    Ok(node)
}

fn first_oid(data: &[u8]) -> Option<Vec<u8>> {
    let mut reader = DerReader::new(data);
    while let Ok(Some(node)) = reader.next() {
        if node.tag == 0x06 {
            return Some(node.value.to_vec());
        }
    }
    None
}

fn bit_string_bytes(value: &[u8]) -> Result<&[u8], String> {
    if value.is_empty() {
        return Err("Failed to parse X.509 certificate: empty BIT STRING".into());
    }
    if value[0] != 0 {
        return Err("Failed to parse X.509 certificate: unsupported BIT STRING padding".into());
    }
    Ok(&value[1..])
}

fn parse_spki_public_key(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut reader = DerReader::new(data);
    let alg = reader.expect(0x30, "SPKI algorithm")?;
    let public_key = reader.expect(0x03, "SPKI public key")?;
    let alg_oid = first_oid(alg.value).ok_or_else(|| {
        "Failed to parse X.509 certificate: missing SPKI algorithm OID".to_string()
    })?;
    Ok((alg_oid, bit_string_bytes(public_key.value)?.to_vec()))
}

fn parse_validity(data: &[u8]) -> Result<(u64, u64), String> {
    let mut reader = DerReader::new(data);
    let not_before = reader
        .next()?
        .ok_or_else(|| "Failed to parse X.509 certificate: missing notBefore".to_string())?;
    let not_after = reader
        .next()?
        .ok_or_else(|| "Failed to parse X.509 certificate: missing notAfter".to_string())?;
    Ok((parse_time(not_before)?, parse_time(not_after)?))
}

fn parse_time(node: DerNode<'_>) -> Result<u64, String> {
    match node.tag {
        0x17 => parse_utc_time(node.value),
        0x18 => parse_generalized_time(node.value),
        _ => Err("Failed to parse X.509 certificate: unsupported time tag".into()),
    }
}

fn parse_utc_time(value: &[u8]) -> Result<u64, String> {
    if value.len() != 13 || value[12] != b'Z' {
        return Err("Failed to parse X.509 certificate: invalid UTCTime".into());
    }
    let year = two_digits(&value[0..2])?;
    let year = if year >= 50 { 1900 + year } else { 2000 + year };
    unix_from_ymdhms(
        year as i32,
        two_digits(&value[2..4])? as u32,
        two_digits(&value[4..6])? as u32,
        two_digits(&value[6..8])? as u32,
        two_digits(&value[8..10])? as u32,
        two_digits(&value[10..12])? as u32,
    )
}

fn parse_generalized_time(value: &[u8]) -> Result<u64, String> {
    if value.len() != 15 || value[14] != b'Z' {
        return Err("Failed to parse X.509 certificate: invalid GeneralizedTime".into());
    }
    let year = (two_digits(&value[0..2])? * 100 + two_digits(&value[2..4])?) as i32;
    unix_from_ymdhms(
        year,
        two_digits(&value[4..6])? as u32,
        two_digits(&value[6..8])? as u32,
        two_digits(&value[8..10])? as u32,
        two_digits(&value[10..12])? as u32,
        two_digits(&value[12..14])? as u32,
    )
}

fn two_digits(bytes: &[u8]) -> Result<u64, String> {
    if bytes.len() != 2 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return Err("Failed to parse X.509 certificate: invalid decimal time field".into());
    }
    Ok(((bytes[0] - b'0') as u64) * 10 + (bytes[1] - b'0') as u64)
}

fn unix_from_ymdhms(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> Result<u64, String> {
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return Err("Failed to parse X.509 certificate: invalid time value".into());
    }
    let days = days_from_civil(year, month, day);
    if days < 0 {
        return Err("Failed to parse X.509 certificate: time before Unix epoch".into());
    }
    Ok(days as u64 * 86_400 + hour as u64 * 3_600 + minute as u64 * 60 + second as u64)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = year as i64;
    let m = month as i64;
    let d = day as i64;
    y -= (m <= 2) as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn extract_cn(name_der_value: &[u8]) -> Option<String> {
    let mut rdns = DerReader::new(name_der_value);
    while let Ok(Some(rdn)) = rdns.next() {
        if rdn.tag != 0x31 {
            continue;
        }
        let mut attrs = DerReader::new(rdn.value);
        while let Ok(Some(attr)) = attrs.next() {
            if attr.tag != 0x30 {
                continue;
            }
            let mut parts = DerReader::new(attr.value);
            let oid = parts.next().ok().flatten()?;
            let value = parts.next().ok().flatten()?;
            if oid.tag == 0x06 && oid.value == [0x55, 0x04, 0x03] {
                return string_value(value);
            }
        }
    }
    None
}

fn string_value(node: DerNode<'_>) -> Option<String> {
    match node.tag {
        0x0c | 0x13 | 0x16 => core::str::from_utf8(node.value)
            .ok()
            .map(ToString::to_string),
        _ => None,
    }
}

fn parse_extensions(data: &[u8]) -> Vec<String> {
    let Ok(exts) = only_node(data, 0x30, "extensions") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut reader = DerReader::new(exts.value);
    while let Ok(Some(ext)) = reader.next() {
        if ext.tag != 0x30 {
            continue;
        }
        let mut parts = DerReader::new(ext.value);
        let Ok(Some(oid)) = parts.next() else {
            continue;
        };
        if oid.tag != 0x06 || oid.value != [0x55, 0x1d, 0x11] {
            continue;
        }
        let mut value = match parts.next() {
            Ok(Some(node)) if node.tag == 0x01 => match parts.next() {
                Ok(Some(value)) => value,
                _ => continue,
            },
            Ok(Some(value)) => value,
            _ => continue,
        };
        if value.tag != 0x04 {
            continue;
        }
        out.extend(parse_san_dns_names(value.value));
    }
    out
}

fn parse_san_dns_names(extn_value: &[u8]) -> Vec<String> {
    let Ok(names) = only_node(extn_value, 0x30, "subjectAltName") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut reader = DerReader::new(names.value);
    while let Ok(Some(name)) = reader.next() {
        if name.tag == 0x82 {
            if let Ok(dns) = core::str::from_utf8(name.value) {
                out.push(dns.to_string());
            }
        }
    }
    out
}

fn unix_now_secs() -> Option<u64> {
    let now = edgerun_rt::now() / 10_000_000;
    (now != 0).then_some(now)
}
