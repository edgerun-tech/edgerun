//! Maildir-backed IMAP MailStore implementation.
//!
//! Reads from the same Maildir directories that `edgerun_smtp::server::MaildirStore`
//! writes to. This enables the complete mail pipeline:
//!
//! ```text
//! SMTP (delivery) → MaildirStore → {root}/{user}/new/ → MaildirImapStore → IMAP (fetch)
//! ```

use crate::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

use crate::imap::message::StoreAction;
use crate::imap::server::MailStore;
use crate::imap::types::{FetchAttr, Flags, Mailbox, MailboxStatus, Message, SearchKey};

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
    /// Cached Maildir status keyed by user. Protected by a mutex so concurrent
    /// SELECT/STATUS calls share one directory scan per Maildir generation.
    status_cache: Arc<Mutex<HashMap<String, CachedMailboxStatus>>>,
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
            status_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Add a user with password.
    pub fn add_user(&self, username: &str, password: &str) {
        self.users
            .write()
            .unwrap()
            .insert(username.to_string(), password.to_string());
    }

    /// Check if a user exists.
    pub fn has_user(&self, username: &str) -> bool {
        self.users.read().unwrap().contains_key(username)
    }

    /// Get the path to a user's INBOX Maildir.
    fn inbox_path(&self, username: &str) -> PathBuf {
        self.root.join(username)
    }

    fn status_path(&self, username: &str) -> PathBuf {
        self.inbox_path(username).join(".edgerun").join("status")
    }

    fn status_lock_path(&self, username: &str) -> PathBuf {
        self.inbox_path(username)
            .join(".edgerun")
            .join("status.lock")
    }

    fn acquire_status_lock(&self, username: &str) -> io::Result<MaildirStatusLock> {
        let dir = self.inbox_path(username).join(".edgerun");
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

    fn read_indexed_mailbox_status(&self, user: &str) -> io::Result<Option<MailboxStatus>> {
        let Ok(text) = fs::read_to_string(self.status_path(user)) else {
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
        Ok(Some(MailboxStatus {
            messages,
            recent,
            uid_next,
            uid_validity: 1,
            uid_not_stored: 0,
        }))
    }

    fn write_indexed_mailbox_status(&self, user: &str, status: MailboxStatus) -> io::Result<()> {
        let dir = self.inbox_path(user).join(".edgerun");
        fs::create_dir_all(&dir)?;
        let ts = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp = dir.join(format!("status.tmp.{}.{}", std::process::id(), ts));
        {
            let mut file = fs::File::create(&tmp)?;
            writeln!(
                file,
                "{} {} {}",
                status.messages, status.recent, status.uid_next
            )?;
        }
        fs::rename(tmp, self.status_path(user))
    }

    fn update_indexed_mailbox_status<F>(&self, user: &str, update: F) -> io::Result<MailboxStatus>
    where
        F: FnOnce(&mut MailboxStatus),
    {
        let _lock = self.acquire_status_lock(user)?;
        let mut status = match self.read_indexed_mailbox_status(user)? {
            Some(status) => status,
            None => self.scan_mailbox_status_for_user(user)?,
        };
        update(&mut status);
        self.write_indexed_mailbox_status(user, status.clone())?;
        self.status_cache.lock().unwrap().remove(user);
        Ok(status)
    }

    fn ensure_indexed_mailbox_status(&self, user: &str) -> io::Result<()> {
        if self.status_path(user).exists() {
            return Ok(());
        }
        let _lock = self.acquire_status_lock(user)?;
        if self.status_path(user).exists() {
            return Ok(());
        }
        let status = self.scan_mailbox_status_for_user(user)?;
        self.write_indexed_mailbox_status(user, status)
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

    fn maildir_modified(&self, user: &str, subdir: &str) -> io::Result<SystemTime> {
        let dir = self.inbox_path(user).join(subdir);
        match fs::metadata(dir) {
            Ok(metadata) => metadata.modified(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(std::time::UNIX_EPOCH),
            Err(e) => Err(e),
        }
    }

    fn count_maildir_files(&self, user: &str, subdir: &str) -> io::Result<u32> {
        let dir = self.inbox_path(user).join(subdir);
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn mailbox_status_for_user(&self, user: &str) -> io::Result<MailboxStatus> {
        if let Some(status) = self.read_indexed_mailbox_status(user)? {
            return Ok(status);
        }

        self.scan_mailbox_status_for_user(user)
    }

    fn scan_mailbox_status_for_user(&self, user: &str) -> io::Result<MailboxStatus> {
        let new_modified = self.maildir_modified(user, "new")?;
        let cur_modified = self.maildir_modified(user, "cur")?;
        let mut cache = self.status_cache.lock().unwrap();

        if let Some(cached) = cache.get(user) {
            if cached.new_modified == new_modified && cached.cur_modified == cur_modified {
                return Ok(cached.status.clone());
            }
        }

        let new_count = self.count_maildir_files(user, "new")?;
        let cur_count = self.count_maildir_files(user, "cur")?;
        let count = new_count + cur_count;
        let status = MailboxStatus {
            messages: count,
            recent: new_count,
            uid_next: count + 1,
            uid_validity: 1,
            uid_not_stored: 0,
        };
        cache.insert(
            user.to_string(),
            CachedMailboxStatus {
                new_modified,
                cur_modified,
                status: status.clone(),
            },
        );
        Ok(status)
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
        let envelope = crate::imap::server::parse_envelope_from_rfc822(&data);

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

#[derive(Debug, Clone)]
struct CachedMailboxStatus {
    new_modified: SystemTime,
    cur_modified: SystemTime,
    status: MailboxStatus,
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

        Ok(Some(self.mailbox_status_for_user(user)?))
    }

    fn select(&self, mailbox: &str) -> io::Result<Option<Mailbox>> {
        let user = mailbox
            .strip_suffix("/INBOX")
            .or(mailbox.strip_suffix("INBOX"))
            .unwrap_or(mailbox)
            .trim_end_matches('/');

        let status = self.mailbox_status_for_user(user)?;

        Ok(Some(Mailbox {
            name: "INBOX".to_string(),
            delimiter: None,
            attributes: vec![],
            status: Some(status),
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
        self.ensure_indexed_mailbox_status(user)?;
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
                    attr_map.insert(
                        "ENVELOPE".to_string(),
                        crate::imap::server::format_envelope_imap(&msg.envelope),
                    );
                }
                FetchAttr::BodySection(_) => {
                    attr_map.insert("BODY[]".to_string(), format!("{{{}}}", msg.size));
                }
                FetchAttr::InternalDate => {
                    let date_str = crate::imap::server::format_internal_date(msg.internal_date);
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
        let (path, state) = &msgs[idx];

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
        if flags.iter().any(|f| f == "\\Seen")
            && matches!(action, StoreAction::Add | StoreAction::Replace)
            && path
                .parent()
                .map(|p| p.file_name().map(|n| n == "new").unwrap_or(false))
                .unwrap_or(false)
        {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let cur_path = self.inbox_path(user).join("cur").join(&filename);
            fs::rename(path, cur_path)?;
            if matches!(state, MessageState::New) {
                self.update_indexed_mailbox_status(user, |status| {
                    status.recent = status.recent.saturating_sub(1);
                })?;
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
                    SearchKey::Seen if !matches!(state, MessageState::Cur) => {
                        matches = false;
                    }
                    SearchKey::Unseen if !matches!(state, MessageState::New) => {
                        matches = false;
                    }
                    SearchKey::Deleted if !self.is_deleted(user, path) => {
                        matches = false;
                    }
                    SearchKey::Undeleted if self.is_deleted(user, path) => {
                        matches = false;
                    }
                    SearchKey::Subject(subj) => {
                        let data = fs::read(path)?;
                        let headers = Self::extract_headers(&data);
                        if !headers
                            .get("Subject")
                            .map(|s| s.contains(subj))
                            .unwrap_or(false)
                        {
                            matches = false;
                        }
                    }
                    SearchKey::From(from) => {
                        let data = fs::read(path)?;
                        let headers = Self::extract_headers(&data);
                        if !headers
                            .get("From")
                            .map(|s| s.contains(from))
                            .unwrap_or(false)
                        {
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

    fn sort(
        &self,
        mailbox: &str,
        keys: &[SearchKey],
        criteria: &[(String, bool)],
    ) -> io::Result<Vec<u32>> {
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
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "SUBJECT" => {
                    entries.sort_by(|a, b| {
                        let ord = a.subject.cmp(&b.subject);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "FROM" => {
                    entries.sort_by(|a, b| {
                        let ord = a.from.cmp(&b.from);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "TO" => {
                    entries.sort_by(|a, b| {
                        let ord = a.to.cmp(&b.to);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "CC" => {
                    // No CC data available, keep current order
                }
                "SIZE" => {
                    entries.sort_by(|a, b| {
                        let ord = a.size.cmp(&b.size);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
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
        self.ensure_indexed_mailbox_status(user)?;

        let mut removed = 0u32;
        let mut removed_recent = 0u32;
        for (i, (path, state)) in msgs.iter().enumerate() {
            if self.is_deleted(user, path) {
                let seq = (i + 1) as u32;
                let path_str = path.to_string_lossy().to_string();
                fs::remove_file(path)?;
                removed = removed.saturating_add(1);
                if matches!(state, MessageState::New) {
                    removed_recent = removed_recent.saturating_add(1);
                }

                // Remove from deleted list
                if let Some(paths) = self.deleted.write().unwrap().get_mut(user) {
                    paths.retain(|p| p != &path_str);
                }

                expunged.push(seq);
            }
        }

        if removed > 0 {
            self.update_indexed_mailbox_status(user, |status| {
                status.messages = status.messages.saturating_sub(removed);
                status.recent = status.recent.saturating_sub(removed_recent);
            })?;
        }

        Ok(expunged)
    }

    fn append(
        &self,
        mailbox: &str,
        _flags: Flags,
        _date: Option<SystemTime>,
        data: &[u8],
    ) -> io::Result<u32> {
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

        self.ensure_indexed_mailbox_status(user)?;
        fs::write(&tmp_path, data)?;
        fs::rename(&tmp_path, &new_path)?;

        let status = self.update_indexed_mailbox_status(user, |status| {
            status.messages = status.messages.saturating_add(1);
            status.recent = status.recent.saturating_add(1);
            status.uid_next = status.uid_next.saturating_add(1);
        })?;
        Ok(status.messages)
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

    fn get_quota(&self, _mailbox: &str) -> io::Result<Option<crate::imap::server::QuotaInfo>> {
        Ok(None)
    }

    fn set_quota(
        &self,
        _mailbox: &str,
        _limits: Vec<(&str, u32)>,
    ) -> io::Result<crate::imap::server::QuotaInfo> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "quota not supported",
        ))
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

    fn create_user_maildir(root: &Path, user: &str) {
        for subdir in ["tmp", "new", "cur", ".edgerun"] {
            fs::create_dir_all(root.join(user).join(subdir)).unwrap();
        }
    }

    fn read_status(root: &Path, user: &str) -> MailboxStatus {
        let text = fs::read_to_string(root.join(user).join(".edgerun").join("status")).unwrap();
        let mut parts = text.split_whitespace();
        MailboxStatus {
            messages: parts.next().unwrap().parse().unwrap(),
            recent: parts.next().unwrap().parse().unwrap(),
            uid_next: parts.next().unwrap().parse().unwrap(),
            uid_validity: 1,
            uid_not_stored: 0,
        }
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

        assert_eq!(
            store.authenticate("ken", "secret").unwrap(),
            Some("ken".to_string())
        );
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
        let data =
            b"From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: Test\r\n\r\nBody";
        let headers = MaildirImapStore::extract_headers(data);
        assert_eq!(headers.get("From").unwrap(), "sender@example.com");
        assert_eq!(headers.get("To").unwrap(), "recipient@example.com");
        assert_eq!(headers.get("Subject").unwrap(), "Test");
    }

    #[test]
    fn test_store_seen_updates_indexed_recent_count() {
        let dir = test_dir("store_seen_index");
        create_user_maildir(&dir, "ken");
        fs::write(dir.join("ken/new/msg1"), b"Subject: one\r\n\r\nbody").unwrap();
        fs::write(dir.join("ken/new/msg2"), b"Subject: two\r\n\r\nbody").unwrap();
        fs::write(dir.join("ken/.edgerun/status"), b"2 2 3\n").unwrap();

        let store = MaildirImapStore::new(&dir).unwrap();
        store
            .store("ken/INBOX", "1", &StoreAction::Add, &["\\Seen".to_string()])
            .unwrap();

        let status = read_status(&dir, "ken");
        assert_eq!(status.messages, 2);
        assert_eq!(status.recent, 1);
        assert_eq!(status.uid_next, 3);
        assert!(dir.join("ken/cur/msg1").exists());
        assert!(!dir.join("ken/new/msg1").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_append_updates_indexed_status() {
        let dir = test_dir("append_index");
        create_user_maildir(&dir, "ken");
        fs::write(dir.join("ken/.edgerun/status"), b"0 0 1\n").unwrap();

        let store = MaildirImapStore::new(&dir).unwrap();
        let messages = store
            .append(
                "ken/INBOX",
                Flags::default(),
                None,
                b"Subject: appended\r\n\r\nbody",
            )
            .unwrap();

        let status = read_status(&dir, "ken");
        assert_eq!(messages, 1);
        assert_eq!(status.messages, 1);
        assert_eq!(status.recent, 1);
        assert_eq!(status.uid_next, 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_append_creates_missing_index_without_double_counting() {
        let dir = test_dir("append_missing_index");
        create_user_maildir(&dir, "ken");

        let store = MaildirImapStore::new(&dir).unwrap();
        let messages = store
            .append(
                "ken/INBOX",
                Flags::default(),
                None,
                b"Subject: appended\r\n\r\nbody",
            )
            .unwrap();

        let status = read_status(&dir, "ken");
        assert_eq!(messages, 1);
        assert_eq!(status.messages, 1);
        assert_eq!(status.recent, 1);
        assert_eq!(status.uid_next, 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_expunge_updates_indexed_status() {
        let dir = test_dir("expunge_index");
        create_user_maildir(&dir, "ken");
        fs::write(dir.join("ken/new/msg1"), b"Subject: one\r\n\r\nbody").unwrap();
        fs::write(dir.join("ken/.edgerun/status"), b"1 1 2\n").unwrap();

        let store = MaildirImapStore::new(&dir).unwrap();
        store
            .store(
                "ken/INBOX",
                "1",
                &StoreAction::Add,
                &["\\Deleted".to_string()],
            )
            .unwrap();
        let expunged = store.expunge("ken/INBOX").unwrap();

        let status = read_status(&dir, "ken");
        assert_eq!(expunged, vec![1]);
        assert_eq!(status.messages, 0);
        assert_eq!(status.recent, 0);
        assert_eq!(status.uid_next, 2);
        assert!(!dir.join("ken/new/msg1").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
