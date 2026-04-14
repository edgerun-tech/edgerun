//! DKIM signature parsing and verification (RFC 6376).

use std::collections::HashMap;
use std::io;

use crate::DnsQuery;
use sha2::{Digest, Sha256};

/// Result of a DKIM verification.
#[derive(Debug, Clone)]
pub struct DkimResult {
    pub status: DkimStatus,
    pub domain: Option<String>,
    pub selector: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DkimStatus {
    Pass,
    Fail,
    PermError,
    TempError,
    None,
}

impl DkimResult {
    pub fn pass(sig: &DkimSignature) -> Self {
        Self {
            status: DkimStatus::Pass,
            domain: sig.domain.clone(),
            selector: sig.selector.clone(),
            error: None,
        }
    }

    pub fn fail(sig: DkimSignature, error: &str) -> Self {
        Self {
            status: DkimStatus::Fail,
            domain: sig.domain.clone(),
            selector: sig.selector.clone(),
            error: Some(error.to_string()),
        }
    }

    pub fn perm_error(msg: &str) -> Self {
        Self {
            status: DkimStatus::PermError,
            domain: None,
            selector: None,
            error: Some(msg.to_string()),
        }
    }

    pub fn as_auth_result(&self) -> &'static str {
        match self.status {
            DkimStatus::Pass => "pass",
            DkimStatus::Fail => "fail",
            DkimStatus::PermError => "permerror",
            DkimStatus::TempError => "temperror",
            DkimStatus::None => "none",
        }
    }
}

/// Parsed DKIM-Signature header fields.
#[derive(Debug, Clone)]
pub struct DkimSignature {
    pub version: Option<String>,
    pub algorithm: Option<String>,
    pub canonicalization: Option<String>,
    pub domain: Option<String>,
    pub selector: Option<String>,
    pub header_list: Vec<String>,
    pub signature: Vec<u8>,
    pub body_hash: Vec<u8>,
    pub length: Option<usize>,
}

/// Parse all DKIM-Signature headers from raw headers.
pub fn parse_dkim_signatures(headers: &[u8]) -> io::Result<Vec<DkimSignature>> {
    let headers_str = String::from_utf8_lossy(headers);
    let mut signatures = Vec::new();

    // Find all DKIM-Signature headers (unfolded)
    let unfolded = unfold_headers(&headers_str);
    let sig_headers = unfolded
        .lines()
        .filter(|line| line.starts_with("DKIM-Signature:"))
        .map(|line| &line["DKIM-Signature:".len()..])
        .collect::<Vec<_>>();

    for sig_header in sig_headers {
        match parse_dkim_tag_list(sig_header) {
            Ok(sig) => signatures.push(sig),
            Err(e) => {
                edgerun_log::warn!("edgerun-email-auth: DKIM parse error: {}", e);
            }
        }
    }

    Ok(signatures)
}

/// Unfold continuation lines in headers.
fn unfold_headers(headers: &str) -> String {
    headers.replace("\r\n ", " ").replace("\r\n\t", " ")
}

/// Parse a DKIM tag=value list.
fn parse_dkim_tag_list(s: &str) -> io::Result<DkimSignature> {
    let tags_vec = edgerun_encoding::kv::parse_tag_list_semicolon(s);
    let tags: HashMap<String, String> = tags_vec.into_iter().collect();

    let version = tags.get("v").cloned();
    let algorithm = tags.get("a").cloned();
    let canonicalization = tags.get("c").cloned();
    let domain = tags.get("d").cloned();
    let selector = tags.get("s").cloned();

    let header_list = tags
        .get("h")
        .map(|h| h.split(':').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let signature = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, tags.get("b").map(|s| s.as_str()).unwrap_or(""))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid DKIM signature: {}", e)))?;

    let body_hash = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, tags.get("bh").map(|s| s.as_str()).unwrap_or(""))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid DKIM body hash: {}", e)))?;

    let length = tags.get("l").and_then(|l| l.parse::<usize>().ok());

    Ok(DkimSignature {
        version,
        algorithm,
        canonicalization,
        domain,
        selector,
        header_list,
        signature,
        body_hash,
        length,
    })
}

