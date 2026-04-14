//! Persistent Maildir mailbox store (RFC 4155).
//!
//! Maildir format:
//! ```text
//! {root}/{user}/
//!   new/       — newly delivered messages (unread)
//!   cur/       — read messages
//!   tmp/       — in-flight delivery (atomic rename to new/)
//! ```
//!
//! Each message is a separate file with unique name:
//! `<timestamp>.<host>.<pid>.<counter>.edgerun`
//!
//! This crate stores messages as individual files — no database, no index files.
//! The filesystem IS the index.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use std::sync::RwLock;

use crate::server::dsn_generator::DsnBounce;
use crate::server::handler::{AuthCredentials, AuthResult, MailHandler};
use crate::types::MailEnvelope;
use edgerun_email_auth::AuthenticationResults;

// ===========================================================================
// MaildirStore
// ===========================================================================

/// Persistent Maildir mailbox store implementing [`MailHandler`].
///
/// Supports multiple users with separate Maildir directories.
/// Postmaster is always accepted (RFC 5321 §4.5.1).
pub struct MaildirStore {
    /// Root directory for all maildirs.
    root: PathBuf,
    /// Per-user domain mappings (e.g. "ken" → ["edgerun.mail", "localhost"]).
    user_domains: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Counter for unique message filenames.
    counter: AtomicU64,
    /// Validated sender domains (simple allowlist).
    valid_senders: Arc<RwLock<Vec<String>>>,
}

impl MaildirStore {
    /// Create a new MaildirStore at the given root path.
    pub fn new(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            user_domains: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicU64::new(0),
            valid_senders: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register a user with their accepted domains.
    ///
    /// Example: `store.add_user("ken", &["edgerun.mail", "localhost"])`
    /// → ken@edgerun.mail and ken@localhost will be accepted.
    pub fn add_user(&self, username: &str, domains: &[&str]) -> io::Result<()> {
        // Create Maildir structure
        let user_dir = self.root.join(username);
        fs::create_dir_all(user_dir.join("new"))?;
        fs::create_dir_all(user_dir.join("cur"))?;
        fs::create_dir_all(user_dir.join("tmp"))?;

        self.user_domains.write().unwrap().insert(
            username.to_string(),
            domains.iter().map(|d| d.to_string()).collect(),
        );

        Ok(())
    }

    /// Add a domain to the valid senders list (all senders from these domains are accepted).
    pub fn add_valid_domain(&self, domain: &str) {
        self.valid_senders.write().unwrap().push(domain.to_string());
    }

    /// Get the path to a user's Maildir.
    fn user_maildir(&self, username: &str) -> PathBuf {
        self.root.join(username)
    }

    /// Extract username from a recipient address.
    ///
    /// Returns the local part (before @) if it's a valid user.
    fn extract_user(&self, address: &str) -> Option<String> {
        let addr = address.trim().trim_start_matches('<').trim_end_matches('>');
        if let Some(at_pos) = addr.rfind('@') {
            let local = &addr[..at_pos];
            let domain = &addr[at_pos + 1..];

            // Check if this user exists and domain is valid
            let domains = self.user_domains.read().unwrap();
            if domains.contains_key(local) {
                if let Some(user_domains) = domains.get(local) {
                    if user_domains.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
                        return Some(local.to_string());
                    }
                }
            }

            // Also accept if domain matches any registered user's domains
            for (_user, user_doms) in domains.iter() {
                if user_doms.iter().any(|d| d.eq_ignore_ascii_case(domain)) {
                    // Domain is valid — check if user exists
                    if domains.contains_key(local) {
                        return Some(local.to_string());
                    }
                }
            }
        }

        // Fallback: check if address matches a username directly
        let domains = self.user_domains.read().unwrap();
        if domains.contains_key(addr) {
            return Some(addr.to_string());
        }

        None
    }

    /// Generate a unique Maildir filename.
    fn unique_filename(&self) -> String {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let hostname = hostname();
        let pid = std::process::id();

        format!("{}.M{}P{}.Q{}.edgerun", ts, hostname, pid, counter)
    }

