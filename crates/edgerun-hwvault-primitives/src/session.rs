// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub expires_at: u64,
    pub signing_key: String,
    pub bound_origin: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub ttl_secs: u64,
    pub max_skew_secs: u64,
    pub nonce_ttl_secs: u64,
    pub max_nonce_len: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            ttl_secs: 900,
            max_skew_secs: 60,
            nonce_ttl_secs: 300,
            max_nonce_len: 128,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIssue {
    pub token: String,
    pub signing_key: String,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone)]
pub struct SessionAuthInput<'a> {
    pub auth_header: Option<&'a str>,
    pub origin_header: Option<&'a str>,
    pub ts_header: Option<&'a str>,
    pub nonce_header: Option<&'a str>,
    pub sig_header: Option<&'a str>,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SessionAuthError {
    #[error("missing bearer token")]
    MissingBearer,
    #[error("unknown session")]
    UnknownSession,
    #[error("session expired")]
    SessionExpired,
    #[error("origin mismatch")]
    OriginMismatch,
    #[error("missing timestamp")]
    MissingTimestamp,
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("timestamp skew too large")]
    TimestampSkew,
    #[error("missing nonce")]
    MissingNonce,
    #[error("invalid nonce")]
    InvalidNonce,
    #[error("missing signature")]
    MissingSignature,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("replay detected")]
    ReplayDetected,
}

pub fn create_session(
    sessions: &mut HashMap<String, SessionState>,
    now_unix_s: u64,
    cfg: &SessionConfig,
    bound_origin: Option<String>,
) -> SessionIssue {
    let token = random_token(32);
    let signing_key = random_token(32);
    let bound = bound_origin.filter(|v| !v.trim().is_empty());

    sessions.insert(
        token.clone(),
        SessionState {
            expires_at: now_unix_s + cfg.ttl_secs.max(1),
            signing_key: signing_key.clone(),
            bound_origin: bound,
        },
    );

    SessionIssue {
        token,
        signing_key,
        ttl_secs: cfg.ttl_secs.max(1),
    }
}

pub fn verify_session_request(
    sessions: &mut HashMap<String, SessionState>,
    used_nonces: &mut HashMap<String, u64>,
    input: SessionAuthInput<'_>,
    now_unix_s: u64,
    cfg: &SessionConfig,
) -> Result<String, SessionAuthError> {
    let auth = input.auth_header.unwrap_or_default().trim();
    if !auth.starts_with("Bearer ") {
        return Err(SessionAuthError::MissingBearer);
    }
    let token = auth.trim_start_matches("Bearer ").trim().to_string();

    let (signing_key, bound_origin) = {
        let Some(s) = sessions.get_mut(&token) else {
            return Err(SessionAuthError::UnknownSession);
        };
        if s.expires_at <= now_unix_s {
            sessions.remove(&token);
            return Err(SessionAuthError::SessionExpired);
        }
        s.expires_at = now_unix_s + cfg.ttl_secs.max(1);
        (s.signing_key.clone(), s.bound_origin.clone())
    };

    if let Some(bound) = bound_origin {
        let origin = input.origin_header.unwrap_or_default();
        if origin != bound {
            return Err(SessionAuthError::OriginMismatch);
        }
    }

    let ts_raw = input.ts_header.ok_or(SessionAuthError::MissingTimestamp)?;
    let ts = ts_raw
        .trim()
        .parse::<u64>()
        .map_err(|_| SessionAuthError::InvalidTimestamp)?;
    if now_unix_s.abs_diff(ts) > cfg.max_skew_secs {
        return Err(SessionAuthError::TimestampSkew);
    }

    let nonce = input
        .nonce_header
        .ok_or(SessionAuthError::MissingNonce)?
        .trim();
    if nonce.is_empty() || nonce.len() > cfg.max_nonce_len {
        return Err(SessionAuthError::InvalidNonce);
    }

    let body_hash = body_hash_b64(input.body);
    let canonical = canonical_message(input.method, input.path, ts, nonce, &body_hash);
    let expected_sig = sign_canonical_b64(&signing_key, &canonical);
    let sig = input
        .sig_header
        .ok_or(SessionAuthError::MissingSignature)?
        .trim();
    if sig != expected_sig {
        return Err(SessionAuthError::InvalidSignature);
    }

    let replay_key = format!("{token}:{nonce}");
    if let Some(exp) = used_nonces.get(&replay_key) {
        if *exp > now_unix_s {
            return Err(SessionAuthError::ReplayDetected);
        }
    }
    used_nonces.insert(replay_key, now_unix_s + cfg.nonce_ttl_secs.max(1));

    Ok(token)
}

pub fn cleanup_expired(
    sessions: &mut HashMap<String, SessionState>,
    used_nonces: &mut HashMap<String, u64>,
    now_unix_s: u64,
) {
    sessions.retain(|_, s| s.expires_at > now_unix_s);
    used_nonces.retain(|_, exp| *exp > now_unix_s);
}

fn random_token(num_bytes: usize) -> String {
    let mut bytes = vec![0_u8; num_bytes];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn body_hash_b64(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

fn canonical_message(
    method: &str,
    path: &str,
    ts: u64,
    nonce: &str,
    body_hash_b64: &str,
) -> String {
    format!("{method}|{path}|{ts}|{nonce}|{body_hash_b64}")
}

fn sign_canonical_b64(key: &str, canonical: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("hmac key");
    mac.update(canonical.as_bytes());
    let sig = mac.finalize().into_bytes();
    URL_SAFE_NO_PAD.encode(sig)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_sign_and_verify_roundtrip() {
        let mut sessions = HashMap::new();
        let mut nonces = HashMap::new();
        let cfg = SessionConfig::default();
        let now = 1_700_000_000_u64;
        let issue = create_session(
            &mut sessions,
            now,
            &cfg,
            Some("https://app.example".to_string()),
        );

        let method = "GET";
        let path = "/v1/policy/info";
        let body = b"";
        let ts = now;
        let nonce = "n-1";
        let body_hash = body_hash_b64(body);
        let canonical = canonical_message(method, path, ts, nonce, &body_hash);
        let sig = sign_canonical_b64(&issue.signing_key, &canonical);

        let token = verify_session_request(
            &mut sessions,
            &mut nonces,
            SessionAuthInput {
                auth_header: Some(&format!("Bearer {}", issue.token)),
                origin_header: Some("https://app.example"),
                ts_header: Some(&ts.to_string()),
                nonce_header: Some(nonce),
                sig_header: Some(&sig),
                method,
                path,
                body,
            },
            now,
            &cfg,
        )
        .expect("must verify");
        assert_eq!(token, issue.token);
    }
}