/// Verify a DKIM signature.
pub async fn verify_signature<D: DnsQuery>(
    dns: &mut D,
    sig: &DkimSignature,
    headers: &[u8],
    body: &[u8],
) -> io::Result<()> {
    // 1. Verify body hash
    let (_header_canon, _body_canon) = parse_canonicalization(sig.canonicalization.as_deref());
    let body_canon_result = match canonicalize_body(body, sig.canonicalization.as_deref()) {
        Ok(b) => b,
        Err(e) => return Err(e),
    };
    let computed_body_hash = Sha256::digest(&body_canon_result);
    if computed_body_hash.as_slice() != sig.body_hash {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "body hash mismatch",
        ));
    }

    // 2. Fetch public key from DNS
    let domain = sig
        .domain
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing domain"))?;
    let selector = sig
        .selector
        .as_deref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing selector"))?;

    let query_name = format!("{}._domainkey.{}", selector, domain);
    let txt_records = dns.query_txt(&query_name).await?;

    let key_record = txt_records
        .iter()
        .find(|r| r.contains("v=DKIM1") || r.contains("v=dkim1"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no DKIM key found"))?;

    let public_key = parse_dkim_key_record(key_record)?;

    // 3. Verify signature over canonicalized headers
    let headers_canon = canonicalize_headers(headers, sig, &sig.header_list)?;
    let hash = Sha256::digest(&headers_canon);

    // For now, support RSA only (Ed25519 would need ed25519-dalek)
    let algorithm = sig.algorithm.as_deref().unwrap_or("rsa-sha256");
    if algorithm != "rsa-sha256" && algorithm != "rsa-sha1" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported algorithm: {}", algorithm),
        ));
    }

    // Use RSA verification from edgerun-crypto
    use rsa::RsaPublicKey;
    use rsa::pkcs8::DecodePublicKey;
    let rsa_key = RsaPublicKey::from_public_key_der(&public_key)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid RSA key: {}", e)))?;

    let verifying_key = rsa::Pkcs1v15Sign::new::<sha2::Sha256>();
    rsa_key
        .verify(verifying_key, &hash, &sig.signature)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("signature verification failed: {}", e)))?;

    Ok(())
}

/// Canonicalize body per RFC 6376.
fn canonicalize_body(body: &[u8], canon: Option<&str>) -> io::Result<Vec<u8>> {
    let (_header_canon, body_canon) = parse_canonicalization(canon);

    match body_canon {
        "simple" => {
            // Body must end with CRLF
            let mut result = body.to_vec();
            if !result.ends_with(b"\r\n") {
                result.extend_from_slice(b"\r\n");
            }
            Ok(result)
        }
        "relaxed" | _ => {
            // Default is relaxed for body
            // Remove trailing whitespace, empty lines at end
            let text = String::from_utf8_lossy(body);
            let mut lines: Vec<&str> = text.lines().collect();
            // Remove trailing empty lines
            while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                lines.pop();
            }
            let result = lines.join("\r\n");
            if result.is_empty() {
                Ok(b"\r\n".to_vec())
            } else {
                Ok(result.into_bytes())
            }
        }
    }
}

/// Canonicalize headers per RFC 6376.
fn canonicalize_headers(headers: &[u8], sig: &DkimSignature, header_list: &[String]) -> io::Result<Vec<u8>> {
    let (header_canon, _) = parse_canonicalization(sig.canonicalization.as_deref());

    let headers_str = String::from_utf8_lossy(headers);
    let header_map = parse_header_fields(&headers_str);

    let mut selected_headers = Vec::new();
    for h in header_list {
        let h_lower = h.to_lowercase();
        if let Some(value) = header_map.get(&h_lower) {
            selected_headers.push(format!("{}:{}", h, value));
        }
    }

    // Add the DKIM-Signature header itself (without b= value)
    selected_headers.push(format_dkim_signature_header_for_canon(sig));

    let result = selected_headers.join("\r\n");

    match header_canon {
        "simple" => Ok(result.into_bytes()),
        "relaxed" | _ => {
            // Relaxed: lowercase header names, strip leading/trailing whitespace
            let relaxed = result
                .split("\r\n")
                .map(|line| {
                    if let Some(colon) = line.find(':') {
                        let name = line[..colon].trim().to_lowercase();
                        let value = line[colon + 1..].trim();
                        format!("{}: {}", name, value.split_whitespace().collect::<Vec<_>>().join(" "))
                    } else {
                        line.to_lowercase()
                    }
                })
                .collect::<Vec<_>>()
                .join("\r\n");
            Ok(format!("{}\r\n", relaxed).into_bytes())
        }
    }
}

