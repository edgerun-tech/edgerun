//! DKIM signing for outbound emails (RFC 6376).

use crate::std;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::io;

use edgerun_crypto::rsa::sha2::{Digest, Sha256};
use edgerun_crypto::rsa::{
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding},
    signature::SignatureEncoding,
    RsaPrivateKey,
};
use edgerun_crypto::OsRng;
use edgerun_encoding::base64;

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
        let private_key = RsaPrivateKey::new(&mut rng, 2048).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("RSA keygen failed: {}", e),
            )
        })?;

        Ok(Self::new(
            domain.to_string(),
            selector.to_string(),
            private_key,
        ))
    }

    pub fn from_private_key_pem(domain: &str, selector: &str, pem: &str) -> io::Result<Self> {
        let private_key = RsaPrivateKey::from_pkcs8_pem(&pem).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse DKIM key: {}", e),
            )
        })?;

        Ok(Self::new(
            domain.to_string(),
            selector.to_string(),
            private_key,
        ))
    }

    pub fn load_from_store<S: DkimKeyStore>(
        domain: &str,
        selector: &str,
        store: &S,
    ) -> io::Result<Self> {
        let pem = store.read_private_key_pem(domain, selector)?;
        Self::from_private_key_pem(domain, selector, &pem)
    }

    pub fn save_public_key_to_store<S: DkimKeyStore>(&self, store: &S) -> io::Result<()> {
        store.write_public_key_txt(&self.domain, &self.selector, &self.public_key_txt())
    }

    pub fn public_key_txt(&self) -> String {
        let public_key = self.private_key.to_public_key();
        let public_key_der = public_key
            .to_pkcs1_der()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to encode public key: {}", e),
                )
            })
            .unwrap();

        let base64_key = base64::standard_encode(public_key_der.as_bytes());

        format!("v=DKIM1; k=rsa; p={}", base64_key)
    }

    pub fn private_key_pem(&self) -> io::Result<String> {
        self.private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map(|pem| pem.to_string())
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("failed to encode DKIM private key: {}", e),
                )
            })
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
        let body_hash_b64 = base64::standard_encode(&body_hash);

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

        let signed_header_names_str = signed_header_names.join(":");

        let dkim_header = format!(
            "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d={}; s={}; h={}; bh={}; b=",
            self.domain, self.selector, signed_header_names_str, body_hash_b64
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

        let hash = Sha256::digest(&canonical_headers);
        let signing_key =
            edgerun_crypto::rsa::pkcs1v15::SigningKey::<Sha256>::new((*self.private_key).clone());
        let signature = edgerun_crypto::rsa::signature::Signer::sign(&signing_key, &hash);
        let signature_b64 = base64::standard_encode(signature.to_bytes().as_ref());

        let final_header = format!(
            "DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d={}; s={}; h={}; bh={}; b={}",
            self.domain, self.selector, signed_header_names_str, body_hash_b64, signature_b64
        );

        Ok(final_header)
    }
}

pub trait DkimKeyStore {
    fn read_private_key_pem(&self, domain: &str, selector: &str) -> io::Result<String>;
    fn write_public_key_txt(
        &self,
        domain: &str,
        selector: &str,
        txt_record: &str,
    ) -> io::Result<()>;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dkim_h_tag_does_not_include_dkim_signature() {
        let signer = DkimSigner::generate("example.com", "mail").unwrap();
        let header = signer
            .sign(
                b"From: a@example.com\r\nTo: b@example.net\r\nSubject: Test",
                b"Hello\r\n",
            )
            .unwrap();

        assert!(header.starts_with("DKIM-Signature: "));
        assert!(header.contains("h=from:to:subject;"));
        assert!(!header.contains("dkim-signature"));
    }
}
