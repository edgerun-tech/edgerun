//! Maildir-backed IMAP MailStore implementation.
//!
//! Reads from the same Maildir directories that `edgerun_smtp::server::MaildirStore`
//! writes to. This enables the complete mail pipeline:
//!
//! ```text
//! SMTP (delivery) → MaildirStore → {root}/{user}/new/ → MaildirImapStore → IMAP (fetch)
//! ```

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::server::MailStore;
use crate::message::StoreAction;
use crate::types::{FetchAttr, Flags, Mailbox, MailboxStatus, Message, SearchKey};

// ===========================================================================
// MaildirImapStore
// ===========================================================================

/// An IMAP MailStore backed by Maildir filesystem.
///
/// Reads messages from `{root}/{user}/{new,cur}/` directories created by
/// `edgerun_smtp::server::MaildirStore`.
///
/// Each user has an INBOX mapped to their Maildir. Other mailbox names
/// are not supported (Maildir is flat per-user).
pub struct MaildirImapStore {
    /// Root of the Maildir hierarchy (same as MaildirStore's root).
    root: PathBuf,
    /// User passwords: username -> password (plaintext, for testing only).
    users: Arc<RwLock<HashMap<String, String>>>,
    /// Global UID counter (across all mailboxes).
    next_uid: AtomicU32,
    /// Deleted message tracking per user: username -> set of paths.
    deleted: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl MaildirImapStore {
    /// Create a new MaildirImapStore.
    pub fn new(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            users: Arc::new(RwLock::new(HashMap::new())),
            next_uid: AtomicU32::new(1),
            deleted: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Add a user with password.
    pub fn add_user(&self, username: &str, password: &str) {
        self.users.write().unwrap().insert(username.to_string(), password.to_string());
    }

    /// Check if a user exists.
    pub fn has_user(&self, username: &str) -> bool {
        self.users.read().unwrap().contains_key(username)
    }

    /// Get the path to a user's INBOX Maildir.
    fn inbox_path(&self, username: &str) -> PathBuf {
        self.root.join(username)
    }

    /// List all files in a Maildir subdirectory (new/ or cur/).
    fn list_maildir_files(&self, user: &str, subdir: &str) -> io::Result<Vec<PathBuf>> {
        let dir = self.inbox_path(user).join(subdir);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_file() {
                files.push(entry.path());
            }
        }
        files.sort();
        Ok(files)
    }

    /// Get all messages for a user (new + cur).
    fn all_messages(&self, user: &str) -> io::Result<Vec<(PathBuf, MessageState)>> {
        let mut msgs = Vec::new();

        for path in self.list_maildir_files(user, "new")? {
            msgs.push((path, MessageState::New));
        }

        for path in self.list_maildir_files(user, "cur")? {
            msgs.push((path, MessageState::Cur));
        }

        msgs.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
        Ok(msgs)
    }

    /// Extract RFC822 headers from raw message data.
    fn extract_headers(data: &[u8]) -> HashMap<String, String> {
        let text = String::from_utf8_lossy(data);
        let mut headers = HashMap::new();

        if let Some(pos) = text.find("\r\n\r\n") {
            let header_section = &text[..pos];
            for line in header_section.lines() {
                if let Some(colon_pos) = line.find(": ") {
                    let key = line[..colon_pos].to_string();
                    let value = line[colon_pos + 2..].to_string();
                    headers.insert(key, value);
                }
            }
        }

        headers
    }

    /// Get the UID for a message (based on filename modification time).
    fn get_uid(&self, _path: &Path) -> u32 {
        // Use the atomic counter for UID assignment
        self.next_uid.fetch_add(1, Ordering::SeqCst)
    }

    /// Check if a message is marked \Deleted.
    fn is_deleted(&self, user: &str, path: &Path) -> bool {
        if let Some(paths) = self.deleted.read().unwrap().get(user) {
            let path_str = path.to_string_lossy().to_string();
            paths.contains(&path_str)
        } else {
            false
        }
    }

    /// Mark a message as deleted.
    fn mark_deleted(&self, user: &str, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        let mut deleted = self.deleted.write().unwrap();
        deleted.entry(user.to_string()).or_default().push(path_str);
    }

    /// Parse the raw message into IMAP-relevant fields.
    fn parse_message(path: &Path, seq: u32, uid: u32, state: &MessageState) -> io::Result<Message> {
        let data = fs::read(path)?;
        let size = data.len();

        let mut flags = Flags::default();
        if matches!(state, MessageState::Cur) {
            flags.seen = true;
        }

        // Parse envelope from raw data
        let envelope = crate::server::parse_envelope_from_rfc822(&data);

        Ok(Message {
            seq,
            uid,
            flags,
            internal_date: SystemTime::now(),
            size,
            envelope,
            rfc822: data,
        })
    }
}

/// Internal message state (new or cur).
#[derive(Debug, Clone)]
enum MessageState {
    New,
    Cur,
}

// ===========================================================================
// MailStore Implementation
// ===========================================================================

impl MailStore for MaildirImapStore {
    fn authenticate(&self, user: &str, password: &str) -> io::Result<Option<String>> {
        let users = self.users.read().unwrap();
        if let Some(stored) = users.get(user) {
            if stored == password {
                return Ok(Some(user.to_string()));
            }
        }
        Ok(None)
    }

