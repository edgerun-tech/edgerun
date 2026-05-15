//! D-Bus session management with biometric verification.
//!
//! Sessions track biometric verification state. Secrets are only accessible
//! when the session has an active biometric verification. After an idle
//! timeout, the session auto-locks and requires re-verification.

use crate::prelude::v1::*;
use alloc::collections::BTreeMap as HashMap;
use edgerun_devices::biometrics::{BiometricAssuranceStrength, BiometricState};

// ===========================================================================
// Session state
// ===========================================================================

/// Default idle timeout before auto-lock (5 minutes).
pub const DEFAULT_IDLE_TIMEOUT_US: u64 = 5 * 60 * 1_000_000;

/// A negotiated D-Bus session with biometric verification state.
#[derive(Clone, Debug)]
pub struct Session {
    pub path: String,
    pub client: String,
    pub biometric_state: BiometricState,
    /// Microseconds since epoch of last successful biometric verification.
    pub last_verified_us: Option<u64>,
    /// Microseconds of inactivity before auto-lock.
    pub idle_timeout_us: u64,
    pub closed: bool,
}

impl Session {
    pub fn new(path: String, client: String, idle_timeout_us: u64) -> Self {
        Self {
            path,
            client,
            biometric_state: BiometricState::default(),
            last_verified_us: None,
            idle_timeout_us,
            closed: false,
        }
    }

    /// Check if the session is biometrically verified and not expired.
    pub fn is_verified(&self, now_us: u64) -> bool {
        if !self.biometric_state.verified {
            return false;
        }
        // Check idle timeout
        if let Some(last) = self.last_verified_us {
            if now_us.saturating_sub(last) > self.idle_timeout_us {
                return false; // auto-locked
            }
        } else {
            return false; // never been verified
        }
        true
    }

    /// Record a successful biometric verification.
    pub fn mark_verified(&mut self, state: BiometricState, now_us: u64) {
        self.biometric_state = state;
        self.last_verified_us = Some(now_us);
    }

    /// Lock the session — clear biometric state.
    pub fn lock(&mut self) {
        self.biometric_state = BiometricState::default();
        self.last_verified_us = None;
    }

    /// Check for auto-lock due to idle timeout.
    /// Returns true if the session was auto-locked.
    pub fn maybe_auto_lock(&mut self, now_us: u64) -> bool {
        if self.biometric_state.verified {
            if let Some(last) = self.last_verified_us {
                if now_us.saturating_sub(last) > self.idle_timeout_us {
                    self.lock();
                    return true;
                }
            }
        }
        false
    }

    /// Get assurance strength of current session.
    pub fn assurance_strength(&self) -> BiometricAssuranceStrength {
        self.biometric_state.assurance_strength()
    }
}

// ===========================================================================
// Biometric verifier
// ===========================================================================

/// Trait for biometric verification.
///
/// Implementations can use fingerprint, face, iris, or other modalities.
/// In production this would call through to hardware (TPM, fingerprint sensor,
/// camera with face detection). For testing, a mock verifier can be used.
pub trait BiometricVerifier: Send {
    /// Attempt biometric verification.
    ///
    /// Returns `BiometricState` with `verified: true` if verification succeeded.
    /// The implementation is responsible for interacting with hardware and
    /// returning the appropriate assurance strength.
    fn verify(&self) -> BiometricState;

    /// Check if any biometric hardware is available.
    fn is_available(&self) -> bool;
}

/// No-op verifier — always fails verification.
/// Used when no biometric hardware is present.
pub struct NoBiometricVerifier;

impl BiometricVerifier for NoBiometricVerifier {
    fn verify(&self) -> BiometricState {
        BiometricState::default()
    }

    fn is_available(&self) -> bool {
        false
    }
}

// ===========================================================================
// Session manager
// ===========================================================================

/// Tracks all open sessions.
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    counter: u64,
    idle_timeout_us: u64,
}

