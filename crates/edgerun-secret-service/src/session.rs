//! D-Bus session management for the Secret Service.
//!
//! Handles OpenSession negotiation:
//! - `plain` — no encryption, secrets passed as-is over local socket
//! - `dh-ietf1024-sha256-aes128-cbc-pkcs7` — DH key exchange (stub: returns error)

use std::collections::HashMap;

// ===========================================================================
// Session state
// ===========================================================================

/// A negotiated D-Bus session.
#[derive(Clone, Debug)]
pub struct Session {
    /// Unique session path: /org/freedesktop/secrets/session/<id>
    pub path: String,
    /// Client unique name (D-Bus bus name, e.g. ":1.42")
    pub client: String,
    /// Negotiated algorithm
    pub algorithm: Algorithm,
    /// Whether the session is open
    pub closed: bool,
}

#[derive(Clone, Debug)]
pub enum Algorithm {
    /// No encryption — secrets passed in plaintext.
    Plain,
    /// DH with 1024-bit IETF Second Oakley Group, HKDF-SHA256, AES-128-CBC.
    /// Currently not implemented — clients must use `plain`.
    DhAes,
}

impl Session {
    pub fn new(path: String, client: String, algorithm: Algorithm) -> Self {
        Self { path, client, algorithm, closed: false }
    }
}

// ===========================================================================
// Session manager
// ===========================================================================

/// Tracks all open sessions.
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    counter: u64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { sessions: HashMap::new(), counter: 0 }
    }

    /// Creates a new session, returns its path.
    pub fn create_session(&mut self, client: &str, algorithm: Algorithm) -> String {
        self.counter += 1;
        let path = format!("/org/freedesktop/secrets/session/s{}", self.counter);
        let session = Session::new(path.clone(), client.to_string(), algorithm);
        self.sessions.insert(path.clone(), session);
        path
    }

    /// Looks up a session by path.
    pub fn get(&self, path: &str) -> Option<&Session> {
        self.sessions.get(path)
    }

    /// Gets mutable access to a session.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut Session> {
        self.sessions.get_mut(path)
    }

    /// Closes a session.
    pub fn close(&mut self, path: &str) {
        if let Some(s) = self.sessions.get_mut(path) {
            s.closed = true;
        }
    }

    /// Removes closed sessions.
    pub fn gc(&mut self) {
        self.sessions.retain(|_, s| !s.closed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_manager_create_and_close() {
        let mut mgr = SessionManager::new();
        let path = mgr.create_session(":1.42", Algorithm::Plain);
        assert!(path.starts_with("/org/freedesktop/secrets/session/"));

        let s = mgr.get(&path).unwrap();
        assert_eq!(s.client, ":1.42");
        assert!(!s.closed);

        mgr.close(&path);
        assert!(mgr.get(&path).unwrap().closed);

        mgr.gc();
        assert!(mgr.get(&path).is_none());
    }
}
