//! DKIM signing for outbound emails (RFC 6376).

use std::io;
use std::path::Path;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::DecodePrivateKey,
    signature::SignatureEncoding,
    RsaPrivateKey,
};
use edgerun_crypto::OsRng;
use sha2::{Digest, Sha256};

pub struct DkimSigner {
    selector: String,
    domain: String,
    private_key: Arc<RsaPrivateKey>,
}

impl DkimSigner {
    pub fn new(domain: String, selector: String, private_key: RsaPrivateKey) -> Self {
        Self {
            selector,
            domain,
            private_key: Arc::new(private_key),
        }
    }

    pub fn generate(domain: &str, selector: &str) -> io::Result<Self> {
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("RSA keygen failed: {}", e)))?;

        Ok(Self::new(
            domain.to_string(),
            selector.to_string(),
            private_key,
        ))
    }

    pub fn load(domain: &str, selector: &str, path: &Path) -> io::Result<Self> {
        let pem = std::fs::read_to_string(path)?;
        let private_key = RsaPrivateKey::from_pkcs8_pem(&pem)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("failed to parse DKIM key: {}", e)))?;

        Ok(Self::new(
            domain.to_string(),
            selector.to_string(),
            private_key,
        ))
    }

    pub fn save_public_key(&self, path: &Path) -> io::Result<()> {
        let txt_record = self.public_key_txt();
        std::fs::write(path, txt_record)?;
        Ok(())
    }

    pub fn public_key_txt(&self) -> String {
        let public_key = self.private_key.to_public_key();
        let public_key_der = public_key.to_pkcs1_der()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("failed to encode public key: {}", e)))
            .unwrap();

        let base64_key = BASE64.encode(public_key_der.as_bytes());

        format!(
            "v=DKIM1; k=rsa; p={}",
            base64_key
        )
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn sign(&self, headers: &[u8], body: &[u8]) -> io::Result<String> {
        let headers_str = String::from_utf8_lossy(headers);

        let canonical_body = canonicalize_body(body)?;
        let body_hash = Sha256::digest(&canonical_body);
        let body_hash_b64 = BASE64.encode(body_hash);

        let mut signed_header_names = Vec::new();
        let unfolded = unfold_headers(&headers_str);
        for line in unfolded.lines() {
            if let Some(name) = line.split_once(':') {
                let name_lower = name.0.to_lowercase();
                if !signed_header_names.contains(&name_lower) {
                    signed_header_names.push(name_lower);
                }
            }
        }

        signed_header_names.push("dkim-signature".to_string());
        let signed_header_names_str = signed_header_names.join(":");

        let dkim_header = format!(
            "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d={}; s={}; h={}; bh={}; b=",
            self.domain,
            self.selector,
            signed_header_names_str,
            body_hash_b64
        );

        let mut headers_with_dkim = headers_str.as_bytes().to_vec();
        if !headers_with_dkim.ends_with(b"\r\n") {
            headers_with_dkim.push(b'\r');
            headers_with_dkim.push(b'\n');
        }
        headers_with_dkim.extend(dkim_header.as_bytes());
        headers_with_dkim.push(b'\r');
        headers_with_dkim.push(b'\n');

        let canonical_headers = canonicalize_headers(headers_with_dkim.as_slice())?;

        let mut sign_data = canonical_headers;
        sign_data.extend(canonical_body);

        let hash = Sha256::digest(&sign_data);
        let signing_key = rsa::pkcs1v15::SigningKey::<Sha256>::new((*self.private_key).clone());
        let signature = rsa::signature::Signer::sign(&signing_key, &hash);
        let signature_b64 = BASE64.encode(signature.to_bytes());

        let final_header = format!(
            "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d={}; s={}; h={}; bh={}; b={}",
            self.domain,
            self.selector,
            signed_header_names_str,
            body_hash_b64,
            signature_b64
        );

        Ok(final_header)
    }
}

impl Clone for DkimSigner {
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            domain: self.domain.clone(),
            private_key: Arc::clone(&self.private_key),
        }
    }
}

fn canonicalize_headers(headers: &[u8]) -> io::Result<Vec<u8>> {
    let headers_str = String::from_utf8_lossy(headers);
    let unfolded = unfold_headers(&headers_str);
    let mut canonical = Vec::new();

    for line in unfolded.lines() {
        if line.is_empty() {
            continue;
        }

        let (name, value) = match line.split_once(':') {
            Some((n, v)) => (n, v),
            None => continue,
        };

        let name_lower = name.to_lowercase();
        let value_trimmed = value.trim();

        canonical.extend(name_lower.as_bytes());
        canonical.push(b':');
        canonical.extend(value_trimmed.as_bytes());
        canonical.push(b'\r');
        canonical.push(b'\n');
    }

    Ok(canonical)
}

fn canonicalize_body(body: &[u8]) -> io::Result<Vec<u8>> {
    let body_str = String::from_utf8_lossy(body);
    let mut canonical = Vec::new();
    let mut prev_was_space = false;

    for byte in body_str.as_bytes() {
        match byte {
            b' ' | b'\t' => {
                if !prev_was_space {
                    canonical.push(b' ');
                    prev_was_space = true;
                }
            }
            b'\r' | b'\n' => {
                canonical.push(b'\r');
                canonical.push(b'\n');
                prev_was_space = false;
            }
            _ => {
                canonical.push(*byte);
                prev_was_space = false;
            }
        }
    }

    Ok(canonical)
}

fn unfold_headers(s: &str) -> String {
    let mut result = String::new();
    let mut prev_line_continuation = false;

    for line in s.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if prev_line_continuation {
                result.push_str(line);
            } else {
                result.push_str(line.trim_start());
            }
            prev_line_continuation = true;
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
            prev_line_continuation = false;
        }
    }

    result
}