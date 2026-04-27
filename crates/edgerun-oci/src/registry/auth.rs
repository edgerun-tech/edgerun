//! Authentication credentials and helpers.

use crate::prelude::*;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[cfg(all(feature = "std", not(target_os = "none")))]
use std::path::PathBuf;

/// Authentication credentials for a registry.
#[derive(Clone, Debug)]
pub enum RegistryAuth {
    /// No authentication (public images).
    Anonymous,
    /// Username and password (basic auth or token auth).
    Basic { username: String, password: String },
    /// Pre-baked bearer token.
    Bearer { token: String },
    /// Resolve credentials from the edgerun secret service.
    ///
    /// The `registry_host` is used as the credential key within the
    /// given namespace. The secret service stores `username:password`
    /// as the secret value (basic auth format).
    #[cfg(all(feature = "std", not(target_os = "none")))]
    FromSecretService {
        data_root: PathBuf,
        namespace: String,
        registry_host: String,
    },
}

/// Read registry config.json for credentials.
/// Supports the standard OCI/Docker-compatible config.json format
/// found at `$HOME/.config/containers/auth.json` or `$HOME/.docker/config.json`.
#[cfg(all(feature = "std", not(target_os = "none")))]
pub fn load_registry_auth(path: &std::path::Path) -> std::io::Result<RegistryAuth> {
    let data = std::fs::read_to_string(path)?;

    // Minimal JSON parsing for auths
    if let Some(auths_start) = data.find("\"auths\"") {
        let rest = &data[auths_start..];
        // Find the first auth entry with "auth"
        if let Some(auth_start) = rest.find("\"auth\"") {
            let auth_rest = &rest[auth_start..];
            if let Some(colon) = auth_rest.find(':') {
                let after_colon = &auth_rest[colon + 1..];
                // Skip whitespace and quote
                let trimmed = after_colon
                    .trim_start_matches(|c: char| !c.is_ascii_alphanumeric() && c != '+');
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
    let decoded = decode_base64_standard(auth)?;
    let s = String::from_utf8(decoded).ok()?;
    let (username, password) = s.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

fn decode_base64_standard(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut quad = [0u8; 4];
    let mut quad_len = 0;

    for byte in input.bytes().filter(|b| !b.is_ascii_whitespace()) {
        quad[quad_len] = byte;
        quad_len += 1;
        if quad_len == 4 {
            decode_base64_quad(&quad, &mut out)?;
            quad_len = 0;
        }
    }

    if quad_len != 0 {
        if quad_len == 1 {
            return None;
        }
        for slot in &mut quad[quad_len..] {
            *slot = b'=';
        }
        decode_base64_quad(&quad, &mut out)?;
    }

    Some(out)
}

fn decode_base64_quad(quad: &[u8; 4], out: &mut Vec<u8>) -> Option<()> {
    let pad = quad.iter().rev().take_while(|&&b| b == b'=').count();
    if pad > 2 || quad[..4 - pad].contains(&b'=') {
        return None;
    }

    let mut value = 0u32;
    for &byte in quad {
        value <<= 6;
        if byte != b'=' {
            value |= decode_base64_char(byte)? as u32;
        }
    }

    out.push((value >> 16) as u8);
    if pad < 2 {
        out.push((value >> 8) as u8);
    }
    if pad == 0 {
        out.push(value as u8);
    }
    Some(())
}

fn decode_base64_char(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
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

    Some((realm?, service.unwrap_or_else(|| "registry".into()), scope))
}

/// Resolve credentials from the secret service for a given registry host.
///
/// The secret is stored as `username:password` (basic auth format) in the
/// secret service backend under `{namespace}/{registry_host}`.
///
/// Returns `None` if no credential is found.
#[cfg(all(feature = "std", not(target_os = "none")))]
pub fn resolve_from_secret_service(
    data_root: &std::path::Path,
    namespace: &str,
    registry_host: &str,
) -> Option<(String, String)> {
    // Build the collection path
    let coll = format!("/org/freedesktop/secrets/collections/{}", namespace);

    // Use the backend to look up the credential
    let backend = edgerun_secret_service::Backend::new_noop(data_root.to_path_buf()).ok()?;
    let (secret_bytes, _meta) = backend.get(&coll, registry_host).ok()??;

    // Secret is stored as "username:password"
    let secret_str = String::from_utf8(secret_bytes).ok()?;
    let (username, password) = secret_str.split_once(':')?;
    Some((username.to_string(), password.to_string()))
}

#[cfg(all(test, feature = "std", not(target_os = "none")))]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("oci_auth_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn resolve_from_secret_service_roundtrip() {
        let root = tmp_root();

        // Store a credential via the secret service backend
        let coll = "/org/freedesktop/secrets/collections/registry";
        let mut backend = edgerun_secret_service::Backend::new_noop(root.clone()).unwrap();
        backend
            .put(coll, "docker.io", b"myuser:mypass123", "Docker Hub", &[])
            .unwrap();

        // Resolve it
        let creds = resolve_from_secret_service(&root, "registry", "docker.io");
        assert_eq!(creds, Some(("myuser".into(), "mypass123".into())));
    }

    #[test]
    fn resolve_from_secret_service_missing_returns_none() {
        let root = tmp_root();
        let creds = resolve_from_secret_service(&root, "registry", "nonexistent");
        assert!(creds.is_none());
    }

    #[test]
    fn resolve_from_secret_service_wrong_format_returns_none() {
        let root = tmp_root();

        let coll = "/org/freedesktop/secrets/collections/registry";
        let mut backend = edgerun_secret_service::Backend::new_noop(root.clone()).unwrap();
        // Store without the colon separator
        backend
            .put(coll, "docker.io", b"no-colon-here", "Bad", &[])
            .unwrap();

        let creds = resolve_from_secret_service(&root, "registry", "docker.io");
        assert!(creds.is_none());
    }

    #[test]
    fn decode_basic_auth_roundtrip() {
        let (user, pass) = decode_basic_auth("bXl1c2VyOm15cGFzcw==").unwrap();
        assert_eq!(user, "myuser");
        assert_eq!(pass, "mypass");
    }

    #[test]
    fn parse_bearer_auth_full() {
        let header = "Bearer realm=\"https://auth.docker.io/token\",service=\"registry.docker.io\",scope=\"repository:library/alpine:pull\"";
        let (realm, service, scope) = parse_bearer_auth(header).unwrap();
        assert_eq!(realm, "https://auth.docker.io/token");
        assert_eq!(service, "registry.docker.io");
        assert_eq!(scope, Some("repository:library/alpine:pull".into()));
    }

    #[test]
    fn parse_bearer_auth_no_scope() {
        let header = "Bearer realm=\"https://auth.example.com/token\",service=\"registry\"";
        let (realm, service, scope) = parse_bearer_auth(header).unwrap();
        assert_eq!(realm, "https://auth.example.com/token");
        assert_eq!(service, "registry");
        assert!(scope.is_none());
    }

    #[test]
    fn parse_bearer_auth_invalid_prefix() {
        assert!(parse_bearer_auth("Basic abc").is_none());
        assert!(parse_bearer_auth("").is_none());
    }
}
