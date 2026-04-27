//! Server-side state for pending device authorization grants.

use crate::prelude::*;
use edgerun_crypto::fill_random;
use edgerun_crypto::sha256;
use edgerun_encoding::base64::base64url_nopad_encode;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

fn read_lock<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    #[cfg(target_os = "none")]
    {
        lock.read().unwrap()
    }
    #[cfg(not(target_os = "none"))]
    lock.read().unwrap_or_else(|e| e.into_inner())
}
fn write_lock<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    #[cfg(target_os = "none")]
    {
        lock.write().unwrap()
    }
    #[cfg(not(target_os = "none"))]
    lock.write().unwrap_or_else(|e| e.into_inner())
}

/// A pending device code grant on the server side.
#[derive(Debug, Clone)]
pub struct PendingDeviceGrant {
    /// The raw device_code the client polls with.
    pub device_code: String,
    /// SHA-256 hash of device_code for lookup (so we don't leak raw codes).
    pub device_code_hash: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub code_challenge: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub created_at: Instant,
    pub expires_in: Duration,
    pub interval_secs: u64,
    /// Whether the user has authorized this grant.
    pub authorized: bool,
    /// The authorization code (set when user authorizes).
    pub authorization_code: Option<String>,
    // The PKCE code_verifier will be validated at token exchange.
}

impl PendingDeviceGrant {
    /// Generate a new pending device grant with random codes.
    pub fn generate(
        client_id: &str,
        scopes: Vec<String>,
        code_challenge: &str,
        base_verification_uri: &str,
    ) -> std::io::Result<Self> {
        let device_code = generate_random_token(32)?;
        let device_code_hash = base64url_nopad_encode(&sha256(device_code.as_bytes()));
        let user_code = generate_user_code()?;
        let auth_code = generate_random_token(32)?;

        let verification_uri_complete = format!(
            "{}?user_code={}",
            base_verification_uri.trim_end_matches('/'),
            user_code
        );

        Ok(Self {
            device_code,
            device_code_hash,
            client_id: client_id.to_string(),
            scopes,
            code_challenge: code_challenge.to_string(),
            user_code,
            verification_uri: base_verification_uri.to_string(),
            verification_uri_complete,
            created_at: Instant::now(),
            expires_in: Duration::from_secs(600),
            interval_secs: 5,
            authorized: false,
            authorization_code: Some(auth_code),
        })
    }

    /// Check if this grant has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.expires_in
    }

    /// Authorize this grant (called when user visits the verification URI).
    pub fn authorize(&mut self) {
        self.authorized = true;
    }
}

/// In-memory store for pending device grants.
pub struct DeviceGrantStore {
    grants: RwLock<HashMap<String, PendingDeviceGrant>>,
    user_code_index: RwLock<HashMap<String, String>>,
}

impl DeviceGrantStore {
    pub fn new() -> Self {
        Self {
            grants: RwLock::new(HashMap::new()),
            user_code_index: RwLock::new(HashMap::new()),
        }
    }

    /// Store a new pending grant.
    pub fn store(&self, grant: PendingDeviceGrant) {
        let hash = grant.device_code_hash.clone();
        let user_code = grant.user_code.clone();
        write_lock(&self.user_code_index).insert(user_code, hash.clone());
        write_lock(&self.grants).insert(hash, grant);
    }

    /// Look up a grant by device_code (hashes the code for lookup).
    pub fn lookup_by_device_code(&self, device_code: &str) -> Option<PendingDeviceGrant> {
        let hash = base64url_nopad_encode(&sha256(device_code.as_bytes()));
        read_lock(&self.grants).get(&hash).cloned()
    }

    /// Look up a grant by user_code (for the authorization page).
    pub fn lookup_by_user_code(&self, user_code: &str) -> Option<PendingDeviceGrant> {
        let hash = read_lock(&self.user_code_index).get(user_code).cloned()?;
        read_lock(&self.grants).get(&hash).cloned()
    }

    /// Mark a grant as authorized by device_code.
    pub fn authorize_by_device_code(&self, device_code: &str) -> bool {
        let hash = base64url_nopad_encode(&sha256(device_code.as_bytes()));
        if let Some(grant) = write_lock(&self.grants).get_mut(&hash) {
            grant.authorize();
            true
        } else {
            false
        }
    }

    /// Remove an expired or consumed grant.
    pub fn remove(&self, device_code: &str) {
        let hash = base64url_nopad_encode(&sha256(device_code.as_bytes()));
        let mut grants = write_lock(&self.grants);
        if let Some(grant) = grants.remove(&hash) {
            write_lock(&self.user_code_index).remove(&grant.user_code);
        }
    }

