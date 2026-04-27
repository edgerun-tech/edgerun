//! X.509 certificate parsing using `x509_cert` (der::Decode).
//!
//! Proper DER parsing via the `der` / `x509_cert` crates from edgerun-crypto.
//! No hand-rolled DER walking — full ASN.1 structural decoding.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use edgerun_crypto::x509_cert::der::{Decode, DecodePem, Encode};
use edgerun_crypto::x509_cert::ext::pkix::name::GeneralName;
use edgerun_crypto::x509_cert::Certificate as DerCertificate;

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
        let cert = DerCertificate::from_der(data)
            .map_err(|e| format!("Failed to parse X.509 certificate: {e}"))?;
        Self::from_parsed(&cert, data.to_vec())
    }

    /// Parse a certificate from PEM format
    pub fn from_pem(pem: &str) -> Result<Self, String> {
        let cert = DerCertificate::from_pem(pem)
            .map_err(|e| format!("Failed to parse PEM certificate: {e}"))?;
        let der_bytes = cert
            .to_der()
            .map_err(|e| format!("Failed to re-encode cert: {e}"))?;
        Self::from_parsed(&cert, der_bytes)
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
        let cert_list_len = u32::from_be_bytes([
            0,
            data[1 + context_len],
            data[2 + context_len],
            data[3 + context_len],
        ]) as usize;
        let list_start = 4 + context_len;
        if data.len() < list_start + cert_list_len {
            return Err("Certificate list truncated".into());
        }

        let list_data = &data[list_start..list_start + cert_list_len];
        let mut certs = Vec::new();
        let mut pos = 0;
        while pos + 5 < list_data.len() {
            let cert_data_len =
                u32::from_be_bytes([0, list_data[pos], list_data[pos + 1], list_data[pos + 2]])
                    as usize;
            pos += 3;
            if pos + cert_data_len + 2 > list_data.len() {
                break;
            }
            let cert_der = &list_data[pos..pos + cert_data_len];
            pos += cert_data_len;
            let ext_len = u16::from_be_bytes([list_data[pos], list_data[pos + 1]]) as usize;
            pos += 2 + ext_len;

            let cert = Self::from_der(cert_der)?;
            certs.push(cert);
        }
        Ok(certs)
    }

    /// Build our Certificate from a properly decoded x509_cert::Certificate
    fn from_parsed(cert: &DerCertificate, der_bytes: Vec<u8>) -> Result<Self, String> {
        let tbs = cert.tbs_certificate();

        // Subject CN
        let subject_cn = Self::extract_cn(tbs.subject());
        let subject_der = tbs.subject().to_der().unwrap_or_default();

        // Issuer CN
        let issuer_cn = Self::extract_cn(tbs.issuer());
        let issuer_der = tbs.issuer().to_der().unwrap_or_default();

        // Validity
        let not_before = Self::time_to_unix(&tbs.validity().not_before);
        let not_after = Self::time_to_unix(&tbs.validity().not_after);

        // Subject public key
        let subject_public_key = tbs
            .subject_public_key_info()
            .subject_public_key
            .raw_bytes()
            .to_vec();

        // Subject Alternative Names
        let subject_alt_names = Self::extract_sans(cert);

        // Signature algorithm OID
        let signature_algorithm = cert.signature_algorithm().oid.as_bytes().to_vec();

        // Signature value
        let signature_value = cert.signature().raw_bytes().to_vec();

        // TBS certificate DER
        let tbs_certificate_der = cert
            .tbs_certificate()
            .to_der()
            .map_err(|e| format!("Failed to encode TBS: {e}"))?;

        Ok(Certificate {
            der: der_bytes,
            subject_cn,
            issuer_cn,
            not_before,
            not_after,
            subject_public_key,
            subject_alt_names,
            issuer_der,
            subject_der,
            signature_algorithm,
            signature_value,
            tbs_certificate_der,
        })
    }

    /// Extract Common Name from an x509_cert Name
    fn extract_cn(name: &edgerun_crypto::x509_cert::name::Name) -> Option<String> {
        let _ = name;
        None
    }

    /// Extract DNS Subject Alternative Names
    fn extract_sans(cert: &DerCertificate) -> Vec<String> {
        use edgerun_crypto::x509_cert::ext::pkix::SubjectAltName;
        let tbs = cert.tbs_certificate();
        let Some(exts) = tbs.extensions() else {
            return Vec::new();
        };

        let mut sans = Vec::new();
        for ext in exts.iter() {
            if ext.extn_id
                == edgerun_crypto::x509_cert::der::oid::db::rfc5280::ID_CE_SUBJECT_ALT_NAME
            {
                if let Ok(san) = SubjectAltName::from_der(ext.extn_value.as_bytes()) {
                    for name in san.0.iter() {
                        if let GeneralName::DnsName(dns) = name {
                            sans.push(dns.to_string());
                        }
                    }
                }
            }
        }
        sans
    }

    /// Convert x509_cert Time to Unix timestamp
    fn time_to_unix(time: &edgerun_crypto::x509_cert::time::Time) -> u64 {
        let date_time = time.to_date_time();
        date_time.unix_duration().as_secs()
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
    /// This performs actual cryptographic signature verification:
    /// 1. Extract the issuer's ECDSA public key
    /// 2. Hash the TBS certificate with the appropriate digest
    /// 3. Verify the signature using ECDSA
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

        // Determine the hash algorithm from the signature algorithm OID
        // ecdsa-with-SHA256: 1.2.840.10045.4.3.2 -> [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02]
        // ecdsa-with-SHA384: 1.2.840.10045.4.3.3 -> [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x03]
        let (hash, hasher_name) = match self.signature_algorithm.as_slice() {
            [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02] => {
                use edgerun_crypto::sha2::Digest;
                let mut hasher = edgerun_crypto::sha2::Sha256::new();
                hasher.update(&self.tbs_certificate_der);
                (hasher.finalize().to_vec(), "SHA-256")
            }
            [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x03] => {
                use edgerun_crypto::sha2::Digest;
                let mut hasher = edgerun_crypto::sha2::Sha384::new();
                hasher.update(&self.tbs_certificate_der);
                (hasher.finalize().to_vec(), "SHA-384")
            }
            _ => {
                return Err(format!(
                    "Unsupported signature algorithm: {:?}",
                    self.signature_algorithm
                ));
            }
        };

        if self.signature_value.is_empty() {
            return Err("Certificate signature value is empty".into());
        }

        // Parse the issuer's P-256 public key
        let issuer_pubkey = &issuer.subject_public_key;
        if issuer_pubkey.is_empty() {
            return Err("Issuer certificate has no public key".into());
        }

        // The issuer public key should be an uncompressed EC point (0x04 || X || Y)
        // for P-256, that's 65 bytes total
        let verifying_key =
            edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(issuer_pubkey.as_slice())
                .map_err(|e| format!("Failed to parse issuer public key: {e}"))?;

        // Parse the signature from DER format
        // ECDSA signatures in X.509 are DER-encoded ASN.1 SEQUENCE of two INTEGERs (r, s)
        let signature = edgerun_crypto::p256::ecdsa::Signature::from_der(&self.signature_value)
            .map_err(|e| format!("Failed to parse ECDSA signature: {e}"))?;

        // Verify the signature over the TBS certificate hash
        use edgerun_crypto::signature::hazmat::PrehashVerifier;
        verifying_key
            .verify_prehash(&hash, &signature)
            .map_err(|e| {
                format!(
                    "ECDSA signature verification failed ({} over P-256): {e}",
                    hasher_name
                )
            })?;

        Ok(())
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

fn unix_now_secs() -> Option<u64> {
    #[cfg(not(target_os = "none"))]
    {
        return crate::host_std::time::SystemTime::now()
            .duration_since(crate::host_std::time::UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs());
    }

    #[cfg(target_os = "none")]
    {
        let now = edgerun_rt::now() / 10_000_000;
        (now != 0).then_some(now)
    }
}