impl SessionManager {
    pub fn new(idle_timeout_us: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            counter: 0,
            idle_timeout_us,
        }
    }

    /// Creates a new session, returns its path.
    pub fn create_session(&mut self, client: &str) -> String {
        self.counter += 1;
        let path = format!("/org/freedesktop/secrets/session/s{}", self.counter);
        let session = Session::new(path.clone(), client.to_string(), self.idle_timeout_us);
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

    /// Lock all sessions for a client.
    pub fn lock_client(&mut self, client: &str) {
        let keys: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.client == client)
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys {
            if let Some(s) = self.sessions.get_mut(&key) {
                s.lock();
            }
        }
    }

    /// Verify all sessions for a client using biometric state.
    pub fn verify_client(&mut self, client: &str, state: BiometricState, now_us: u64) {
        let keys: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.client == client)
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys {
            if let Some(s) = self.sessions.get_mut(&key) {
                s.mark_verified(state.clone(), now_us);
            }
        }
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

    /// Auto-lock all idle sessions.
    /// Returns the number of sessions that were auto-locked.
    pub fn auto_lock_idle(&mut self) -> usize {
        let now = now_us();
        let mut count = 0;
        for (_, s) in self.sessions.iter_mut() {
            if !s.closed && s.maybe_auto_lock(now) {
                count += 1;
            }
        }
        count
    }
}