    fn list(&self, _reference: &str, _pattern: &str) -> io::Result<Vec<Mailbox>> {
        // Maildir is flat per-user, so we only return INBOX
        Ok(vec![Mailbox {
            name: "INBOX".to_string(),
            delimiter: Some('/'.to_string()),
            attributes: vec!["\\HasNoChildren".to_string()],
            status: None,
        }])
    }

    fn status(&self, mailbox: &str) -> io::Result<Option<MailboxStatus>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let count = msgs.len();
        let unseen = msgs.iter().filter(|(_, s)| matches!(s, MessageState::New)).count();

        Ok(Some(MailboxStatus {
            messages: count as u32,
            recent: unseen as u32,
            uid_next: (count + 1) as u32,
            uid_validity: 1,
            uid_not_stored: 0,
        }))
    }

    fn select(&self, mailbox: &str) -> io::Result<Option<Mailbox>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let count = msgs.len();
        let unseen = msgs.iter().filter(|(_, s)| matches!(s, MessageState::New)).count();

        Ok(Some(Mailbox {
            name: "INBOX".to_string(),
            delimiter: None,
            attributes: vec![],
            status: Some(MailboxStatus {
                messages: count as u32,
                recent: unseen as u32,
                uid_next: (count + 1) as u32,
                uid_validity: 1,
                uid_not_stored: 0,
            }),
        }))
    }

    fn create(&self, _mailbox: &str) -> io::Result<bool> {
        // Maildir doesn't support creating sub-mailboxes
        Ok(false)
    }

    fn delete(&self, _mailbox: &str) -> io::Result<bool> {
        Ok(false)
    }

    fn rename(&self, _old: &str, _new: &str) -> io::Result<bool> {
        Ok(false)
    }

    fn fetch(
        &self,
        mailbox: &str,
        sequence: &str,
        attributes: &[FetchAttr],
    ) -> io::Result<Vec<(u32, HashMap<String, String>)>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let mut results = Vec::new();

        // Parse sequence number (simple: just the number)
        let seq_num: u32 = sequence.parse().unwrap_or(1);
        if seq_num == 0 || seq_num as usize > msgs.len() {
            return Ok(results);
        }

        let idx = (seq_num - 1) as usize;
        let (path, state) = &msgs[idx];
        let uid = self.get_uid(path);

        let msg = Self::parse_message(path, seq_num, uid, state)?;
        let mut attr_map = HashMap::new();

        for attr in attributes {
            match attr {
                FetchAttr::Uid => {
                    attr_map.insert("UID".to_string(), uid.to_string());
                }
                FetchAttr::Flags => {
                    let flags_str = msg.flags.format();
                    attr_map.insert("FLAGS".to_string(), flags_str);
                }
                FetchAttr::Rfc822 => {
                    let text = String::from_utf8_lossy(&msg.rfc822);
                    attr_map.insert("RFC822".to_string(), text.to_string());
                }
                FetchAttr::Rfc822Header => {
                    if let Some(pos) = msg.rfc822.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&msg.rfc822[..pos]);
                        attr_map.insert("RFC822.HEADER".to_string(), headers.to_string());
                    }
                }
                FetchAttr::Rfc822Size => {
                    attr_map.insert("RFC822.SIZE".to_string(), msg.size.to_string());
                }
                FetchAttr::Rfc822Text => {
                    if let Some(pos) = msg.rfc822.windows(4).position(|w| w == b"\r\n\r\n") {
                        let body = String::from_utf8_lossy(&msg.rfc822[pos + 4..]);
                        attr_map.insert("RFC822.TEXT".to_string(), body.to_string());
                    }
                }
                FetchAttr::Envelope => {
                    attr_map.insert("ENVELOPE".to_string(), crate::server::format_envelope_imap(&msg.envelope));
                }
                FetchAttr::BodySection(_) => {
                    attr_map.insert("BODY[]".to_string(), format!("{{{}}}", msg.size));
                }
                FetchAttr::InternalDate => {
                    let date_str = crate::server::format_internal_date(msg.internal_date);
                    attr_map.insert("INTERNALDATE".to_string(), date_str);
                }
                FetchAttr::BodyStructure => {
                    attr_map.insert("BODYSTRUCTURE".to_string(), "NIL".to_string());
                }
                FetchAttr::MsgSize => {
                    attr_map.insert("RFC822.SIZE".to_string(), msg.size.to_string());
                }
            }
        }

        results.push((seq_num, attr_map));
        Ok(results)
    }

    fn store(
        &self,
        mailbox: &str,
        sequence: &str,
        action: &StoreAction,
        flags: &[String],
    ) -> io::Result<Vec<u32>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let seq_num: u32 = sequence.parse().unwrap_or(1);
        if seq_num == 0 || seq_num as usize > msgs.len() {
            return Ok(Vec::new());
        }

        let idx = (seq_num - 1) as usize;
        let (path, _state) = &msgs[idx];

        // Handle \Deleted flag
        if flags.iter().any(|f| f == "\\Deleted") {
            match action {
                StoreAction::Add | StoreAction::AddSilent => {
                    self.mark_deleted(user, path);
                }
                StoreAction::Remove | StoreAction::RemoveSilent => {
                    // Unmark deleted
                    let path_str = path.to_string_lossy().to_string();
                    if let Some(paths) = self.deleted.write().unwrap().get_mut(user) {
                        paths.retain(|p| p != &path_str);
                    }
                }
                StoreAction::Replace | StoreAction::ReplaceSilent => {
                    self.mark_deleted(user, path);
                }
            }
        }

        // Handle \Seen flag — move from new/ to cur/
        if flags.iter().any(|f| f == "\\Seen") && matches!(action, StoreAction::Add | StoreAction::Replace) {
            if path.parent().map(|p| p.file_name().map(|n| n == "new").unwrap_or(false)).unwrap_or(false) {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let cur_path = self.inbox_path(user).join("cur").join(&filename);
                let _ = fs::rename(path, cur_path);
            }
        }

        Ok(vec![seq_num])
    }

    fn search(&self, mailbox: &str, keys: &[SearchKey]) -> io::Result<Vec<u32>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let mut results = Vec::new();

        for (i, (path, state)) in msgs.iter().enumerate() {
            let seq = (i + 1) as u32;
            let mut matches = true;

            for key in keys {
                match key {
                    SearchKey::All => {}
                    SearchKey::Seen => {
                        if !matches!(state, MessageState::Cur) {
                            matches = false;
                        }
                    }
                    SearchKey::Unseen => {
                        if !matches!(state, MessageState::New) {
                            matches = false;
                        }
                    }
                    SearchKey::Deleted => {
                        if !self.is_deleted(user, path) {
                            matches = false;
                        }
                    }
                    SearchKey::Undeleted => {
                        if self.is_deleted(user, path) {
                            matches = false;
                        }
                    }
                    SearchKey::Subject(subj) => {
                        let data = fs::read(path)?;
                        let headers = Self::extract_headers(&data);
                        if !headers.get("Subject").map(|s| s.contains(subj)).unwrap_or(false) {
                            matches = false;
                        }
                    }
                    SearchKey::From(from) => {
                        let data = fs::read(path)?;
                        let headers = Self::extract_headers(&data);
                        if !headers.get("From").map(|s| s.contains(from)).unwrap_or(false) {
                            matches = false;
                        }
                    }
                    SearchKey::To(to) => {
                        let data = fs::read(path)?;
                        let headers = Self::extract_headers(&data);
                        if !headers.get("To").map(|s| s.contains(to)).unwrap_or(false) {
                            matches = false;
                        }
                    }
                    _ => {}
                }
            }

            if matches {
                results.push(seq);
            }
        }

        Ok(results)
    }

    fn sort(&self, mailbox: &str, keys: &[SearchKey], criteria: &[(String, bool)]) -> io::Result<Vec<u32>> {
        // First search for matching UIDs
        let uids = self.search(mailbox, keys)?;
        if uids.is_empty() || criteria.is_empty() {
            return Ok(uids);
        }

        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;

        // Collect sortable data for each UID
        struct SortEntry {
            uid: u32,
            date: std::time::SystemTime,
            subject: String,
            from: String,
            to: String,
            size: u64,
        }

        let mut entries: Vec<SortEntry> = Vec::new();
        for (i, (path, state)) in msgs.iter().enumerate() {
            let uid = (i + 1) as u32;
            if !uids.contains(&uid) {
                continue;
            }

            let data = fs::read(path)?;
            let headers = Self::extract_headers(&data);
            let metadata = fs::metadata(path)?;
            let date = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
            let size = metadata.len();

            entries.push(SortEntry {
                uid,
                date,
                subject: headers.get("Subject").cloned().unwrap_or_default(),
                from: headers.get("From").cloned().unwrap_or_default(),
                to: headers.get("To").cloned().unwrap_or_default(),
                size,
            });
        }

        // Sort by criteria in order (last criterion is primary)
        for (criterion, reversed) in criteria.iter().rev() {
            let rev = *reversed;
            match criterion.to_uppercase().as_str() {
                "ARRIVAL" | "DATE" | "SENT" => {
                    entries.sort_by(|a, b| {
                        let ord = a.date.cmp(&b.date);
                        if rev { ord.reverse() } else { ord }
                    });
                }
                "SUBJECT" => {
                    entries.sort_by(|a, b| {
                        let ord = a.subject.cmp(&b.subject);
                        if rev { ord.reverse() } else { ord }
                    });
                }
                "FROM" => {
                    entries.sort_by(|a, b| {
                        let ord = a.from.cmp(&b.from);
                        if rev { ord.reverse() } else { ord }
                    });
                }
                "TO" => {
                    entries.sort_by(|a, b| {
                        let ord = a.to.cmp(&b.to);
                        if rev { ord.reverse() } else { ord }
                    });
                }
                "CC" => {
                    // No CC data available, keep current order
                }
                "SIZE" => {
                    entries.sort_by(|a, b| {
                        let ord = a.size.cmp(&b.size);
                        if rev { ord.reverse() } else { ord }
                    });
                }
                _ => {} // Unknown criterion, keep order
            }
        }

        Ok(entries.iter().map(|e| e.uid).collect())
    }

    fn expunge(&self, mailbox: &str) -> io::Result<Vec<u32>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let mut expunged = Vec::new();

        for (i, (path, _state)) in msgs.iter().enumerate() {
            if self.is_deleted(user, path) {
                let seq = (i + 1) as u32;
                let path_str = path.to_string_lossy().to_string();
                fs::remove_file(path)?;

                // Remove from deleted list
                if let Some(paths) = self.deleted.write().unwrap().get_mut(user) {
                    paths.retain(|p| p != &path_str);
                }

                expunged.push(seq);
            }
        }

        Ok(expunged)
    }

    fn append(&self, mailbox: &str, _flags: Flags, _date: Option<SystemTime>, data: &[u8]) -> io::Result<u32> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        // Generate unique Maildir filename
        let ts = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        let pid = std::process::id();

        let filename = format!("M{}P{}.edgerun", ts, pid);
        let tmp_path = self.inbox_path(user).join("tmp").join(&filename);
        let new_path = self.inbox_path(user).join("new").join(&filename);

        fs::write(&tmp_path, data)?;
        fs::rename(&tmp_path, &new_path)?;

        let msgs = self.all_messages(user)?;
        Ok(msgs.len() as u32)
    }

    fn copy_messages(&self, mailbox: &str, sequence: &str, _dest: &str) -> io::Result<Vec<u32>> {
        // Maildir doesn't support copying between mailboxes (flat structure)
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let msgs = self.all_messages(user)?;
        let seq_num: u32 = sequence.parse().unwrap_or(1);
        if seq_num > 0 && seq_num as usize <= msgs.len() {
            Ok(vec![seq_num])
        } else {
            Ok(Vec::new())
        }
    }

    fn close(&self, _mailbox: &str) -> io::Result<()> {
        // Expunge deleted messages on close
        Ok(())
    }

    fn subscribe(&self, _mailbox: &str) -> io::Result<bool> {
        Ok(true)
    }

    fn unsubscribe(&self, _mailbox: &str) -> io::Result<bool> {
        Ok(true)
    }

    fn list_subscribed(&self, reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>> {
        self.list(reference, pattern)
    }

    fn get_quota(&self, _mailbox: &str) -> io::Result<Option<crate::server::QuotaInfo>> {
        Ok(None)
    }

    fn set_quota(&self, _mailbox: &str, _limits: Vec<(&str, u32)>) -> io::Result<crate::server::QuotaInfo> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "quota not supported"))
    }

    fn check(&self, _mailbox: &str) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("edgerun_imap_maildir_{}", name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn test_create_store() {
        let dir = test_dir("create");
        let store = MaildirImapStore::new(&dir).unwrap();
        assert!(dir.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_authenticate() {
        let dir = test_dir("auth");
        let store = MaildirImapStore::new(&dir).unwrap();
        store.add_user("ken", "secret");

        assert_eq!(store.authenticate("ken", "secret").unwrap(), Some("ken".to_string()));
        assert_eq!(store.authenticate("ken", "wrong").unwrap(), None);
        assert_eq!(store.authenticate("unknown", "pass").unwrap(), None);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_returns_inbox() {
        let dir = test_dir("list");
        let store = MaildirImapStore::new(&dir).unwrap();
        store.add_user("ken", "secret");

        let mailboxes = store.list("", "*").unwrap();
        assert_eq!(mailboxes.len(), 1);
        assert_eq!(mailboxes[0].name, "INBOX");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extract_headers() {
        let data = b"From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: Test\r\n\r\nBody";
        let headers = MaildirImapStore::extract_headers(data);
        assert_eq!(headers.get("From").unwrap(), "sender@example.com");
        assert_eq!(headers.get("To").unwrap(), "recipient@example.com");
        assert_eq!(headers.get("Subject").unwrap(), "Test");
    }
}