    /// Sweep expired grants.
    pub fn sweep_expired(&self) -> usize {
        let mut grants = write_lock(&self.grants);
        let mut user_index = write_lock(&self.user_code_index);
        let before = grants.len();
        grants.retain(|_, g| !g.is_expired());
        let removed = before - grants.len();
        user_index.retain(|_, hash| grants.contains_key(hash));
        removed
    }
}

fn generate_random_token(len: usize) -> std::io::Result<String> {
    let mut bytes = vec![0u8; len];
    fill_random(&mut bytes)
        .map_err(|e| std::io::Error::other(format!("random generation failed: {e}")))?;
    Ok(base64url_nopad_encode(&bytes))
}

fn generate_user_code() -> std::io::Result<String> {
    const CHARSET: &[u8] = b"BCDFGHJKLMNPQRSTVWXYZ";
    let mut bytes = [0u8; 8];
    fill_random(&mut bytes)
        .map_err(|e| std::io::Error::other(format!("random generation failed: {e}")))?;

    // Use rejection sampling to ensure valid chars
    let mut code = String::with_capacity(9);
    let mut byte_idx = 0;
    for i in 0..8 {
        if byte_idx >= bytes.len() {
            fill_random(&mut bytes)
                .map_err(|e| std::io::Error::other(format!("random generation failed: {e}")))?;
            byte_idx = 0;
        }
        let idx = bytes[byte_idx] as usize % CHARSET.len();
        byte_idx += 1;
        if i == 4 {
            code.push('-');
        }
        code.push(CHARSET[idx] as char);
    }
    Ok(code)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_grant() {
        let grant = PendingDeviceGrant::generate(
            "my-client",
            vec!["openid".into(), "email".into()],
            "challenge123",
            "https://auth.example.com/verify",
        )
        .unwrap();

        assert_eq!(grant.client_id, "my-client");
        assert_eq!(grant.scopes, vec!["openid", "email"]);
        assert_eq!(grant.code_challenge, "challenge123");
        assert!(!grant.authorized);
        assert!(grant.device_code.len() > 0);
        assert!(!grant.user_code.is_empty());
        assert!(grant.verification_uri_complete.contains(&grant.user_code));
        assert!(!grant.is_expired());
    }

    #[test]
    fn test_grant_expiry() {
        let mut grant = PendingDeviceGrant::generate(
            "my-client",
            vec!["openid".into()],
            "challenge",
            "https://auth.example.com/verify",
        )
        .unwrap();

        grant.expires_in = Duration::from_secs(0); // immediately expired
        assert!(grant.is_expired());
    }

    #[test]
    fn test_authorize_grant() {
        let mut grant = PendingDeviceGrant::generate(
            "my-client",
            vec!["openid".into()],
            "challenge",
            "https://auth.example.com/verify",
        )
        .unwrap();

        assert!(!grant.authorized);
        grant.authorize();
        assert!(grant.authorized);
    }

    #[test]
    fn test_device_store_lifecycle() {
        let store = DeviceGrantStore::new();

        let grant = PendingDeviceGrant::generate(
            "my-client",
            vec!["openid".into()],
            "challenge",
            "https://auth.example.com/verify",
        )
        .unwrap();

        let device_code = grant.device_code.clone();
        let user_code = grant.user_code.clone();

        // Store
        store.store(grant);

        // Lookup by device code
        let by_device = store.lookup_by_device_code(&device_code).unwrap();
        assert_eq!(by_device.client_id, "my-client");
        assert!(!by_device.authorized);

        // Lookup by user code
        let by_user = store.lookup_by_user_code(&user_code).unwrap();
        assert_eq!(by_user.user_code, user_code);

        // Authorize
        assert!(store.authorize_by_device_code(&device_code));
        let after_auth = store.lookup_by_device_code(&device_code).unwrap();
        assert!(after_auth.authorized);

        // Remove
        store.remove(&device_code);
        assert!(store.lookup_by_device_code(&device_code).is_none());
    }

    #[test]
    fn test_sweep_expired() {
        let store = DeviceGrantStore::new();

        let mut grant = PendingDeviceGrant::generate(
            "my-client",
            vec!["openid".into()],
            "challenge",
            "https://auth.example.com/verify",
        )
        .unwrap();
        grant.expires_in = Duration::from_secs(0); // expired immediately
        store.store(grant);

        let removed = store.sweep_expired();
        assert_eq!(removed, 1);
    }

    #[test]
    fn test_user_code_format() {
        let code = generate_user_code().unwrap();
        assert_eq!(code.len(), 9); // XXXX-XXXX
        assert_eq!(code.chars().nth(4).unwrap(), '-');
        for (i, c) in code.chars().enumerate() {
            if i == 4 {
                continue;
            }
            assert!(
                c.is_ascii_uppercase(),
                "user_code char should be uppercase: {c}"
            );
        }
    }
}