fn now_us() -> u64 {
    edgerun_time::now_unix_micros()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_manager_create_and_close() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let path = mgr.create_session(":1.42");
        assert!(path.starts_with("/org/freedesktop/secrets/session/"));

        let s = mgr.get(&path).unwrap();
        assert_eq!(s.client, ":1.42");
        assert!(!s.closed);
        assert!(!s.is_verified(now_us()));

        mgr.close(&path);
        assert!(mgr.get(&path).unwrap().closed);

        mgr.gc();
        assert!(mgr.get(&path).is_none());
    }

    #[test]
    fn session_not_verified_by_default() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let path = mgr.create_session(":1.1");

        let s = mgr.get(&path).unwrap();
        assert!(!s.is_verified(now_us()));
        assert_eq!(s.assurance_strength(), BiometricAssuranceStrength::None);
    }

    #[test]
    fn session_lock_clears_state() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let path = mgr.create_session(":1.1");

        // Simulate verification
        {
            let s = mgr.get_mut(&path).unwrap();
            let mut state = BiometricState::default();
            state.verified = true;
            s.mark_verified(state, now_us());
        }
        assert!(mgr.get(&path).unwrap().is_verified(now_us()));

        // Lock
        mgr.get_mut(&path).unwrap().lock();
        assert!(!mgr.get(&path).unwrap().is_verified(now_us()));
    }

    #[test]
    fn session_auto_locks_on_idle() {
        // Very short timeout for testing
        let mut mgr = SessionManager::new(50_000); // 50ms
        let path = mgr.create_session(":1.1");

        // Verify
        {
            let s = mgr.get_mut(&path).unwrap();
            let mut state = BiometricState::default();
            state.verified = true;
            s.mark_verified(state, now_us());
        }
        assert!(mgr.get(&path).unwrap().is_verified(now_us()));

        // Wait for timeout
        std::thread::sleep(std::time::Duration::from_millis(60));

        // Should be auto-locked now (is_verified checks timeout)
        let now = now_us();
        assert!(!mgr.get(&path).unwrap().is_verified(now));
    }

    #[test]
    fn assurance_strength_variants() {
        let mut s = Session::new("/path".into(), ":1.1".into(), DEFAULT_IDLE_TIMEOUT_US);

        // Default: none
        assert_eq!(s.assurance_strength(), BiometricAssuranceStrength::None);

        // User presence only
        let mut state = BiometricState::default();
        state.user_present = true;
        s.mark_verified(state, now_us());
        assert_eq!(
            s.assurance_strength(),
            BiometricAssuranceStrength::UserPresence
        );

        // Biometric match
        let mut state = BiometricState::default();
        state.verified = true;
        s.mark_verified(state, now_us());
        assert_eq!(
            s.assurance_strength(),
            BiometricAssuranceStrength::BiometricMatch
        );

        // Hardware protected
        let mut state = BiometricState::default();
        state.verified = true;
        state.hardware_protected = true;
        s.mark_verified(state, now_us());
        assert_eq!(
            s.assurance_strength(),
            BiometricAssuranceStrength::HardwareProtectedBiometric
        );
    }

    #[test]
    fn no_biometric_verifier_always_fails() {
        let v = NoBiometricVerifier;
        assert!(!v.is_available());
        let state = v.verify();
        assert!(!state.verified);
    }

    #[test]
    fn session_manager_lock_client_all_sessions_locked() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let p1 = mgr.create_session(":1.42");
        let p2 = mgr.create_session(":1.42");
        let p3 = mgr.create_session(":1.42");
        {
            let mut state = BiometricState::default();
            state.verified = true;
            mgr.verify_client(":1.42", state.clone(), now_us());
        }
        assert!(mgr.get(&p1).unwrap().is_verified(now_us()));
        assert!(mgr.get(&p2).unwrap().is_verified(now_us()));
        assert!(mgr.get(&p3).unwrap().is_verified(now_us()));
        mgr.lock_client(":1.42");
        assert!(!mgr.get(&p1).unwrap().is_verified(now_us()));
        assert!(!mgr.get(&p2).unwrap().is_verified(now_us()));
        assert!(!mgr.get(&p3).unwrap().is_verified(now_us()));
    }

    #[test]
    fn session_manager_lock_client_other_clients_unchanged() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let p1 = mgr.create_session(":1.42");
        let p2 = mgr.create_session(":1.99");
        {
            let mut state = BiometricState::default();
            state.verified = true;
            mgr.verify_client(":1.42", state.clone(), now_us());
            mgr.verify_client(":1.99", state.clone(), now_us());
        }
        mgr.lock_client(":1.42");
        assert!(!mgr.get(&p1).unwrap().is_verified(now_us()));
        assert!(mgr.get(&p2).unwrap().is_verified(now_us()));
    }

    #[test]
    fn session_manager_verify_client_multiple_sessions() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let p1 = mgr.create_session(":1.42");
        let p2 = mgr.create_session(":1.42");
        let mut state = BiometricState::default();
        state.verified = true;
        state.hardware_protected = true;
        mgr.verify_client(":1.42", state, now_us());
        assert!(mgr.get(&p1).unwrap().is_verified(now_us()));
        assert!(mgr.get(&p2).unwrap().is_verified(now_us()));
        assert_eq!(
            mgr.get(&p1).unwrap().assurance_strength(),
            BiometricAssuranceStrength::HardwareProtectedBiometric
        );
    }

    #[test]
    fn session_manager_auto_lock_idle_returns_count() {
        let mut mgr = SessionManager::new(50_000);
        let _p1 = mgr.create_session(":1.1");
        let _p2 = mgr.create_session(":1.1");
        let _p3 = mgr.create_session(":1.1");
        {
            let mut state = BiometricState::default();
            state.verified = true;
            mgr.verify_client(":1.1", state, now_us());
        }
        std::thread::sleep(std::time::Duration::from_millis(60));
        let count = mgr.auto_lock_idle();
        assert_eq!(count, 3);
    }

    #[test]
    fn session_manager_auto_lock_idle_mixed_states() {
        let mut mgr = SessionManager::new(50_000);
        let p1 = mgr.create_session(":1.1");
        let p2 = mgr.create_session(":1.1");
        {
            let mut state = BiometricState::default();
            state.verified = true;
            mgr.verify_client(":1.1", state, now_us());
        }
        if let Some(s) = mgr.get_mut(&p2) {
            s.lock();
        }
        std::thread::sleep(std::time::Duration::from_millis(60));
        let count = mgr.auto_lock_idle();
        assert_eq!(count, 1);
    }

    #[test]
    fn session_manager_gc_removes_closed_only() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let p1 = mgr.create_session(":1.1");
        let p2 = mgr.create_session(":1.2");
        let p3 = mgr.create_session(":1.3");
        mgr.close(&p1);
        mgr.close(&p3);
        mgr.gc();
        assert!(mgr.get(&p1).is_none());
        assert!(mgr.get(&p2).is_some());
        assert!(mgr.get(&p3).is_none());
    }

    #[test]
    fn session_manager_counter_increments() {
        let mut mgr = SessionManager::new(DEFAULT_IDLE_TIMEOUT_US);
        let p1 = mgr.create_session(":1.1");
        let p2 = mgr.create_session(":1.1");
        let p3 = mgr.create_session(":1.1");
        assert_eq!(p1, "/org/freedesktop/secrets/session/s1");
        assert_eq!(p2, "/org/freedesktop/secrets/session/s2");
        assert_eq!(p3, "/org/freedesktop/secrets/session/s3");
    }

    #[test]
    fn session_maybe_auto_lock_not_verified() {
        let mut mgr = SessionManager::new(0);
        let path = mgr.create_session(":1.1");
        assert!(!mgr.get_mut(&path).unwrap().maybe_auto_lock(now_us()));
    }
}