fn parse_canonicalization(canon: Option<&str>) -> (&str, &str) {
    if let Some(c) = canon {
        if let Some(pos) = c.find('/') {
            (&c[..pos], &c[pos + 1..])
        } else {
            (c, "simple")
        }
    } else {
        ("relaxed", "simple")
    }
}

fn parse_header_fields(headers: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_name = String::new();
    let mut current_value = String::new();

    for line in headers.lines() {
        if line.starts_with(char::is_whitespace) {
            // Continuation line
            current_value.push(' ');
            current_value.push_str(line.trim());
        } else if !current_name.is_empty() {
            map.insert(current_name.to_lowercase(), current_value.clone());
            if let Some(colon) = line.find(':') {
                current_name = line[..colon].to_string();
                current_value = line[colon + 1..].trim().to_string();
            }
        } else if let Some(colon) = line.find(':') {
            current_name = line[..colon].to_string();
            current_value = line[colon + 1..].trim().to_string();
        }
    }

    if !current_name.is_empty() {
        map.insert(current_name.to_lowercase(), current_value);
    }

    map
}

fn format_dkim_signature_header_for_canon(sig: &DkimSignature) -> String {
    // Reconstruct the DKIM-Signature header without the b= value
    let mut parts = Vec::new();
    if let Some(ref v) = sig.version {
        parts.push(format!("v={}", v));
    }
    if let Some(ref a) = sig.algorithm {
        parts.push(format!("a={}", a));
    }
    if let Some(ref c) = sig.canonicalization {
        parts.push(format!("c={}", c));
    }
    if let Some(ref d) = sig.domain {
        parts.push(format!("d={}", d));
    }
    if let Some(ref s) = sig.selector {
        parts.push(format!("s={}", s));
    }
    if !sig.header_list.is_empty() {
        parts.push(format!("h={}", sig.header_list.join(":")));
    }
    parts.push(format!("bh={}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &sig.body_hash)));
    parts.push("b=".to_string());

    format!("DKIM-Signature: {}", parts.join("; "))
}

fn parse_dkim_key_record(record: &str) -> io::Result<Vec<u8>> {
    let mut tags = HashMap::new();
    for part in record.split(';') {
        let part = part.trim();
        if let Some(eq_pos) = part.find('=') {
            let tag = part[..eq_pos].trim();
            let value = part[eq_pos + 1..].trim();
            tags.insert(tag.to_string(), value.to_string());
        }
    }

    let pub_key_b64 = tags
        .get("p")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing p= tag"))?;

    if pub_key_b64.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DKIM key revoked (p= is empty)",
        ));
    }

    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, pub_key_b64)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid DKIM key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_canonicalization_default() {
        let (h, b) = parse_canonicalization(None);
        assert_eq!(h, "relaxed");
        assert_eq!(b, "simple");
    }

    #[test]
    fn test_parse_canonicalization_relaxed_relaxed() {
        let (h, b) = parse_canonicalization(Some("relaxed/relaxed"));
        assert_eq!(h, "relaxed");
        assert_eq!(b, "relaxed");
    }

    #[test]
    fn test_canonicalize_body_simple() {
        let body = b"Hello\r\n";
        let result = canonicalize_body(body, Some("simple/simple")).unwrap();
        assert_eq!(result, b"Hello\r\n");
    }

    #[test]
    fn test_canonicalize_body_simple_add_crlf() {
        let body = b"Hello";
        let result = canonicalize_body(body, Some("simple/simple")).unwrap();
        assert_eq!(result, b"Hello\r\n");
    }
}
