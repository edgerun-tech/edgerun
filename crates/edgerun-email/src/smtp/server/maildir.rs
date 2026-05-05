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

use crate::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;

use crate::smtp::server::dsn_generator::DsnBounce;
use crate::smtp::server::handler::{AuthCredentials, AuthResult, MailHandler};
use crate::smtp::types::MailEnvelope;

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
    /// Optional mailbox that receives unknown users at registered local domains.
    catch_all_user: Arc<RwLock<Option<String>>>,
    /// SMTP AUTH tokens keyed by username and address aliases.
    auth_users: Arc<RwLock<HashMap<String, String>>>,
    /// Counter for unique message filenames.
    counter: AtomicU64,
    /// Stable per-process token used in Maildir filenames.
    ///
    /// This is computed once at store creation. The old filename helper opened
    /// a loopback TCP listener for every delivered message, which made local
    /// delivery pay unnecessary socket setup cost on the SMTP hot path.
    filename_host: String,
    /// Validated sender domains (simple allowlist).
    valid_senders: Arc<RwLock<Vec<String>>>,
    /// In-process mailbox status delta index. SMTP delivery only appends new
    /// messages, so batching those deltas avoids a lock/read/write cycle for
    /// every accepted message while preserving the on-disk sidecar for IMAP.
    status_index: Arc<MaildirStatusIndex>,
    /// Whether to fsync each delivered message before the Maildir tmp->new
    /// rename. Atomic delivery does not require this, but strict crash
    /// durability can opt back in with EDGERUN_MAILDIR_SYNC_DELIVERY=1.
    sync_delivery: bool,
}

impl MaildirStore {
    /// Create a new MaildirStore at the given root path.
    pub fn new(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        let status_index = MaildirStatusIndex::new(root.to_path_buf());
        Ok(Self {
            root: root.to_path_buf(),
            user_domains: Arc::new(RwLock::new(HashMap::new())),
            catch_all_user: Arc::new(RwLock::new(None)),
            auth_users: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicU64::new(0),
            filename_host: filename_host_token(),
            valid_senders: Arc::new(RwLock::new(Vec::new())),
            status_index,
            sync_delivery: maildir_sync_delivery_enabled(),
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
        self.ensure_mailbox_status(username)?;

        self.user_domains.write().unwrap().insert(
            username.to_string(),
            domains.iter().map(|d| d.to_string()).collect(),
        );

        Ok(())
    }

    /// Register SMTP AUTH credentials for a local mailbox user.
    pub fn set_user_token(&self, username: &str, token: &str) -> io::Result<()> {
        let domains = self.user_domains.read().unwrap();
        let Some(user_domains) = domains.get(username) else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("auth user not registered: {username}"),
            ));
        };