    /// List messages in a user's mailbox.
    pub fn list_messages(&self, username: &str) -> io::Result<Vec<MaildirMessage>> {
        let user_dir = self.user_maildir(username);
        let mut messages = Vec::new();

        // Scan new/
        if let Ok(entries) = fs::read_dir(user_dir.join("new")) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.path().is_file() {
                        messages.push(MaildirMessage {
                            path: entry.path(),
                            state: MaildirState::New,
                        });
                    }
                }
            }
        }

        // Scan cur/
        if let Ok(entries) = fs::read_dir(user_dir.join("cur")) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry.path().is_file() {
                        messages.push(MaildirMessage {
                            path: entry.path(),
                            state: MaildirState::Cur,
                        });
                    }
                }
            }
        }

        messages.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(messages)
    }

    /// Read a message from the mailbox.
    pub fn read_message(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    /// Move a message from new/ to cur/ (mark as read).
    pub fn mark_read(&self, path: &Path) -> io::Result<()> {
        let filename = path.file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no filename"))?;
        let cur_path = self.root.join("cur").join(filename);
        fs::rename(path, cur_path)?;
        Ok(())
    }

    /// Delete a message from the mailbox.
    pub fn delete_message(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    /// Get mailbox statistics for a user.
    pub fn mailbox_stats(&self, username: &str) -> io::Result<MailboxStats> {
        let user_dir = self.user_maildir(username);

        let new_count = fs::read_dir(user_dir.join("new"))
            .map(|e| e.filter(|e| e.as_ref().map(|e| e.path().is_file()).unwrap_or(false)).count())
            .unwrap_or(0);

        let cur_count = fs::read_dir(user_dir.join("cur"))
            .map(|e| e.filter(|e| e.as_ref().map(|e| e.path().is_file()).unwrap_or(false)).count())
            .unwrap_or(0);

        let total_size = self.list_messages(username)?
            .iter()
            .filter_map(|m| m.path.metadata().ok().map(|m| m.len()))
            .sum();

        Ok(MailboxStats {
            new_count,
            cur_count,
            total_count: new_count + cur_count,
            total_size,
        })
    }
}

// ===========================================================================
// MailHandler Implementation
// ===========================================================================

impl MailHandler for MaildirStore {
    fn validate_sender(&self, address: &str) -> io::Result<()> {
        // Accept all senders (open relay for validation — real filtering happens at SMTP level)
        let _ = address;
        Ok(())
    }

    fn validate_recipient(&self, address: &str) -> io::Result<()> {
        // Postmaster is always accepted (RFC 5321 §4.5.1)
        let lower = address.to_lowercase();
        if lower.starts_with("postmaster@") || lower == "postmaster" {
            return Ok(());
        }

        // Check if this is a valid user@domain
        if self.extract_user(address).is_some() {
            return Ok(());
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("recipient not found: {}", address),
        ))
    }

    fn accept_mail(&self, envelope: &MailEnvelope) -> io::Result<()> {
        for recipient in &envelope.recipients {
            if let Some(username) = self.extract_user(recipient) {
                // Write to tmp/ first, then rename to new/ (atomic delivery)
                let tmp_path = self.user_maildir(&username).join("tmp").join(self.unique_filename());
                let new_path = self.user_maildir(&username).join("new").join(tmp_path.file_name().unwrap());

                // Write the message
                let mut file = fs::File::create(&tmp_path)?;
                file.write_all(&envelope.data)?;
                file.sync_all()?;

                // Atomic rename: tmp/ → new/
                fs::rename(&tmp_path, &new_path)?;
            }
        }

        Ok(())
    }

    fn authenticate(
        &self,
        _mechanism: &str,
        _credentials: &AuthCredentials,
    ) -> AuthResult {
        AuthResult::Unsupported
    }

    fn auth_required(&self) -> bool {
        false
    }

    fn send_bounce(&self, _bounce: &DsnBounce) -> io::Result<()> {
        // Bounce handling is done by the relay system
        Ok(())
    }

    fn on_mail_received(
        &self,
        _envelope: &MailEnvelope,
        _auth_results: &edgerun_email_auth::AuthenticationResults,
    ) {
        // Logged by the SMTP server; no additional action needed
    }
}

// ===========================================================================
// Message Types
// ===========================================================================

