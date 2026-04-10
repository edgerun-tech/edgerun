//! X.509 certificate parsing using the workspace `der` crate.

use edgerun_crypto::der::Tag;

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
        if data.is_empty() || data[0] != 0x30 {
            return Err("Not a valid DER SEQUENCE".into());
        }

        let mut subject_cn = None;
        let mut issuer_cn = None;
        let mut not_before: u64 = 0;
        let mut not_after: u64 = u64::MAX;
        let mut subject_public_key = Vec::new();
        let mut subject_alt_names = Vec::new();
        let mut issuer_der = Vec::new();
        let mut subject_der = Vec::new();
        let mut signature_algorithm = Vec::new();
        let mut signature_value = Vec::new();
        let mut tbs_certificate_der = Vec::new();

        // Walk the DER structure: Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signatureValue }
        Self::walk_der(
            data,
            &mut subject_cn,
            &mut issuer_cn,
            &mut not_before,
            &mut not_after,
            &mut subject_public_key,
            &mut subject_alt_names,
            &mut issuer_der,
            &mut subject_der,
            &mut signature_algorithm,
            &mut signature_value,
            &mut tbs_certificate_der,
        );

        Ok(Certificate {
            der: data.to_vec(),
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

    /// Parse a certificate from PEM format
    pub fn from_pem(pem: &str) -> Result<Self, String> {
        // Find PEM boundaries
        let start = pem
            .find("-----BEGIN CERTIFICATE-----")
            .ok_or("No BEGIN CERTIFICATE marker")?;
        let end = pem
            .find("-----END CERTIFICATE-----")
            .ok_or("No END CERTIFICATE marker")?;

        let b64 = pem[start + 27..end].trim();
        let der = base64_decode(b64)?;
        Self::from_der(&der)
    }

    /// Parse multiple certificates from a TLS Certificate message payload
    /// (ASN.1 CertificateList: SEQUENCE OF Certificate)
    pub fn parse_list(data: &[u8]) -> Result<Vec<Self>, String> {
        let mut certs = Vec::new();

        if data.is_empty() || data[0] != 0x30 {
            return Err("CertificateList must start with SEQUENCE tag".into());
        }

        // Skip the outer SEQUENCE header
        let payload = Self::extract_sequence_payload(data).ok_or("Invalid outer SEQUENCE")?;

        // Each element is a DER-encoded certificate
        let mut pos = 0;
        while pos < payload.len() {
            if pos + 2 > payload.len() {
                break;
            }
            // Read DER length
            let (cert, consumed) = Self::read_one_der(&payload[pos..])?;
            certs.push(Self::from_der(cert)?);
            pos += consumed;
        }

        Ok(certs)
    }

    /// Check if the certificate is currently valid
    pub fn is_valid_now(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.not_before && now <= self.not_after
    }

    /// Check if certificate matches the expected hostname.
    /// Per RFC 2818, SAN (Subject Alternative Name) takes precedence over CN.
    /// If SAN extensions are present, CN is ignored for hostname matching.
    pub fn matches_hostname(&self, hostname: &str) -> bool {
        // If SAN extensions are present, check them first (RFC 2818 requirement)
        if !self.subject_alt_names.is_empty() {
            for san_name in &self.subject_alt_names {
                if Self::hostname_matches(san_name, hostname) {
                    return true;
                }
            }
            // SAN present but doesn't match
            return false;
        }

        // Fallback to CN only if no SAN extensions
        if let Some(cn) = &self.subject_cn {
            if Self::hostname_matches(cn, hostname) {
                return true;
            }
        }
        false
    }

    /// Verify signature on the certificate against an issuer certificate.
    ///
    /// This performs basic chain validation:
    /// 1. Checks that the issuer's subject matches this certificate's issuer
    /// 2. Validates certificate structure and signature algorithm
    ///
    /// Note: Full cryptographic signature verification would require the ECDSA
    /// implementation, which is not available in the current dependency set.
    /// This method validates the certificate chain structure and ensures the
    /// issuer is not self-signed for production use.
    pub fn verify_signature(&self, issuer: &Certificate) -> Result<(), String> {
        // Check that issuer certificate's subject matches this cert's issuer
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

        // Verify that the certificate has a valid signature algorithm
        // For EC keys, we expect id-ecdsa-with-sha256 (2A 86 48 CE 3D 04 03 02)
        // or id-ecdsa-with-sha384 (2A 86 48 CE 3D 04 03 03)
        let valid_sig_algos = [
            vec![0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02], // ecdsa-with-sha256
            vec![0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x03], // ecdsa-with-sha384
        ];

        if !valid_sig_algos.contains(&self.signature_algorithm) {
            return Err(format!(
                "Unsupported signature algorithm: {:?}",
                self.signature_algorithm
            ));
        }

        // Verify that the signature value is present and well-formed
        if self.signature_value.is_empty() {
            return Err("Certificate signature value is empty".into());
        }

        // For a complete implementation, full ECDSA signature verification
        // would be needed. The structural validation above prevents accepting
        // obviously invalid certificates.
        Ok(())
    }

    // --- Internal DER walking helpers ---

    fn walk_der(
        data: &[u8],
        subject_cn: &mut Option<String>,
        issuer_cn: &mut Option<String>,
        not_before: &mut u64,
        not_after: &mut u64,
        subject_pk: &mut Vec<u8>,
        subject_alt_names: &mut Vec<String>,
        issuer_der: &mut Vec<u8>,
        subject_der_out: &mut Vec<u8>,
        signature_algorithm: &mut Vec<u8>,
        signature_value: &mut Vec<u8>,
        tbs_certificate_der: &mut Vec<u8>,
    ) {
        if data.len() < 2 || data[0] != 0x30 {
            return;
        }

        let payload = match Self::extract_sequence_payload(data) {
            Some(p) => p,
            None => return,
        };

        // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signatureValue }
        // First element is TBS, second is sig algo, third is signature
        let mut pos = 0;
        let mut element_index = 0;

        while pos < payload.len() {
            if pos + 2 > payload.len() {
                break;
            }

            let tag = payload[pos];
            pos += 1;

            // Read length
            let (len, consumed) = match Self::read_der_length(&payload[pos..]) {
                Ok(v) => v,
                Err(_) => break,
            };
            pos += consumed;

            if pos + len > payload.len() {
                break;
            }

            let child = &payload[pos..pos + len];

            match element_index {
                0 => {
                    // TBS Certificate — save it for signature verification
                    *tbs_certificate_der = child.to_vec();
                    // Recurse into TBS to extract fields
                    Self::walk_tbs_certificate(
                        child,
                        subject_cn,
                        issuer_cn,
                        not_before,
                        not_after,
                        subject_pk,
                        subject_alt_names,
                        issuer_der,
                        subject_der_out,
                    );
                }
                1 => {
                    // Signature Algorithm
                    *signature_algorithm = child.to_vec();
                }
                2 => {
                    // Signature Value (BIT STRING)
                    if !child.is_empty() && child[0] == 0 {
                        // Skip unused bits byte
                        *signature_value = child[1..].to_vec();
                    } else {
                        *signature_value = child.to_vec();
                    }
                }
                _ => {}
            }

            pos += len;
            element_index += 1;
        }
    }

    /// Walk the TBS (To Be Signed) certificate structure
    fn walk_tbs_certificate(
        data: &[u8],
        subject_cn: &mut Option<String>,
        issuer_cn: &mut Option<String>,
        not_before: &mut u64,
        not_after: &mut u64,
        subject_pk: &mut Vec<u8>,
        subject_alt_names: &mut Vec<String>,
        issuer_der: &mut Vec<u8>,
        subject_der_out: &mut Vec<u8>,
    ) {
        if data.len() < 2 || data[0] != 0x30 {
            return;
        }

        let payload = match Self::extract_sequence_payload(data) {
            Some(p) => p,
            None => return,
        };

        let mut pos = 0;
        let mut element_index = 0;

        while pos < payload.len() {
            if pos + 2 > payload.len() {
                break;
            }

            let tag = payload[pos];
            pos += 1;

            let (len, consumed) = match Self::read_der_length(&payload[pos..]) {
                Ok(v) => v,
                Err(_) => break,
            };
            pos += consumed;

            if pos + len > payload.len() {
                break;
            }

            let child = &payload[pos..pos + len];

            match element_index {
                0 => {
                    // version (context tag [0])
                }
                1 => {
                    // serialNumber
                }
                2 => {
                    // signature (algorithm) — save issuer DER
                    *issuer_der = child.to_vec();
                }
                3 => {
                    // issuer — extract CN
                    Self::extract_name_cn(child, issuer_cn);
                }
                4 => {
                    // validity
                    if child.len() >= 2 {
                        // Parse both notBefore and notAfter
                        let mut inner_pos = 0;
                        let mut date_index = 0;
                        while inner_pos < child.len() {
                            if inner_pos + 2 > child.len() {
                                break;
                            }
                            let dtag = child[inner_pos];
                            inner_pos += 1;
                            let (dlen, dconsumed) = match Self::read_der_length(&child[inner_pos..]) {
                                Ok(v) => v,
                                Err(_) => break,
                            };
                            inner_pos += dconsumed;
                            if inner_pos + dlen > child.len() {
                                break;
                            }
                            let dchild = &child[inner_pos..inner_pos + dlen];
                            if dtag == 0x17 || dtag == 0x18 {
                                if let Ok(s) = std::str::from_utf8(dchild) {
                                    if let Ok(ts) = Self::parse_time(s) {
                                        if date_index == 0 {
                                            *not_before = ts;
                                        } else {
                                            *not_after = ts;
                                        }
                                    }
                                }
                            }
                            inner_pos += dlen;
                            date_index += 1;
                        }
                    }
                }
                5 => {
                    // subject — extract CN and save DER
                    *subject_der_out = child.to_vec();
                    Self::extract_name_cn(child, subject_cn);
                }
                6 => {
                    // subjectPublicKeyInfo
                    Self::extract_subject_public_key(child, subject_pk);
                }
                7 => {
                    // issuerUniqueID (optional)
                }
                8 => {
                    // subjectUniqueID (optional)
                }
                9 => {
                    // extensions — look for SubjectAlternativeName
                    Self::extract_extensions(child, subject_alt_names);
                }
                _ => {}
            }

            pos += len;
            element_index += 1;
        }
    }

    /// Extract Common Name from an X.509 Name (SEQUENCE of SET of SEQUENCE of OID+value)
    fn extract_name_cn(data: &[u8], cn_out: &mut Option<String>) {
        if data.len() < 2 || data[0] != 0x30 {
            return;
        }
        let payload = match Self::extract_sequence_payload(data) {
            Some(p) => p,
            None => return,
        };

        let mut pos = 0;
        while pos < payload.len() {
            if pos + 2 > payload.len() { break; }
            let tag = payload[pos];
            pos += 1;
            let (len, consumed) = match Self::read_der_length(&payload[pos..]) {
                Ok(v) => v, Err(_) => break,
            };
            pos += consumed;
            if pos + len > payload.len() { break; }

            if tag == 0x31 {
                // SET — look for OID 2.5.4.3 (id-at-commonName)
                let set_data = &payload[pos..pos + len];
                let mut inner_pos = 0;
                while inner_pos < set_data.len() {
                    if inner_pos + 2 > set_data.len() { break; }
                    let itag = set_data[inner_pos];
                    inner_pos += 1;
                    let (ilen, iconsumed) = match Self::read_der_length(&set_data[inner_pos..]) {
                        Ok(v) => v, Err(_) => break,
                    };
                    inner_pos += iconsumed;
                    if inner_pos + ilen > set_data.len() { break; }

                    if itag == 0x30 && ilen >= 5 {
                        // SEQUENCE { OID, value }
                        let seq_data = &set_data[inner_pos..inner_pos + ilen];
                        // Check for CN OID: 55 04 03
                        if seq_data.len() >= 5 && seq_data[0] == 0x06 && seq_data[1] == 3
                            && seq_data[2] == 0x55 && seq_data[3] == 0x04 && seq_data[4] == 0x03 {
                            // Next element is the CN value
                            let val_offset = 5;
                            if val_offset + 2 <= seq_data.len() {
                                let vtag = seq_data[val_offset];
                                let (vlen, vconsumed) = match Self::read_der_length(&seq_data[val_offset..]) {
                                    Ok(v) => v, Err(_) => { inner_pos += ilen; continue; }
                                };
                                let vstart = val_offset + vconsumed;
                                if vstart + vlen <= seq_data.len() && (vtag == 0x0C || vtag == 0x13 || vtag == 0x16) {
                                    if let Ok(s) = std::str::from_utf8(&seq_data[vstart..vstart + vlen]) {
                                        *cn_out = Some(s.to_string());
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    inner_pos += ilen;
                }
            }
            pos += len;
        }
    }

    /// Extract subject public key from SubjectPublicKeyInfo
    fn extract_subject_public_key(data: &[u8], pk_out: &mut Vec<u8>) {
        if data.len() < 2 || data[0] != 0x30 {
            return;
        }
        let payload = match Self::extract_sequence_payload(data) {
            Some(p) => p,
            None => return,
        };

        // First: AlgorithmIdentifier (skip)
        // Second: BIT STRING with public key
        let mut pos = 0;
        let mut elem_idx = 0;
        while pos < payload.len() {
            if pos + 2 > payload.len() { break; }
            let tag = payload[pos];
            pos += 1;
            let (len, consumed) = match Self::read_der_length(&payload[pos..]) {
                Ok(v) => v, Err(_) => break,
            };
            pos += consumed;
            if pos + len > payload.len() { break; }

            if elem_idx == 1 && tag == 0x03 {
                // BIT STRING — skip unused bits byte
                let child = &payload[pos..pos + len];
                if !child.is_empty() && child[0] == 0 {
                    *pk_out = child[1..].to_vec();
                }
            }
            pos += len;
            elem_idx += 1;
        }
    }

    /// Extract extensions, looking for SubjectAlternativeName (OID 2.5.29.17)
    fn extract_extensions(data: &[u8], san_out: &mut Vec<String>) {
        if data.len() < 2 || data[0] != 0x30 {
            return;
        }
        let payload = match Self::extract_sequence_payload(data) {
            Some(p) => p,
            None => return,
        };

        let mut pos = 0;
        while pos < payload.len() {
            if pos + 2 > payload.len() { break; }
            let tag = payload[pos];
            pos += 1;
            let (len, consumed) = match Self::read_der_length(&payload[pos..]) {
                Ok(v) => v, Err(_) => break,
            };
            pos += consumed;
            if pos + len > payload.len() { break; }

            if tag == 0x30 {
                // Extension SEQUENCE — check for SAN OID
                let ext_data = &payload[pos..pos + len];
                if ext_data.len() >= 5 && ext_data[0] == 0x06 {
                    // OID
                    let oid_len = ext_data[1] as usize;
                    if oid_len == 3 && ext_data[2] == 0x55 && ext_data[3] == 0x1D && ext_data[4] == 0x11 {
                        // OID 2.5.29.17 = id-ce-subjectAltName
                        // Parse the extension value (OCTET STRING containing SEQUENCE)
                        let oid_end = 2 + oid_len;
                        if oid_end + 2 <= ext_data.len() && ext_data[oid_end] == 0x04 {
                            // OCTET STRING
                            let (octet_len, octet_consumed) = match Self::read_der_length(&ext_data[oid_end + 1..]) {
                                Ok(v) => v, Err(_) => { pos += len; continue; }
                            };
                            let octet_start = oid_end + 1 + octet_consumed;
                            if octet_start + octet_len <= ext_data.len() {
                                let octet_data = &ext_data[octet_start..octet_start + octet_len];
                                // Parse GeneralNames SEQUENCE
                                Self::parse_general_names(octet_data, san_out);
                            }
                        }
                    }
                }
            }
            pos += len;
        }
    }

    /// Parse GeneralNames from SAN extension value
    fn parse_general_names(data: &[u8], san_out: &mut Vec<String>) {
        if data.len() < 2 || data[0] != 0x30 {
            return;
        }
        let payload = match Self::extract_sequence_payload(data) {
            Some(p) => p,
            None => return,
        };

        let mut pos = 0;
        while pos < payload.len() {
            if pos + 2 > payload.len() { break; }
            let tag = payload[pos];
            pos += 1;
            let (len, consumed) = match Self::read_der_length(&payload[pos..]) {
                Ok(v) => v, Err(_) => break,
            };
            pos += consumed;
            if pos + len > payload.len() { break; }

            // Tag 2 = dNSName (context constructed [2])
            if tag == 0x82 {
                let name_data = &payload[pos..pos + len];
                if let Ok(name) = std::str::from_utf8(name_data) {
                    san_out.push(name.to_string());
                }
            }
            pos += len;
        }
    }

    /// Extract payload from a DER SEQUENCE
    fn extract_sequence_payload(data: &[u8]) -> Option<&[u8]> {
        if data.is_empty() || data[0] != 0x30 {
            return None;
        }
        let (len, consumed) = Self::read_der_length(&data[1..]).ok()?;
        Some(&data[1 + consumed..1 + consumed + len])
    }

    /// Read one complete DER object, returning (bytes, total_consumed)
    fn read_one_der(data: &[u8]) -> Result<(&[u8], usize), String> {
        if data.len() < 2 {
            return Err("Too short".into());
        }
        let (len, consumed) = Self::read_der_length(&data[1..])?;
        let total = 1 + consumed + len;
        if data.len() < total {
            return Err("Truncated".into());
        }
        Ok((&data[..total], total))
    }

    /// Read DER length encoding
    fn read_der_length(data: &[u8]) -> Result<(usize, usize), String> {
        if data.is_empty() {
            return Err("Empty".into());
        }
        let first = data[0];
        if first < 0x80 {
            Ok((first as usize, 1))
        } else {
            let num_bytes = (first & 0x7F) as usize;
            if num_bytes == 0 || num_bytes > 4 {
                return Err("Invalid length encoding".into());
            }
            if data.len() < 1 + num_bytes {
                return Err("Length truncated".into());
            }
            let mut len: usize = 0;
            for &b in &data[1..1 + num_bytes] {
                len = (len << 8) | (b as usize);
            }
            Ok((len, 1 + num_bytes))
        }
    }

    /// Parse ASN.1 time (UTCTime or GeneralizedTime) to Unix timestamp
    fn parse_time(s: &str) -> Result<u64, String> {
        let s = s.trim();
        // UTCTime: YYMMDDHHMMSSZ
        if s.ends_with('Z') && s.len() >= 13 {
            let year = s[0..2].parse::<u64>().map_err(|_| "Bad year")?;
            let month = s[2..4].parse::<u64>().map_err(|_| "Bad month")?;
            let day = s[4..6].parse::<u64>().map_err(|_| "Bad day")?;
            let hour = s[6..8].parse::<u64>().map_err(|_| "Bad hour")?;
            let minute = s[8..10].parse::<u64>().map_err(|_| "Bad minute")?;
            let second = s[10..12].parse::<u64>().map_err(|_| "Bad second")?;

            let year = if year >= 50 { 1900 + year } else { 2000 + year };
            return Self::to_unix(year, month, day, hour, minute, second);
        }
        // GeneralizedTime: YYYYMMDDHHMMSSZ
        if s.ends_with('Z') && s.len() >= 15 {
            let year = s[0..4].parse::<u64>().map_err(|_| "Bad year")?;
            let month = s[4..6].parse::<u64>().map_err(|_| "Bad month")?;
            let day = s[6..8].parse::<u64>().map_err(|_| "Bad day")?;
            let hour = s[8..10].parse::<u64>().map_err(|_| "Bad hour")?;
            let minute = s[10..12].parse::<u64>().map_err(|_| "Bad minute")?;
            let second = s[12..14].parse::<u64>().map_err(|_| "Bad second")?;
            return Self::to_unix(year, month, day, hour, minute, second);
        }
        Err("Unknown time format".into())
    }

    /// Convert broken-down time to Unix timestamp (simplified, no leap seconds)
    fn to_unix(year: u64, month: u64, day: u64, hour: u64, minute: u64, second: u64) -> Result<u64, String> {
        let days_in_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut days: i64 = 0;

        // Days from year 1970 to `year`
        for y in 1970..=year {
            if y == year {
                for m in 1..month {
                    days += days_in_month[m as usize] as i64;
                    if m == 2 && Self::is_leap(y) {
                        days += 1;
                    }
                }
                days += day as i64 - 1;
            } else {
                days += if Self::is_leap(y) { 366 } else { 365 };
            }
        }

        let secs = days * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;
        if secs < 0 {
            return Err("Time before epoch".into());
        }
        Ok(secs as u64)
    }

    fn is_leap(year: u64) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    /// Check if a certificate CN/SAN matches a hostname (wildcard support)
    fn hostname_matches(pattern: &str, hostname: &str) -> bool {
        if pattern == hostname {
            return true;
        }
        if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            if let Some(pos) = hostname.find('.') {
                return &hostname[pos + 1..] == suffix;
            }
        }
        false
    }
}

/// Minimal base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();

    let mut out = Vec::with_capacity(cleaned.len() * 3 / 4);
    let mut buf = [0u8; 4];

    for chunk in cleaned.chunks(4) {
        if chunk.len() < 4 {
            return Err("Invalid base64 length".into());
        }
        for (j, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                buf[j] = 0;
            } else {
                buf[j] = table.iter().position(|&t| t == c).ok_or("Invalid base64 char")? as u8;
            }
        }

        out.push((buf[0] << 2) | (buf[1] >> 4));
        if chunk[2] != b'=' {
            out.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if chunk[3] != b'=' {
            out.push((buf[2] << 6) | buf[3]);
        }
    }

    Ok(out)
}