        let mut auth_users = self.auth_users.write().unwrap();
        auth_users.insert(username.to_ascii_lowercase(), token.to_string());
        for domain in user_domains {
            auth_users.insert(
                format!("{}@{}", username, domain).to_ascii_lowercase(),
                token.to_string(),
            );
        }
        Ok(())
    }

    /// Route unknown local recipients to an existing mailbox user.
    pub fn set_catch_all_user(&self, username: &str) -> io::Result<()> {
        if !self.user_domains.read().unwrap().contains_key(username) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("catch-all user not registered: {username}"),
            ));
        }
        *self.catch_all_user.write().unwrap() = Some(username.to_string());
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

    fn ensure_mailbox_status(&self, username: &str) -> io::Result<()> {
        self.status_index.ensure(username, || {
            let stats = self.scan_mailbox_stats(username)?;
            Ok(MailboxStatusCounters {
                messages: stats.total_count as u32,
                recent: stats.new_count as u32,
                uid_next: stats.total_count as u32 + 1,
            })
        })
    }

    fn read_mailbox_status(&self, username: &str) -> io::Result<Option<MailboxStatusCounters>> {
        self.status_index.read_effective(username)
    }

    fn increment_mailbox_status(&self, username: &str) -> io::Result<()> {
        self.status_index.increment_delivery(username)
    }

    pub fn flush_mailbox_statuses(&self) -> io::Result<()> {
        self.status_index.flush_all()
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
                    if let Some(catch_all_user) = self.catch_all_user.read().unwrap().clone() {
                        if domains.contains_key(&catch_all_user) {
                            return Some(catch_all_user);
                        }
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
        let pid = std::process::id();

        format!(
            "{}.M{}P{}.Q{}.edgerun",
            ts, self.filename_host, pid, counter
        )
    }

    /// List messages in a user's mailbox.
    pub fn list_messages(&self, username: &str) -> io::Result<Vec<MaildirMessage>> {
        let user_dir = self.user_maildir(username);
        let mut messages = Vec::new();

        // Scan new/
        if let Ok(entries) = fs::read_dir(user_dir.join("new")) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    messages.push(MaildirMessage {
                        path: entry.path(),
                        state: MaildirState::New,
                    });
                }
            }
        }

        // Scan cur/
        if let Ok(entries) = fs::read_dir(user_dir.join("cur")) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    messages.push(MaildirMessage {
                        path: entry.path(),
                        state: MaildirState::Cur,
                    });
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
        let filename = path
            .file_name()
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
        let counters = match self.read_mailbox_status(username)? {
            Some(counters) => counters,
            None => {
                self.ensure_mailbox_status(username)?;
                self.read_mailbox_status(username)?.unwrap_or_default()
            }
        };
        let total_size = self
            .list_messages(username)?
            .iter()
            .filter_map(|m| m.path.metadata().ok().map(|m| m.len()))
            .sum();

        Ok(MailboxStats {
            new_count: counters.recent as usize,
            cur_count: counters.messages.saturating_sub(counters.recent) as usize,
            total_count: counters.messages as usize,
            total_size,
        })
    }

    fn scan_mailbox_stats(&self, username: &str) -> io::Result<MailboxStats> {
        let user_dir = self.user_maildir(username);

        let new_count = fs::read_dir(user_dir.join("new"))
            .map(|e| {
                e.filter(|e| e.as_ref().map(|e| e.path().is_file()).unwrap_or(false))
                    .count()
            })
            .unwrap_or(0);

        let cur_count = fs::read_dir(user_dir.join("cur"))
            .map(|e| {
                e.filter(|e| e.as_ref().map(|e| e.path().is_file()).unwrap_or(false))
                    .count()
            })
            .unwrap_or(0);

        let total_size = self
            .list_messages(username)?
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
        self.accept_mail_for_recipients(envelope, &envelope.recipients)
    }

    fn accept_mail_for_recipients(
        &self,
        envelope: &MailEnvelope,
        recipients: &[String],
    ) -> io::Result<()> {
        for recipient in recipients {
            if let Some(username) = self.extract_user(recipient) {
                // Write to tmp/ first, then rename to new/ (atomic delivery)
                let tmp_path = self
                    .user_maildir(&username)
                    .join("tmp")
                    .join(self.unique_filename());
                let new_path = self
                    .user_maildir(&username)
                    .join("new")
                    .join(tmp_path.file_name().unwrap());

                // Write the message
                let mut file = fs::File::create(&tmp_path)?;
                file.write_all(&envelope.data)?;
                if self.sync_delivery {
                    file.sync_all()?;
                }

                // Atomic rename: tmp/ → new/
                fs::rename(&tmp_path, &new_path)?;
                if let Err(e) = self.increment_mailbox_status(&username) {
                    edgerun_log::warn!(
                        "edgerun-smtp: failed to update Maildir status index for {}: {}",
                        username,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    fn authenticate(&self, mechanism: &str, credentials: &AuthCredentials) -> AuthResult {
        if !matches!(mechanism.to_ascii_uppercase().as_str(), "PLAIN" | "LOGIN") {
            return AuthResult::Unsupported;
        }

        let authc_id = credentials.authc_id.to_ascii_lowercase();
        let authz_id = credentials.authz_id.to_ascii_lowercase();
        let auth_users = self.auth_users.read().unwrap();
        let Some(stored_token) = auth_users.get(&authc_id) else {
            return AuthResult::Failed;
        };
        if stored_token != &credentials.token {
            return AuthResult::Failed;
        }
        if !authz_id.is_empty()
            && authz_id != authc_id
            && auth_local_part(&authz_id) != auth_local_part(&authc_id)
        {
            return AuthResult::Failed;
        }
        AuthResult::Authenticated(credentials.authc_id.clone())
    }

    fn auth_required(&self) -> bool {
        false
    }

    fn send_bounce(&self, _bounce: &DsnBounce) -> io::Result<()> {
        // Bounce handling is done by the relay system
        Ok(())
    }

    #[cfg(feature = "dkim")]
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

#[derive(Debug, Clone, Copy, Default)]
struct MailboxStatusCounters {
    messages: u32,
    recent: u32,
    uid_next: u32,
}

#[derive(Debug, Default)]
struct MailboxStatusDelta {
    pending_deliveries: u32,
}

struct MaildirStatusIndex {
    root: PathBuf,
    entries: Mutex<HashMap<String, MailboxStatusDelta>>,
    tmp_counter: AtomicU64,
}

impl MaildirStatusIndex {
    fn new(root: PathBuf) -> Arc<Self> {
        let index = Arc::new(Self {
            root,
            entries: Mutex::new(HashMap::new()),
            tmp_counter: AtomicU64::new(0),
        });
        Self::spawn_flusher(&index);
        index
    }

    fn spawn_flusher(index: &Arc<Self>) {
        let weak = Arc::downgrade(index);
        std::thread::Builder::new()
            .name("edgerun-maildir-status-flush".to_string())
            .spawn(move || status_flush_loop(weak))
            .expect("failed to spawn Maildir status flusher");
    }

    fn user_maildir(&self, username: &str) -> PathBuf {
        self.root.join(username)
    }

    fn status_dir(&self, username: &str) -> PathBuf {
        self.user_maildir(username).join(".edgerun")
    }

    fn status_path(&self, username: &str) -> PathBuf {
        self.status_dir(username).join("status")
    }

    fn status_lock_path(&self, username: &str) -> PathBuf {
        self.status_dir(username).join("status.lock")
    }

    fn acquire_status_lock(&self, username: &str) -> io::Result<MaildirStatusLock> {
        let dir = self.status_dir(username);
        fs::create_dir_all(&dir)?;
        let path = self.status_lock_path(username);
        for _ in 0..5000 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(MaildirStatusLock { path }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "timed out waiting for Maildir status lock",
        ))
    }

    fn ensure<F>(&self, username: &str, load: F) -> io::Result<()>
    where
        F: FnOnce() -> io::Result<MailboxStatusCounters>,
    {
        if self.status_path(username).exists() {
            self.entries
                .lock()
                .unwrap()
                .entry(username.to_string())
                .or_default();
            return Ok(());
        }
        let _lock = self.acquire_status_lock(username)?;
        if !self.status_path(username).exists() {
            self.write_disk_status(username, load()?)?;
        }
        self.entries
            .lock()
            .unwrap()
            .entry(username.to_string())
            .or_default();
        Ok(())
    }

    fn read_effective(&self, username: &str) -> io::Result<Option<MailboxStatusCounters>> {
        let mut status = match self.read_disk_status(username)? {
            Some(status) => status,
            None => return Ok(None),
        };
        let pending = self
            .entries
            .lock()
            .unwrap()
            .get(username)
            .map(|entry| entry.pending_deliveries)
            .unwrap_or(0);
        status.messages = status.messages.saturating_add(pending);
        status.recent = status.recent.saturating_add(pending);
        status.uid_next = status.uid_next.saturating_add(pending);
        Ok(Some(status))
    }

    fn increment_delivery(&self, username: &str) -> io::Result<()> {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(username.to_string()).or_default();
        entry.pending_deliveries = entry.pending_deliveries.saturating_add(1);
        Ok(())
    }

    fn flush_all(&self) -> io::Result<()> {
        let users = {
            let entries = self.entries.lock().unwrap();
            entries
                .iter()
                .filter(|(_, entry)| entry.pending_deliveries > 0)
                .map(|(user, _)| user.clone())
                .collect::<Vec<_>>()
        };

        for user in users {
            self.flush_user(&user)?;
        }
        Ok(())
    }

    fn flush_user(&self, username: &str) -> io::Result<()> {
        let pending = {
            let mut entries = self.entries.lock().unwrap();
            let Some(entry) = entries.get_mut(username) else {
                return Ok(());
            };
            let pending = entry.pending_deliveries;
            entry.pending_deliveries = 0;
            pending
        };
        if pending == 0 {
            return Ok(());
        }

        let result = (|| {
            let _lock = self.acquire_status_lock(username)?;
            let mut status = self.read_disk_status(username)?.unwrap_or_default();
            status.messages = status.messages.saturating_add(pending);
            status.recent = status.recent.saturating_add(pending);
            status.uid_next = status.uid_next.saturating_add(pending);
            self.write_disk_status(username, status)
        })();

        if result.is_err() {
            let mut entries = self.entries.lock().unwrap();
            let entry = entries.entry(username.to_string()).or_default();
            entry.pending_deliveries = entry.pending_deliveries.saturating_add(pending);
        }

        result
    }

    fn read_disk_status(&self, username: &str) -> io::Result<Option<MailboxStatusCounters>> {
        let Ok(text) = fs::read_to_string(self.status_path(username)) else {
            return Ok(None);
        };
        let mut parts = text.split_whitespace();
        let Some(messages) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
            return Ok(None);
        };
        let Some(recent) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
            return Ok(None);
        };
        let Some(uid_next) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
            return Ok(None);
        };
        Ok(Some(MailboxStatusCounters {
            messages,
            recent,
            uid_next,
        }))
    }

    fn write_disk_status(&self, username: &str, status: MailboxStatusCounters) -> io::Result<()> {
        let dir = self.status_dir(username);
        fs::create_dir_all(&dir)?;
        let path = self.status_path(username);
        let tmp = dir.join(format!(
            "status.tmp.{}.{}",
            std::process::id(),
            self.tmp_counter.fetch_add(1, Ordering::Relaxed)
        ));
        {
            let mut file = fs::File::create(&tmp)?;
            writeln!(
                file,
                "{} {} {}",
                status.messages, status.recent, status.uid_next
            )?;
        }
        fs::rename(tmp, path)
    }
}

impl Drop for MaildirStatusIndex {
    fn drop(&mut self) {
        let _ = self.flush_all();
    }
}

struct MaildirStatusLock {
    path: PathBuf,
}

impl Drop for MaildirStatusLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn filename_host_token() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn status_flush_loop(index: Weak<MaildirStatusIndex>) {
    while let Some(index) = index.upgrade() {
        std::thread::sleep(Duration::from_millis(500));
        if let Err(e) = index.flush_all() {
            edgerun_log::warn!("edgerun-smtp: failed to flush Maildir status index: {}", e);
        }
    }
}

fn maildir_sync_delivery_enabled() -> bool {
    std::env::var("EDGERUN_MAILDIR_SYNC_DELIVERY")
        .ok()
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn auth_local_part(identity: &str) -> &str {
    identity
        .split_once('@')
        .map(|(local, _)| local)
        .unwrap_or(identity)
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
    fn test_smtp_auth_known_user() {
        let dir = test_dir("smtp_auth");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail"]).unwrap();
        store.set_user_token("ken", "secret").unwrap();

        let creds = AuthCredentials {
            authz_id: String::new(),
            authc_id: "ken@edgerun.mail".to_string(),
            token: "secret".to_string(),
        };
        assert!(matches!(
            store.authenticate("PLAIN", &creds),
            AuthResult::Authenticated(_)
        ));

        let wrong = AuthCredentials {
            authz_id: String::new(),
            authc_id: "ken".to_string(),
            token: "wrong".to_string(),
        };
        assert!(matches!(
            store.authenticate("PLAIN", &wrong),
            AuthResult::Failed
        ));

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
        store
            .add_user("ken", &["edgerun.mail", "localhost"])
            .unwrap();

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
        assert!(messages
            .iter()
            .all(|m| matches!(m.state, MaildirState::New)));

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
    fn test_mailbox_status_batches_delivery_deltas_until_flush() {
        let dir = test_dir("status_batch");
        let store = MaildirStore::new(&dir).unwrap();
        store.add_user("ken", &["edgerun.mail"]).unwrap();

        let mut envelope = MailEnvelope::new("sender@gmail.com".to_string());
        envelope.recipients.push("ken@edgerun.mail".to_string());
        envelope.data = b"From: sender@gmail.com\r\nTo: ken@edgerun.mail\r\n\r\nHello".to_vec();

        store.accept_mail(&envelope).unwrap();
        let status_path = dir.join("ken/.edgerun/status");
        assert_eq!(fs::read_to_string(&status_path).unwrap(), "0 0 1\n");

        let stats = store.mailbox_stats("ken").unwrap();
        assert_eq!(stats.new_count, 1);
        assert_eq!(stats.total_count, 1);

        store.flush_mailbox_statuses().unwrap();
        assert_eq!(fs::read_to_string(&status_path).unwrap(), "1 1 2\n");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_user() {
        let dir = test_dir("extract_user");
        let store = MaildirStore::new(&dir).unwrap();
        store
            .add_user("ken", &["edgerun.mail", "localhost"])
            .unwrap();
        store.add_user("admin", &["edgerun.mail"]).unwrap();

        assert_eq!(
            store.extract_user("ken@edgerun.mail"),
            Some("ken".to_string())
        );
        assert_eq!(
            store.extract_user("admin@edgerun.mail"),
            Some("admin".to_string())
        );
        assert_eq!(store.extract_user("unknown@edgerun.mail"), None);

        let _ = fs::remove_dir_all(&dir);
    }
}
