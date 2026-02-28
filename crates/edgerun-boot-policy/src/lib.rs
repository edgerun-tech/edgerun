// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const RUNTIME_IMAGE_POLICY_SIGNING_CONTEXT: &str = "edgerun-runtime-image-policy-v1";
pub const RUNTIME_IMAGE_REQUEST_SIGNING_CONTEXT: &str = "edgerun-runtime-image-request-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeImageRequestSigned {
    pub owner_pubkey: String,
    pub device_pubkey_b64url: String,
    pub rpc_url: String,
    pub request_nonce_b64url: String,
    pub request_issued_at_unix_s: u64,
    pub request_signature_b64url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeImagePolicyResponseSigned {
    pub ok: bool,
    pub image_ref: String,
    pub request_nonce_b64url: String,
    pub issued_at_unix_s: u64,
    pub valid_until_unix_s: u64,
    pub rollback_index: u64,
    pub signature_b64url: String,
    #[serde(default)]
    pub signing_key_id: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeImagePolicyCanonicalInput<'a> {
    pub owner_pubkey: &'a str,
    pub device_pubkey_b64url: &'a str,
    pub rpc_url: &'a str,
    pub request_nonce_b64url: &'a str,
    pub image_ref: &'a str,
    pub issued_at_unix_s: u64,
    pub valid_until_unix_s: u64,
    pub rollback_index: u64,
}

pub fn runtime_image_request_canonical_message(req: &RuntimeImageRequestSigned) -> String {
    format!(
        "{RUNTIME_IMAGE_REQUEST_SIGNING_CONTEXT}\nowner_pubkey={}\ndevice_pubkey_b64url={}\nrpc_url={}\nrequest_nonce_b64url={}\nrequest_issued_at_unix_s={}",
        req.owner_pubkey,
        req.device_pubkey_b64url,
        req.rpc_url,
        req.request_nonce_b64url,
        req.request_issued_at_unix_s
    )
}

pub fn runtime_image_policy_canonical_message(
    input: &RuntimeImagePolicyCanonicalInput<'_>,
) -> String {
    format!(
        "{RUNTIME_IMAGE_POLICY_SIGNING_CONTEXT}\nowner_pubkey={}\ndevice_pubkey_b64url={}\nrpc_url={}\nrequest_nonce_b64url={}\nimage_ref={}\nissued_at_unix_s={}\nvalid_until_unix_s={}\nrollback_index={}",
        input.owner_pubkey,
        input.device_pubkey_b64url,
        input.rpc_url,
        input.request_nonce_b64url,
        input.image_ref,
        input.issued_at_unix_s,
        input.valid_until_unix_s,
        input.rollback_index
    )
}

pub fn decode_ed25519_verify_key_b64url(raw: &str) -> Result<VerifyingKey> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.trim().as_bytes())
        .context("invalid base64url verify key")?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("verify key must decode to 32 bytes"))?;
    VerifyingKey::from_bytes(&arr).context("invalid ed25519 verify key bytes")
}

pub fn decode_ed25519_signature_b64url(raw: &str) -> Result<Signature> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.trim().as_bytes())
        .context("invalid base64url signature")?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("signature must decode to 64 bytes"))?;
    Ok(Signature::from_bytes(&arr))
}

pub fn verify_runtime_image_policy_signature(
    verify_key: &VerifyingKey,
    canonical: &str,
    signature_b64url: &str,
) -> Result<()> {
    let signature = decode_ed25519_signature_b64url(signature_b64url)?;
    verify_key
        .verify(canonical.as_bytes(), &signature)
        .context("runtime image policy signature verification failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_canonical_is_stable() {
        let req = RuntimeImageRequestSigned {
            owner_pubkey: "owner".to_string(),
            device_pubkey_b64url: "device".to_string(),
            rpc_url: "http://127.0.0.1:8899".to_string(),
            request_nonce_b64url: "nonce".to_string(),
            request_issued_at_unix_s: 42,
            request_signature_b64url: "sig".to_string(),
        };
        let got = runtime_image_request_canonical_message(&req);
        let expected = "edgerun-runtime-image-request-v1\nowner_pubkey=owner\ndevice_pubkey_b64url=device\nrpc_url=http://127.0.0.1:8899\nrequest_nonce_b64url=nonce\nrequest_issued_at_unix_s=42";
        assert_eq!(got, expected);
    }

    #[test]
    fn policy_canonical_is_stable() {
        let got = runtime_image_policy_canonical_message(&RuntimeImagePolicyCanonicalInput {
            owner_pubkey: "owner",
            device_pubkey_b64url: "device",
            rpc_url: "http://127.0.0.1:8899",
            request_nonce_b64url: "nonce",
            image_ref: "ghcr.io/edgerun/worker:main",
            issued_at_unix_s: 1,
            valid_until_unix_s: 2,
            rollback_index: 3,
        });
        let expected = "edgerun-runtime-image-policy-v1\nowner_pubkey=owner\ndevice_pubkey_b64url=device\nrpc_url=http://127.0.0.1:8899\nrequest_nonce_b64url=nonce\nimage_ref=ghcr.io/edgerun/worker:main\nissued_at_unix_s=1\nvalid_until_unix_s=2\nrollback_index=3";
        assert_eq!(got, expected);
    }
}
