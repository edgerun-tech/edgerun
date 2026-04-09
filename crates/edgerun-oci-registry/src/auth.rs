//! Authentication credentials and helpers.

use std::collections::HashMap;
use std::fs;
use std::io;

use crate::base64;

/// Authentication credentials for a registry.
#[derive(Clone, Debug)]
pub enum RegistryAuth {
    /// No authentication (public images).
    Anonymous,
    /// Username and password (basic auth or token auth).
    Basic { username: String, password: String },
    /// Pre-baked bearer token.
    Bearer { token: String },
}

/// Read registry config.json for credentials.
/// Supports the standard OCI/Docker-compatible config.json format
/// found at `$HOME/.config/containers/auth.json` or `$HOME/.docker/config.json`.
pub fn load_registry_auth(path: &std::path::Path) -> io::Result<RegistryAuth> {
    let data = fs::read_to_string(path)?;

    // Minimal JSON parsing for auths
    if let Some(auths_start) = data.find("\"auths\"") {
        let rest = &data[auths_start..];
        // Find the first auth entry with "auth"
        if let Some(auth_start) = rest.find("\"auth\"") {
            let auth_rest = &rest[auth_start..];
            if let Some(colon) = auth_rest.find(':') {
                let after_colon = &auth_rest[colon + 1..];
                // Skip whitespace and quote
                let trimmed = after_colon.trim_start_matches(|c: char| !c.is_ascii_alphanumeric() && c != '+');
                // Find the closing quote
                if let Some(end_quote) = trimmed.find('"') {
                    let auth_str = &trimmed[..end_quote];
                    if let Some((username, password)) = decode_basic_auth(auth_str) {
                        return Ok(RegistryAuth::Basic { username, password });
                    }
                }
            }
        }
    }

    Ok(RegistryAuth::Anonymous)
}

/// Decode a base64-encoded basic auth string ("user:pass").
pub fn decode_basic_auth(auth: &str) -> Option<(String, String)> {
    let decoded = base64::decode(auth).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let (username, password) = s.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

/// Parse a WWW-Authenticate Bearer challenge header.
///
/// Returns (realm, service, scope).
pub fn parse_bearer_auth(header: &str) -> Option<(String, String, Option<String>)> {
    // Parse: Bearer realm="https://auth.docker.io/token",service="registry.docker.io",scope="repository:library/alpine:pull"
    if !header.starts_with("Bearer ") && !header.starts_with("bearer ") {
        return None;
    }

    let params = &header[7..];
    let mut realm = None;
    let mut service = None;
    let mut scope = None;

    for part in params.split(',') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=') {
            let val = val.trim_matches('"');
            match key {
                "realm" => realm = Some(val.to_string()),
                "service" => service = Some(val.to_string()),
                "scope" => scope = Some(val.to_string()),
                _ => {}
            }
        }
    }

    Some((
        realm?,
        service.unwrap_or_else(|| "registry".into()),
        scope,
    ))
}