/// A message in a Maildir mailbox.
#[derive(Debug, Clone)]
pub struct MaildirMessage {
    /// Full path to the message file.
    pub path: PathBuf,
    /// Message state (new or cur).
    pub state: MaildirState,
}

/// Message state in Maildir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaildirState {
    /// Newly delivered, not yet read.
    New,
    /// Has been seen by the MUA (read).
    Cur,
}

/// Mailbox statistics.
#[derive(Debug, Clone)]
pub struct MailboxStats {
    /// Number of new (unread) messages.
    pub new_count: usize,
    /// Number of read messages.
    pub cur_count: usize,
    /// Total messages.
    pub total_count: usize,
    /// Total size in bytes.
    pub total_size: u64,
}

// ===========================================================================
// Helpers
// ===========================================================================

fn hostname() -> String {
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|a| format!("{}", a))
        .unwrap_or_else(|| "localhost".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("edgerun_maildir_{}", name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_create_store() {
        let dir = test_dir("create_store");
        let store = MaildirStore::new(&dir).unwrap();
        assert!(dir.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_add_user_creates_maildir() {
        let dir = test_dir("add_user");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail"]).unwrap();

        assert!(dir.join("ken/new").exists());
        assert!(dir.join("ken/cur").exists());
        assert!(dir.join("ken/tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_recipient_postmaster() {
        let dir = test_dir("postmaster");
        let store = MaildirStore::new(&dir).unwrap();

        assert!(store.validate_recipient("postmaster@edgerun.mail").is_ok());
        assert!(store.validate_recipient("POSTMASTER@edgerun.mail").is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_recipient_known_user() {
        let dir = test_dir("known_user");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail", "localhost"]).unwrap();

        assert!(store.validate_recipient("ken@edgerun.mail").is_ok());
        assert!(store.validate_recipient("ken@localhost").is_ok());
        assert!(store.validate_recipient("unknown@edgerun.mail").is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_accept_mail_delivers_to_maildir() {
        let dir = test_dir("deliver");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail"]).unwrap();

        let mut envelope = MailEnvelope::new("sender@gmail.com".to_string());
        envelope.recipients.push("ken@edgerun.mail".to_string());
        envelope.data = b"From: sender@gmail.com\r\nTo: ken@edgerun.mail\r\n\r\nHello".to_vec();

        store.accept_mail(&envelope).unwrap();

        let new_dir = dir.join("ken/new");
        let entries: Vec<_> = fs::read_dir(&new_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);

        let entry = entries[0].as_ref().unwrap();
        let content = fs::read(entry.path()).unwrap();
        assert_eq!(content, envelope.data);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_messages() {
        let dir = test_dir("list");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail"]).unwrap();

        // Deliver two messages
        for i in 0..2 {
            let mut envelope = MailEnvelope::new("sender@gmail.com".to_string());
            envelope.recipients.push("ken@edgerun.mail".to_string());
            envelope.data = format!("Message {}", i).into_bytes();
            store.accept_mail(&envelope).unwrap();
        }

        let messages = store.list_messages("ken").unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages.iter().all(|m| matches!(m.state, MaildirState::New)));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_mailbox_stats() {
        let dir = test_dir("stats");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail"]).unwrap();

        // Deliver 3 messages
        for i in 0..3 {
            let mut envelope = MailEnvelope::new("sender@gmail.com".to_string());
            envelope.recipients.push("ken@edgerun.mail".to_string());
            envelope.data = format!("Message {}", i).into_bytes();
            store.accept_mail(&envelope).unwrap();
        }

        let stats = store.mailbox_stats("ken").unwrap();
        assert_eq!(stats.new_count, 3);
        assert_eq!(stats.cur_count, 0);
        assert_eq!(stats.total_count, 3);
        assert!(stats.total_size > 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_user() {
        let dir = test_dir("extract_user");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail", "localhost"]).unwrap();
        store.add_user("admin", &["edgerun.mail"]).unwrap();

        assert_eq!(store.extract_user("ken@edgerun.mail"), Some("ken".to_string()));
        assert_eq!(store.extract_user("admin@edgerun.mail"), Some("admin".to_string()));
        assert_eq!(store.extract_user("unknown@edgerun.mail"), None);

        let _ = fs::remove_dir_all(&dir);
    }
}
