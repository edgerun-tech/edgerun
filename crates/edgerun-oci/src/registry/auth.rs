//! Authentication credentials and helpers.

use crate::prelude::*;
use alloc::string::{String, ToString};

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
    let value = edgerun_json::parse_json(&data)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
    if let Some(auths) = value
        .as_object()
        .and_then(|object| object.get_object("auths"))
    {
        for (_, entry) in auths.fields() {
            if let Some(auth) = entry.as_object().and_then(|object| object.get_str("auth")) {
                if let Some((username, password)) = decode_basic_auth(auth) {
                    return Ok(RegistryAuth::Basic { username, password });
                }
            }
        }
    }

    Ok(RegistryAuth::Anonymous)
}

/// Decode a base64-encoded basic auth string ("user:pass").
pub fn decode_basic_auth(auth: &str) -> Option<(String, String)> {
    let compact: String = auth.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let decoded = edgerun_encoding::base64::standard_decode(&compact).ok()?;
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
#[path = "../../tests/unit_src/src/registry/auth_tests.rs"]
mod tests;
