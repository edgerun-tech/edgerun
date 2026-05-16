//! SMTP AUTH / SASL payload helpers.
//!
//! This module parses mechanism payload bytes only. It does not validate
//! identities or secrets; runtime mail backends own that policy.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthCredentials {
    /// Authorization identity: who the client wants to act as.
    pub authz_id: String,
    /// Authentication identity: who the client claims to be.
    pub authc_id: String,
    /// Token / secret.
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthParseError {
    InvalidBase64(String),
    InvalidUtf8,
    InvalidPlain,
}

impl fmt::Display for AuthParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBase64(err) => write!(f, "invalid base64: {err}"),
            Self::InvalidUtf8 => write!(f, "invalid utf-8"),
            Self::InvalidPlain => write!(f, "SASL PLAIN requires authz_id\\0authc_id\\0token"),
        }
    }
}

impl AuthCredentials {
    /// Decode SASL PLAIN mechanism payload.
    ///
    /// Format: `authz_id\0authc_id\0token`.
    pub fn from_plain(payload: &[u8]) -> Result<Self, AuthParseError> {
        let text = core::str::from_utf8(payload).map_err(|_| AuthParseError::InvalidUtf8)?;
        let mut parts = text.splitn(3, '\0');
        let authz_id = parts.next().ok_or(AuthParseError::InvalidPlain)?;
        let authc_id = parts.next().ok_or(AuthParseError::InvalidPlain)?;
        let token = parts.next().ok_or(AuthParseError::InvalidPlain)?;

        Ok(Self {
            authz_id: authz_id.to_string(),
            authc_id: authc_id.to_string(),
            token: token.to_string(),
        })
    }
}

pub fn decode_base64_raw(encoded: &str) -> Result<Vec<u8>, AuthParseError> {
    edgerun_encoding::base64::standard_decode(encoded)
        .map_err(|err| AuthParseError::InvalidBase64(err.to_string()))
}

pub fn decode_plain_response(encoded: &str) -> Result<AuthCredentials, AuthParseError> {
    let decoded = decode_base64_raw(encoded)?;
    AuthCredentials::from_plain(&decoded)
}

pub fn decode_login_field(encoded: &str) -> Result<String, AuthParseError> {
    let decoded = decode_base64_raw(encoded)?;
    String::from_utf8(decoded).map_err(|_| AuthParseError::InvalidUtf8)
}

pub fn credentials_from_login(
    username: &str,
    token: &str,
) -> Result<AuthCredentials, AuthParseError> {
    let mut payload = Vec::with_capacity(username.len() + token.len() + 2);
    payload.push(0);
    payload.extend_from_slice(username.as_bytes());
    payload.push(0);
    payload.extend_from_slice(token.as_bytes());
    AuthCredentials::from_plain(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_encoding::base64::standard_encode;

    #[test]
    fn decodes_plain_credentials() {
        let creds = AuthCredentials::from_plain(b"\0user@example.com\0secret").unwrap();
        assert_eq!(creds.authz_id, "");
        assert_eq!(creds.authc_id, "user@example.com");
        assert_eq!(creds.token, "secret");
    }

    #[test]
    fn rejects_invalid_plain_payload() {
        assert_eq!(
            AuthCredentials::from_plain(b"not-plain").unwrap_err(),
            AuthParseError::InvalidPlain
        );
    }

    #[test]
    fn decodes_base64_plain_response() {
        let payload = standard_encode(b"admin\0user@example.com\0secret");
        let creds = decode_plain_response(&payload).unwrap();
        assert_eq!(creds.authz_id, "admin");
        assert_eq!(creds.authc_id, "user@example.com");
        assert_eq!(creds.token, "secret");
    }

    #[test]
    fn builds_login_credentials() {
        let user = standard_encode(b"user@example.com");
        let token = standard_encode(b"secret");
        let creds = credentials_from_login(
            &decode_login_field(&user).unwrap(),
            &decode_login_field(&token).unwrap(),
        )
        .unwrap();
        assert_eq!(creds.authc_id, "user@example.com");
        assert_eq!(creds.token, "secret");
    }
}
