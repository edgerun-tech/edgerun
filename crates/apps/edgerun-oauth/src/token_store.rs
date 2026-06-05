//! OAuth token storage using edgerun-secret-service (encrypted blob store).

use crate::prelude::*;
use crate::types::Credentials;
use edgerun_secret_service::Backend;
#[cfg(not(target_os = "none"))]
use std::eprintln;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const COLLECTION: &str = "/org/freedesktop/secrets/collections/default";
const NAMESPACE: &str = "edgerun-oauth";

/// Default path for the secret service data root.
pub fn default_token_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".edgerun").join("secrets")
}

/// Secret-service-backed token store.
pub struct TokenStore {
    backend: Mutex<Backend>,
}

impl TokenStore {
    /// Create a new TokenStore with the default data root.
    pub fn new() -> std::io::Result<Self> {
        let path = default_token_path();
        Self::with_data_root(path)
    }

    /// Create with an explicit data root for the secret service.
    pub fn with_data_root(data_root: PathBuf) -> std::io::Result<Self> {
        let backend = Backend::new(data_root, edgerun_secret_service::no_op_event_recorder())?;
        Ok(Self {
            backend: Mutex::new(backend),
        })
    }

    /// Save credentials to the secret store.
    pub fn save(&self, creds: &Credentials) -> std::io::Result<()> {
        let json = credentials_to_json(creds);
        let attrs = vec![("type".into(), "oauth-token".into())];
        let mut be = self.backend.lock().unwrap();
        be.put(
            COLLECTION,
            "default",
            json.as_bytes(),
            "OAuth Token",
            &attrs,
        )
    }

    /// Load credentials from the secret store.
    pub fn load(&self) -> std::io::Result<Option<Credentials>> {
        let be = self.backend.lock().unwrap();
        match be.get(COLLECTION, "default")? {
            Some((bytes, _meta)) => {
                let text = String::from_utf8_lossy(&bytes);
                match credentials_from_json(&text) {
                    Ok(creds) => Ok(Some(creds)),
                    Err(e) => {
                        edgerun_log::warn!("failed to parse credentials from secret store: {e}");
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    }

    /// Get valid credentials, loading from the secret store.
    /// Returns None if no valid tokens exist.
    pub fn get_valid(&self, grace_secs: u64) -> Option<Credentials> {
        let creds = self.load().ok()??;
        if creds.is_expired_at(crate::runtime_now_secs(), grace_secs) {
            return None;
        }
        Some(creds)
    }

    /// Clear stored credentials.
    pub fn clear(&self) -> std::io::Result<()> {
        let mut be = self.backend.lock().unwrap();
        be.delete(COLLECTION, "default").map(|_| ())
    }
}

fn credentials_to_json(creds: &Credentials) -> String {
    let mut out = String::from("{");
    let mut first = true;
    if let Some(ref v) = creds.access_token {
        crate::json_fixed::write_string_field(&mut out, &mut first, "access_token", v);
    }
    if let Some(ref v) = creds.refresh_token {
        crate::json_fixed::write_string_field(&mut out, &mut first, "refresh_token", v);
    }
    if let Some(ref v) = creds.id_token {
        crate::json_fixed::write_string_field(&mut out, &mut first, "id_token", v);
    }
    if let Some(ref v) = creds.token_type {
        crate::json_fixed::write_string_field(&mut out, &mut first, "token_type", v);
    }
    if let Some(v) = creds.expiry_date {
        crate::json_fixed::write_u64_field(&mut out, &mut first, "expiry_date", v);
    }
    if let Some(ref v) = creds.scope {
        crate::json_fixed::write_string_field(&mut out, &mut first, "scope", v);
    }
    out.push('}');
    out
}

fn credentials_from_json(s: &str) -> Result<Credentials, String> {
    let value = crate::json_fixed::parse_object(s)?;

    Ok(Credentials {
        access_token: value.str_field("access_token"),
        refresh_token: value.str_field("refresh_token"),
        id_token: value.str_field("id_token"),
        token_type: value.str_field("token_type"),
        expiry_date: value.u64_field("expiry_date"),
        scope: value.str_field("scope"),
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_creds() -> Credentials {
        let now = 1_000;
        Credentials {
            access_token: Some("tok".into()),
            refresh_token: Some("ref".into()),
            id_token: Some("jwt".into()),
            token_type: Some("Bearer".into()),
            expiry_date: Some(now + 3600),
            scope: Some("openid profile".into()),
        }
    }

    #[test]
    fn test_credentials_json_roundtrip() {
        let creds = test_creds();
        let json = credentials_to_json(&creds);
        let parsed = credentials_from_json(&json).unwrap();
        assert_eq!(parsed.access_token, creds.access_token);
        assert_eq!(parsed.refresh_token, creds.refresh_token);
        assert_eq!(parsed.id_token, creds.id_token);
        assert_eq!(parsed.token_type, creds.token_type);
        assert_eq!(parsed.scope, creds.scope);
    }

    #[test]
    fn test_credentials_minimal_json() {
        let creds = Credentials {
            access_token: Some("tok".into()),
            refresh_token: None,
            id_token: None,
            token_type: None,
            expiry_date: None,
            scope: None,
        };
        let json = credentials_to_json(&creds);
        let parsed = credentials_from_json(&json).unwrap();
        assert_eq!(parsed.access_token, Some("tok".into()));
        assert!(parsed.refresh_token.is_none());
    }
}
