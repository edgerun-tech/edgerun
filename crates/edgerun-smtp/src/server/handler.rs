//! Pluggable mail backend trait and in-memory implementation.

use std::io;

use crate::types::MailEnvelope;

// ===========================================================================
// Auth Result
// ===========================================================================

/// Result of an authentication attempt.
#[derive(Debug)]
pub enum AuthResult {
    /// Authentication succeeded — returns the authenticated identity.
    Authenticated(String),
    /// Authentication failed — client may retry.
    Failed,
    /// Mechanism not supported.
    Unsupported,
}

// ===========================================================================
// MailHandler Trait
// ===========================================================================

/// Trait for a pluggable mail backend.
pub trait MailHandler: Send + Sync + 'static {
    /// Validate a sender address. Returns `Ok(())` if accepted.
    fn validate_sender(&self, address: &str) -> io::Result<()>;

    /// Validate a recipient address. Returns `Ok(())` if accepted.
    /// Special: `postmaster@<domain>` MUST always be accepted (RFC 5321 §4.5.1).
    fn validate_recipient(&self, address: &str) -> io::Result<()>;

    /// Accept a complete mail transaction.
    fn accept_mail(&self, envelope: &MailEnvelope) -> io::Result<()>;

    /// Authenticate a user via SASL mechanism.
    /// Default implementation returns `AuthResult::Unsupported`.
    fn authenticate(
        &self,
        _mechanism: &str,
        _credentials: &AuthCredentials,
    ) -> AuthResult {
        AuthResult::Unsupported
    }

    /// Whether authentication is required before MAIL FROM.
    fn auth_required(&self) -> bool {
        false
    }
}

// ===========================================================================
// Auth Credentials
// ===========================================================================

/// Decoded SASL credentials.
pub struct AuthCredentials {
    /// Authorization identity (who the client wants to act as).
    pub authz_id: String,
    /// Authentication identity (who the client claims to be).
    pub authc_id: String,
    /// Password / secret.
    pub password: String,
}

impl AuthCredentials {
    /// Decode SASL PLAIN mechanism payload.
    /// Format: `authz_id\0authc_id\0password`
    pub fn from_plain(payload: &[u8]) -> io::Result<Self> {
        let text = std::str::from_utf8(payload)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let parts: Vec<&str> = text.split('\0').collect();
        if parts.len() < 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SASL PLAIN requires authz_id\\0authc_id\\0password",
            ));
        }

        Ok(Self {
            authz_id: parts[0].to_string(),
            authc_id: parts[1].to_string(),
            password: parts[2].to_string(),
        })
    }
}

// ===========================================================================
// In-Memory Mail Handler (testing)
// ===========================================================================

/// Simple in-memory mail store.
pub struct MemoryMailStore {
    mailboxes: std::sync::Mutex<std::collections::HashMap<String, Vec<MailEnvelope>>>,
    users: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MemoryMailStore {
    pub fn new() -> Self {
        Self {
            mailboxes: std::sync::Mutex::new(std::collections::HashMap::new()),
            users: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Add a test user with a plaintext password.
    pub fn add_user(&self, email: &str, password: &str) {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        mailboxes.insert(email.to_string(), Vec::new());
        let mut users = self.users.lock().unwrap();
        users.insert(email.to_string(), password.to_string());
    }
}

impl Default for MemoryMailStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MailHandler for MemoryMailStore {
    fn validate_sender(&self, _address: &str) -> io::Result<()> {
        Ok(())
    }

    fn validate_recipient(&self, address: &str) -> io::Result<()> {
        // postmaster must always be accepted
        if address.eq_ignore_ascii_case("postmaster")
            || address.starts_with("postmaster@")
        {
            return Ok(());
        }

        let mailboxes = self.mailboxes.lock().unwrap();
        if mailboxes.contains_key(address) {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "mailbox not found"))
        }
    }

    fn accept_mail(&self, envelope: &MailEnvelope) -> io::Result<()> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        for recipient in &envelope.recipients {
            if let Some(mailbox) = mailboxes.get_mut(recipient) {
                mailbox.push(envelope.clone());
            }
        }
        Ok(())
    }

    fn authenticate(&self, mechanism: &str, credentials: &AuthCredentials) -> AuthResult {
        match mechanism.to_uppercase().as_str() {
            "PLAIN" | "LOGIN" => {
                let users = self.users.lock().unwrap();
                if let Some(stored_pass) = users.get(&credentials.authc_id) {
                    if stored_pass == &credentials.password {
                        return AuthResult::Authenticated(credentials.authc_id.clone());
                    }
                }
                AuthResult::Failed
            }
            _ => AuthResult::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_decode() {
        // "\0user@example.com\0password"
        let payload = b"\x00user@example.com\x00password";
        let creds = AuthCredentials::from_plain(payload).unwrap();
        assert_eq!(creds.authz_id, "");
        assert_eq!(creds.authc_id, "user@example.com");
        assert_eq!(creds.password, "password");
    }

    #[test]
    fn test_plain_decode_with_authz() {
        // "admin\0user@example.com\0password"
        let payload = b"admin\x00user@example.com\x00password";
        let creds = AuthCredentials::from_plain(payload).unwrap();
        assert_eq!(creds.authz_id, "admin");
        assert_eq!(creds.authc_id, "user@example.com");
        assert_eq!(creds.password, "password");
    }

    #[test]
    fn test_plain_decode_invalid() {
        assert!(AuthCredentials::from_plain(b"nodellama").is_err());
    }

    #[test]
    fn test_memory_store_auth() {
        let store = MemoryMailStore::new();
        store.add_user("user@example.com", "secret123");

        let creds = AuthCredentials {
            authz_id: String::new(),
            authc_id: "user@example.com".to_string(),
            password: "secret123".to_string(),
        };
        match store.authenticate("PLAIN", &creds) {
            AuthResult::Authenticated(id) => assert_eq!(id, "user@example.com"),
            other => panic!("Expected Authenticated, got {:?}", other),
        }
    }

    #[test]
    fn test_memory_store_auth_fail() {
        let store = MemoryMailStore::new();
        store.add_user("user@example.com", "secret123");

        let creds = AuthCredentials {
            authz_id: String::new(),
            authc_id: "user@example.com".to_string(),
            password: "wrong".to_string(),
        };
        assert!(matches!(store.authenticate("PLAIN", &creds), AuthResult::Failed));
    }

    #[test]
    fn test_memory_store_auth_unsupported_mechanism() {
        let store = MemoryMailStore::new();
        let creds = AuthCredentials {
            authz_id: String::new(),
            authc_id: "user".to_string(),
            password: "pass".to_string(),
        };
        assert!(matches!(store.authenticate("CRAM-MD5", &creds), AuthResult::Unsupported));
    }

    #[test]
    fn test_postmaster_always_accepted() {
        let store = MemoryMailStore::new();
        assert!(store.validate_recipient("postmaster").is_ok());
        assert!(store.validate_recipient("postmaster@example.com").is_ok());
        assert!(store.validate_recipient("POSTMASTER").is_ok());
    }
}
