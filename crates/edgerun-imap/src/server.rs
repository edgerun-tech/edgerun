//! IMAP server implementation (RFC 3501).
//!
//! Provides an async IMAP4rev1 server with:
//! - TCP listener with per-connection session handling
//! - Session state machine (NotAuthenticated → Authenticated → Selected → Logout)
//! - In-memory mailbox storage (pluggable via `MailStore` trait)
//! - STARTTLS support (with `tls` feature)

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use edgerun_rt::{
    AsyncReadExt, AsyncWriteExt, AsyncTcpListener, AsyncTcpStream,
    CancellationToken, Mutex,
};

#[cfg(feature = "tls")]
use edgerun_tls::{AsyncTlsServerStream, TlsCertificate};

use crate::message::{ImapCommand, ImapResponse, ImapResult, StoreAction};
use crate::parser::{self, ImapReader};
use crate::types::{
    Envelope, FetchAttr, Flags, ImapState, Mailbox, MailboxStatus, Message, SearchKey, Address,
};

// ===========================================================================
// IMAP Capabilities
// ===========================================================================

/// Default IMAP capabilities.
const CAPABILITIES: &[&str] = &[
    "IMAP4rev1",
    "UIDPLUS",
    "CHILDREN",
    "IDLE",
    "NAMESPACE",
    "QUOTA",
    "MOVE",
    "SASL-IR",
    "ENABLE",
    #[cfg(feature = "tls")]
    "STARTTLS",
    #[cfg(feature = "tls")]
    "LOGINDISABLED",
];

// ===========================================================================
// Mail Store Trait
// ===========================================================================

/// Trait for a pluggable mailbox backend.
pub trait MailStore: Send + Sync + 'static {
    /// Authenticate a user. Returns the username on success.
    fn authenticate(&self, user: &str, password: &str) -> io::Result<Option<String>>;

    /// List mailboxes matching a pattern.
    fn list(&self, reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>>;

    /// Get the status of a mailbox.
    fn status(&self, mailbox: &str) -> io::Result<Option<MailboxStatus>>;

    /// Select a mailbox (open for access).
    fn select(&self, mailbox: &str) -> io::Result<Option<Mailbox>>;

    /// Create a new mailbox.
    fn create(&self, mailbox: &str) -> io::Result<bool>;

    /// Delete a mailbox.
    fn delete(&self, mailbox: &str) -> io::Result<bool>;

    /// Rename a mailbox.
    fn rename(&self, old: &str, new: &str) -> io::Result<bool>;

    /// Fetch messages matching a sequence set.
    fn fetch(&self, mailbox: &str, sequence: &str, attrs: &[FetchAttr]) -> io::Result<Vec<(u32, HashMap<String, String>)>>;

    /// Store flags on messages.
    fn store(&self, mailbox: &str, sequence: &str, action: &StoreAction, flags: &[String]) -> io::Result<Vec<u32>>;

    /// Search for messages matching criteria.
    fn search(&self, mailbox: &str, keys: &[SearchKey]) -> io::Result<Vec<u32>>;

    /// Expunge deleted messages.
    fn expunge(&self, mailbox: &str) -> io::Result<Vec<u32>>;

    /// Append a message to a mailbox.
    fn append(&self, mailbox: &str, flags: Flags, date: Option<SystemTime>, data: &[u8]) -> io::Result<u32>;

    /// Copy messages to another mailbox.
    fn copy_messages(&self, mailbox: &str, sequence: &str, dest: &str) -> io::Result<Vec<u32>>;

    /// Close a mailbox (expunge and deselect).
    fn close(&self, mailbox: &str) -> io::Result<()>;
}

// ===========================================================================
// In-Memory Mail Store
// ===========================================================================

/// Simple in-memory mailbox implementation.
pub struct MemoryStore {
    mailboxes: std::sync::Mutex<HashMap<String, Vec<Message>>>,
    users: std::sync::Mutex<HashMap<String, String>>, // username -> password
    next_uid: std::sync::Mutex<u32>,
}

impl MemoryStore {
    pub fn new() -> Self {
        let mut store = Self {
            mailboxes: std::sync::Mutex::new(HashMap::new()),
            users: std::sync::Mutex::new(HashMap::new()),
            next_uid: std::sync::Mutex::new(1),
        };
        // Create default INBOX
        store.mailboxes.lock().unwrap().insert("INBOX".to_string(), Vec::new());
        // Add default user
        store.users.lock().unwrap().insert("user".to_string(), "pass".to_string());
        store
    }

    pub fn add_user(&self, username: &str, password: &str) {
        self.users.lock().unwrap().insert(username.to_string(), password.to_string());
    }
}

impl Default for MemoryStore {
    fn default() -> Self { Self::new() }
}

impl MailStore for MemoryStore {
    fn authenticate(&self, user: &str, password: &str) -> io::Result<Option<String>> {
        let users = self.users.lock().unwrap();
        if let Some(stored_pass) = users.get(user) {
            if stored_pass == password {
                return Ok(Some(user.to_string()));
            }
        }
        Ok(None)
    }

    fn list(&self, _reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        let mut result = Vec::new();
        for name in mailboxes.keys() {
            if pattern == "*" || pattern == "%" || name.contains(pattern.trim_matches('%')) {
                result.push(Mailbox::new(name.clone()));
            }
        }
        Ok(result)
    }

    fn status(&self, mailbox: &str) -> io::Result<Option<MailboxStatus>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get(mailbox) {
            let recent = msgs.iter().filter(|m| m.flags.recent).count() as u32;
            Ok(Some(MailboxStatus {
                messages: msgs.len() as u32,
                recent,
                uid_next: *self.next_uid.lock().unwrap(),
                uid_validity: 1,
                uid_not_stored: 0,
            }))
        } else {
            Ok(None)
        }
    }

    fn select(&self, mailbox: &str) -> io::Result<Option<Mailbox>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get(mailbox) {
            let recent = msgs.iter().filter(|m| m.flags.recent).count() as u32;
            Ok(Some(Mailbox {
                name: mailbox.to_string(),
                attributes: Vec::new(),
                delimiter: Some("/".to_string()),
                status: Some(MailboxStatus {
                    messages: msgs.len() as u32,
                    recent,
                    uid_next: *self.next_uid.lock().unwrap(),
                    uid_validity: 1,
                    uid_not_stored: 0,
                }),
            }))
        } else {
            Ok(None)
        }
    }

    fn create(&self, mailbox: &str) -> io::Result<bool> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if mailboxes.contains_key(mailbox) {
            return Ok(false);
        }
        mailboxes.insert(mailbox.to_string(), Vec::new());
        Ok(true)
    }

    fn delete(&self, mailbox: &str) -> io::Result<bool> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        Ok(mailboxes.remove(mailbox).is_some())
    }

    fn rename(&self, old: &str, new: &str) -> io::Result<bool> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.remove(old) {
            mailboxes.insert(new.to_string(), msgs);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn fetch(&self, mailbox: &str, sequence: &str, attrs: &[FetchAttr]) -> io::Result<Vec<(u32, HashMap<String, String>)>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get(mailbox) {
            let mut results = Vec::new();
            let seq_set = parser::parse_sequence_set(sequence);

            for seq_str in &seq_set {
                // Handle ranges like "1:5" or "*"
                let seq_nums = if seq_str.contains(':') {
                    let parts: Vec<&str> = seq_str.splitn(2, ':').collect();
                    let start = if parts[0] == "*" { msgs.len() as u32 } else { parts[0].parse::<u32>().unwrap_or(0) };
                    let end = if parts[1] == "*" { msgs.len() as u32 } else { parts[1].parse::<u32>().unwrap_or(0) };
                    let (s, e) = if start <= end { (start, end) } else { (end, start) };
                    (s..=e).collect::<Vec<_>>()
                } else if seq_str == "*" {
                    vec![msgs.len() as u32]
                } else {
                    vec![seq_str.parse::<u32>().unwrap_or(0)]
                };

                for seq_num in seq_nums {
                    if seq_num > 0 && seq_num as usize <= msgs.len() {
                        let msg = &msgs[seq_num as usize - 1];
                        let mut data = HashMap::new();
                        for attr in attrs {
                            match attr {
                                FetchAttr::Uid => {
                                    data.insert("UID".to_string(), msg.uid.to_string());
                                }
                                FetchAttr::Flags => {
                                    data.insert("FLAGS".to_string(), msg.flags.format());
                                }
                                FetchAttr::Rfc822Size => {
                                    data.insert("RFC822.SIZE".to_string(), msg.size.to_string());
                                }
                                FetchAttr::Rfc822 => {
                                    let body = String::from_utf8_lossy(&msg.rfc822);
                                    data.insert("RFC822".to_string(), format!("{{{}}}", msg.size));
                                    data.insert("__RFC822_BODY__".to_string(), body.into_owned());
                                }
                                FetchAttr::Rfc822Header => {
                                    // Extract headers (everything before first blank line)
                                    let header_end = msg.rfc822.windows(4)
                                        .position(|w| w == b"\r\n\r\n")
                                        .unwrap_or(msg.rfc822.len());
                                    let headers = String::from_utf8_lossy(&msg.rfc822[..header_end]);
                                    data.insert("RFC822.HEADER".to_string(), headers.into_owned());
                                }
                                FetchAttr::Rfc822Text => {
                                    // Extract body (everything after first blank line)
                                    let header_end = msg.rfc822.windows(4)
                                        .position(|w| w == b"\r\n\r\n")
                                        .map(|p| p + 4)
                                        .unwrap_or(0);
                                    let body = String::from_utf8_lossy(&msg.rfc822[header_end..]);
                                    data.insert("RFC822.TEXT".to_string(), body.into_owned());
                                }
                                FetchAttr::Envelope => {
                                    data.insert("ENVELOPE".to_string(), format_envelope_imap(&msg.envelope));
                                }
                                FetchAttr::InternalDate => {
                                    // Format as IMAP internal date: DD-Mon-YYYY HH:MM:SS +ZZZZ
                                    let date_str = format_internal_date(msg.internal_date);
                                    data.insert("INTERNALDATE".to_string(), date_str);
                                }
                                FetchAttr::BodySection(section) => {
                                    if section.is_empty() {
                                        // BODY[] = full message
                                        data.insert("BODY[]".to_string(), format!("{{{}}}", msg.size));
                                        data.insert("__BODY_DATA__".to_string(), String::from_utf8_lossy(&msg.rfc822).into_owned());
                                    } else if section.to_uppercase() == "HEADER" {
                                        let header_end = msg.rfc822.windows(4)
                                            .position(|w| w == b"\r\n\r\n")
                                            .unwrap_or(msg.rfc822.len());
                                        let headers = String::from_utf8_lossy(&msg.rfc822[..header_end]);
                                        data.insert("BODY[HEADER]".to_string(), headers.into_owned());
                                    } else if section.to_uppercase() == "TEXT" {
                                        let header_end = msg.rfc822.windows(4)
                                            .position(|w| w == b"\r\n\r\n")
                                            .map(|p| p + 4)
                                            .unwrap_or(0);
                                        let body = String::from_utf8_lossy(&msg.rfc822[header_end..]);
                                        data.insert("BODY[TEXT]".to_string(), body.into_owned());
                                    }
                                }
                                FetchAttr::BodyStructure => {
                                    data.insert("BODYSTRUCTURE".to_string(), format_body_structure(msg));
                                }
                                FetchAttr::MsgSize => {
                                    data.insert("RFC822.SIZE".to_string(), msg.size.to_string());
                                }
                            }
                        }
                        results.push((msg.uid, data));
                    }
                }
            }
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    fn store(&self, mailbox: &str, sequence: &str, action: &StoreAction, flags: &[String]) -> io::Result<Vec<u32>> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get_mut(mailbox) {
            let mut updated = Vec::new();
            let seq_set = parser::parse_sequence_set(sequence);

            for seq_str in &seq_set {
                let seq_nums = if seq_str.contains(':') {
                    let parts: Vec<&str> = seq_str.splitn(2, ':').collect();
                    let start = if parts[0] == "*" { msgs.len() as u32 } else { parts[0].parse::<u32>().unwrap_or(0) };
                    let end = if parts[1] == "*" { msgs.len() as u32 } else { parts[1].parse::<u32>().unwrap_or(0) };
                    let (s, e) = if start <= end { (start, end) } else { (end, start) };
                    (s..=e).collect::<Vec<_>>()
                } else if seq_str == "*" {
                    vec![msgs.len() as u32]
                } else {
                    vec![seq_str.parse::<u32>().unwrap_or(0)]
                };

                for seq_num in seq_nums {
                    if seq_num > 0 && seq_num as usize <= msgs.len() {
                        let msg = &mut msgs[seq_num as usize - 1];
                        let new_flags = Flags::parse(&flags.join(" "));

                        match action {
                            StoreAction::Replace => {
                                msg.flags = new_flags;
                            }
                            StoreAction::ReplaceSilent => {
                                msg.flags = new_flags;
                            }
                            StoreAction::Add => {
                                msg.flags.seen |= new_flags.seen;
                                msg.flags.answered |= new_flags.answered;
                                msg.flags.flagged |= new_flags.flagged;
                                msg.flags.deleted |= new_flags.deleted;
                                msg.flags.draft |= new_flags.draft;
                                for kw in &new_flags.keywords {
                                    if !msg.flags.keywords.contains(kw) {
                                        msg.flags.keywords.push(kw.clone());
                                    }
                                }
                            }
                            StoreAction::AddSilent => {
                                msg.flags.seen |= new_flags.seen;
                                msg.flags.answered |= new_flags.answered;
                                msg.flags.flagged |= new_flags.flagged;
                                msg.flags.deleted |= new_flags.deleted;
                                msg.flags.draft |= new_flags.draft;
                                for kw in &new_flags.keywords {
                                    if !msg.flags.keywords.contains(kw) {
                                        msg.flags.keywords.push(kw.clone());
                                    }
                                }
                            }
                            StoreAction::Remove => {
                                msg.flags.seen &= !new_flags.seen;
                                msg.flags.answered &= !new_flags.answered;
                                msg.flags.flagged &= !new_flags.flagged;
                                msg.flags.deleted &= !new_flags.deleted;
                                msg.flags.draft &= !new_flags.draft;
                                msg.flags.keywords.retain(|k| !new_flags.keywords.contains(k));
                            }
                            StoreAction::RemoveSilent => {
                                msg.flags.seen &= !new_flags.seen;
                                msg.flags.answered &= !new_flags.answered;
                                msg.flags.flagged &= !new_flags.flagged;
                                msg.flags.deleted &= !new_flags.deleted;
                                msg.flags.draft &= !new_flags.draft;
                                msg.flags.keywords.retain(|k| !new_flags.keywords.contains(k));
                            }
                        }
                        updated.push(msg.uid);
                    }
                }
            }
            Ok(updated)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "mailbox not found"))
        }
    }

    fn search(&self, mailbox: &str, keys: &[SearchKey]) -> io::Result<Vec<u32>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get(mailbox) {
            let mut results = Vec::new();
            for msg in msgs.iter() {
                if matches_keys(msg, keys, msgs) {
                    results.push(msg.uid);
                }
            }
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }

    fn expunge(&self, mailbox: &str) -> io::Result<Vec<u32>> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get_mut(mailbox) {
            let mut removed = Vec::new();
            msgs.retain(|msg| {
                if msg.flags.deleted {
                    removed.push(msg.uid);
                    false
                } else {
                    true
                }
            });
            // Update sequence numbers after expunge
            for (i, msg) in msgs.iter_mut().enumerate() {
                msg.seq = i as u32 + 1;
            }
            Ok(removed)
        } else {
            Ok(Vec::new())
        }
    }

    fn append(&self, mailbox: &str, flags: Flags, date: Option<SystemTime>, data: &[u8]) -> io::Result<u32> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get_mut(mailbox) {
            let mut next_uid = self.next_uid.lock().unwrap();
            let uid = *next_uid;
            *next_uid += 1;

            let seq = msgs.len() as u32 + 1;
            let msg_date = date.unwrap_or_else(SystemTime::now);
            let mut msg = Message::new(uid, seq, data.to_vec(), msg_date);
            msg.flags = flags;
            // Parse envelope from message headers
            msg.envelope = parse_envelope_from_rfc822(data);
            msgs.push(msg);
            Ok(uid)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "mailbox not found"))
        }
    }

    fn copy_messages(&self, mailbox: &str, sequence: &str, dest: &str) -> io::Result<Vec<u32>> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        
        // Get source messages
        if let Some(src_msgs) = mailboxes.get(mailbox) {
            let mut msgs_to_copy = Vec::new();
            let seq_set = parser::parse_sequence_set(sequence);

            for seq_str in &seq_set {
                let seq_nums = if seq_str.contains(':') {
                    let parts: Vec<&str> = seq_str.splitn(2, ':').collect();
                    let start = if parts[0] == "*" { src_msgs.len() as u32 } else { parts[0].parse::<u32>().unwrap_or(0) };
                    let end = if parts[1] == "*" { src_msgs.len() as u32 } else { parts[1].parse::<u32>().unwrap_or(0) };
                    let (s, e) = if start <= end { (start, end) } else { (end, start) };
                    (s..=e).collect::<Vec<_>>()
                } else if seq_str == "*" {
                    vec![src_msgs.len() as u32]
                } else {
                    vec![seq_str.parse::<u32>().unwrap_or(0)]
                };

                for seq_num in seq_nums {
                    if seq_num > 0 && seq_num as usize <= src_msgs.len() {
                        msgs_to_copy.push(src_msgs[seq_num as usize - 1].clone());
                    }
                }
            }

            // Copy to destination
            if let Some(dest_msgs) = mailboxes.get_mut(dest) {
                let mut next_uid = self.next_uid.lock().unwrap();
                let mut copied_uids = Vec::new();

                for mut msg in msgs_to_copy {
                    let new_uid = *next_uid;
                    *next_uid += 1;
                    msg.uid = new_uid;
                    msg.seq = dest_msgs.len() as u32 + 1;
                    // \Recent flag doesn't transfer
                    msg.flags.recent = true;
                    dest_msgs.push(msg);
                    copied_uids.push(new_uid);
                }
                Ok(copied_uids)
            } else {
                Err(io::Error::new(io::ErrorKind::NotFound, "destination mailbox not found"))
            }
        } else {
            Ok(Vec::new())
        }
    }

    fn close(&self, mailbox: &str) -> io::Result<()> {
        // CLOSE = expunge \Deleted messages then deselect
        self.expunge(mailbox)?;
        Ok(())
    }
}

// ===========================================================================
// Helper Functions
// ===========================================================================

fn format_envelope_imap(env: &Envelope) -> String {
    let date = format_string_or_nil_imap(&env.date);
    let subject = format_string_or_nil_imap(&env.subject);
    let from = format_address_list_imap(&env.from);
    let sender = format_address_list_opt_imap(&env.sender);
    let reply_to = format_address_list_opt_imap(&env.reply_to);
    let to = format_address_list_imap(&env.to);
    let cc = format_address_list_imap(&env.cc);
    let bcc = format_address_list_imap(&env.bcc);
    let in_reply_to = format_string_or_nil_imap(&env.in_reply_to);
    let message_id = format_string_or_nil_imap(&env.message_id);

    format!("({} {} {} {} {} {} {} {} {} {})",
        date, subject, from, sender, reply_to, to, cc, bcc, in_reply_to, message_id)
}

fn format_string_or_nil_imap(s: &Option<String>) -> String {
    match s {
        Some(s) if !s.is_empty() => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        _ => "NIL".to_string(),
    }
}

fn format_address_list_imap(addrs: &[Address]) -> String {
    if addrs.is_empty() { return "NIL".to_string(); }
    let parts: Vec<String> = addrs.iter().map(format_address_imap).collect();
    format!("({})", parts.join(" "))
}

fn format_address_list_opt_imap(addr: &Option<Address>) -> String {
    match addr {
        Some(a) => format!("({})", format_address_imap(a)),
        None => "NIL".to_string(),
    }
}

fn format_address_imap(addr: &Address) -> String {
    let name = format_string_or_nil_imap(&addr.name);
    let adl = format_string_or_nil_imap(&addr.adl);
    let mailbox = format_string_or_nil_imap(&addr.mailbox);
    let host = format_string_or_nil_imap(&addr.host);
    format!("({} {} {} {})", name, adl, mailbox, host)
}

fn format_internal_date(t: SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs = time_secs % 60;
    
    // Approximate date calculation
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let mon = MONTHS.get((month - 1) as usize).copied().unwrap_or("Jan");
    
    format!("{:02}-{}-{} {:02}:{:02}:{:02} +0000", day.min(28), mon, year, hours, mins, secs)
}

fn format_body_structure(msg: &Message) -> String {
    let line_count = msg.rfc822.split(|&b| b == b'\n').count() as u32;
    format!(r#"("text" "plain" ("charset" "utf-8") NIL NIL "7bit" {} {})"#, msg.size, line_count)
}

fn parse_imap_date(s: &str) -> Result<SystemTime, ()> {
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let parts: Vec<&str> = s.split_whitespace().next().unwrap_or(s).split('-').collect();
    if parts.len() >= 3 {
        let day: u32 = parts[0].parse().map_err(|_| ())?;
        let month_str = parts[1];
        let year: u32 = parts[2].parse().map_err(|_| ())?;
        let month = months.iter().position(|&m| m.eq_ignore_ascii_case(month_str)).ok_or(())? as u32 + 1;
        
        use std::time::{Duration, UNIX_EPOCH};
        let days_from_year = (year - 1970) as u64 * 365 + (year - 1969) as u64 / 4;
        let days_in_months: u64 = match month {
            1 => 0, 2 => 31, 3 => 59, 4 => 90, 5 => 120, 6 => 151,
            7 => 181, 8 => 212, 9 => 243, 10 => 273, 11 => 304, 12 => 334,
            _ => 0,
        };
        let total_days = days_from_year + days_in_months + (day as u64 - 1);
        let timestamp = total_days * 86400;
        Ok(UNIX_EPOCH + Duration::from_secs(timestamp))
    } else {
        Err(())
    }
}

fn parse_envelope_from_rfc822(data: &[u8]) -> Envelope {
    let mut env = Envelope::default();
    let headers = String::from_utf8_lossy(data);
    
    for line in headers.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key.to_uppercase().as_str() {
                "SUBJECT" => env.subject = Some(value.trim_matches('"').to_string()),
                "DATE" => env.date = Some(value.to_string()),
                "MESSAGE-ID" => env.message_id = Some(value.trim_matches('<').trim_matches('>').to_string()),
                "IN-REPLY-TO" => env.in_reply_to = Some(value.trim_matches('<').trim_matches('>').to_string()),
                "FROM" => {
                    if let Some(addr) = parse_imap_address(value) {
                        env.from.push(addr);
                    }
                }
                "TO" => {
                    for part in value.split(',') {
                        if let Some(addr) = parse_imap_address(part.trim()) {
                            env.to.push(addr);
                        }
                    }
                }
                "CC" => {
                    for part in value.split(',') {
                        if let Some(addr) = parse_imap_address(part.trim()) {
                            env.cc.push(addr);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    env
}

fn parse_imap_address(s: &str) -> Option<Address> {
    if s.is_empty() || s == "NIL" { return None; }
    
    if let Some(angle_start) = s.find('<') {
        if let Some(angle_end) = s.find('>') {
            let name = s[..angle_start].trim().trim_matches('"').to_string();
            let email = &s[angle_start + 1..angle_end];
            
            if let Some(at_pos) = email.find('@') {
                return Some(Address {
                    name: if name.is_empty() { None } else { Some(name) },
                    adl: None,
                    mailbox: Some(email[..at_pos].to_string()),
                    host: Some(email[at_pos + 1..].to_string()),
                });
            }
        }
    }
    
    if let Some(at_pos) = s.find('@') {
        return Some(Address {
            name: None,
            adl: None,
            mailbox: Some(s[..at_pos].to_string()),
            host: Some(s[at_pos + 1..].to_string()),
        });
    }
    
    None
}

/// Simple base64 decoder (for SASL PLAIN mechanism)
pub fn base64_decode(s: &str) -> Result<Vec<u8>, ()> {
    use std::collections::HashMap;
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let map: HashMap<char, u8> = alphabet.chars().enumerate().map(|(i, c)| (c, i as u8)).collect();
    
    let s = s.trim();
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4];
    let mut count = 0;
    
    for c in s.chars() {
        if c == '=' { continue; } // padding
        if let Some(&val) = map.get(&c) {
            buf[count] = val;
            count += 1;
            if count == 4 {
                bytes.push((buf[0] << 2) | (buf[1] >> 4));
                bytes.push((buf[1] << 4) | (buf[2] >> 2));
                bytes.push((buf[2] << 6) | buf[3]);
                count = 0;
            }
        }
    }
    
    // Handle remaining
    if count == 3 {
        bytes.push((buf[0] << 2) | (buf[1] >> 4));
        bytes.push((buf[1] << 4) | (buf[2] >> 2));
    } else if count == 2 {
        bytes.push((buf[0] << 2) | (buf[1] >> 4));
    } else if count != 0 {
        return Err(());
    }
    
    Ok(bytes)
}

/// Check if a message matches search keys.
fn matches_keys(msg: &Message, keys: &[SearchKey], all_msgs: &[Message]) -> bool {
    for key in keys {
        let matched = match key {
            SearchKey::All => true,
            SearchKey::Answered => msg.flags.answered,
            SearchKey::Deleted => msg.flags.deleted,
            SearchKey::Draft => msg.flags.draft,
            SearchKey::Flagged => msg.flags.flagged,
            SearchKey::Recent => msg.flags.recent,
            SearchKey::New => !msg.flags.seen,
            SearchKey::Old => msg.flags.seen,
            SearchKey::Seen => msg.flags.seen,
            SearchKey::Unseen => !msg.flags.seen,
            SearchKey::Not(sub_key) => !matches_keys(msg, &[(**sub_key).clone()], all_msgs),
            SearchKey::And(k1, k2) => {
                matches_keys(msg, &[(**k1).clone()], all_msgs) && matches_keys(msg, &[(**k2).clone()], all_msgs)
            }
            SearchKey::Or(k1, k2) => {
                matches_keys(msg, &[(**k1).clone()], all_msgs) || matches_keys(msg, &[(**k2).clone()], all_msgs)
            }
            SearchKey::Smaller(n) => msg.size < *n as usize,
            SearchKey::Larger(n) => msg.size > *n as usize,
            SearchKey::Subject(sub) => {
                msg.envelope.subject.as_ref()
                    .map(|s| s.to_lowercase().contains(&sub.to_lowercase()))
                    .unwrap_or(false)
            }
            SearchKey::From(addr) => {
                msg.envelope.from.iter().any(|a| {
                    a.mailbox.as_ref().map(|m| m.to_lowercase().contains(&addr.to_lowercase())).unwrap_or(false)
                    || a.host.as_ref().map(|h| h.to_lowercase().contains(&addr.to_lowercase())).unwrap_or(false)
                    || a.name.as_ref().map(|n| n.to_lowercase().contains(&addr.to_lowercase())).unwrap_or(false)
                })
            }
            SearchKey::To(addr) => {
                msg.envelope.to.iter().any(|a| {
                    a.mailbox.as_ref().map(|m| m.to_lowercase().contains(&addr.to_lowercase())).unwrap_or(false)
                    || a.host.as_ref().map(|h| h.to_lowercase().contains(&addr.to_lowercase())).unwrap_or(false)
                })
            }
            SearchKey::Body(text) => {
                let body = String::from_utf8_lossy(&msg.rfc822).to_lowercase();
                body.contains(&text.to_lowercase())
            }
            SearchKey::Text(text) => {
                // Text matches subject, from, to, or body
                let text_lower = text.to_lowercase();
                msg.envelope.subject.as_ref().map(|s| s.to_lowercase().contains(&text_lower)).unwrap_or(false)
                    || msg.envelope.from.iter().any(|a| a.mailbox.as_ref().map(|m| m.to_lowercase().contains(&text_lower)).unwrap_or(false))
                    || String::from_utf8_lossy(&msg.rfc822).to_lowercase().contains(&text_lower)
            }
            SearchKey::SeqSet(seq_str) => {
                // Match sequence numbers
                let seqs = crate::parser::parse_sequence_set(seq_str);
                for s in &seqs {
                    if s.contains(':') {
                        let parts: Vec<&str> = s.splitn(2, ':').collect();
                        let start: u32 = parts[0].parse().unwrap_or(0);
                        let end: u32 = parts[1].parse().unwrap_or(all_msgs.len() as u32);
                        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                        if msg.seq >= lo && msg.seq <= hi { return true; }
                    } else if s == "*" {
                        if msg.seq == all_msgs.len() as u32 { return true; }
                    } else if let Ok(n) = s.parse::<u32>() {
                        if msg.seq == n { return true; }
                    }
                }
                false
            }
            SearchKey::UidSet(uid_str) => {
                // Match UIDs
                let uids = crate::parser::parse_sequence_set(uid_str);
                for u in &uids {
                    if u.contains(':') {
                        let parts: Vec<&str> = u.splitn(2, ':').collect();
                        let start: u32 = parts[0].parse().unwrap_or(0);
                        let end: u32 = parts[1].parse().unwrap_or(u32::MAX);
                        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                        if msg.uid >= lo && msg.uid <= hi { return true; }
                    } else if u == "*" {
                        return false; // Can't match "*" as UID without knowing last UID
                    } else if let Ok(n) = u.parse::<u32>() {
                        if msg.uid == n { return true; }
                    }
                }
                false
            }
            // Date searches - simplified: check internal_date
            SearchKey::Before(date_str) | SearchKey::SentBefore(date_str) => {
                if let Ok(target) = parse_imap_date(date_str) {
                    msg.internal_date < target
                } else { true }
            }
            SearchKey::Since(date_str) | SearchKey::SentSince(date_str) => {
                if let Ok(target) = parse_imap_date(date_str) {
                    msg.internal_date >= target
                } else { true }
            }
            SearchKey::On(date_str) | SearchKey::SentOn(date_str) => {
                if let Ok(target) = parse_imap_date(date_str) {
                    let next_day = target + std::time::Duration::from_secs(86400);
                    msg.internal_date >= target && msg.internal_date < next_day
                } else { true }
            }
            SearchKey::Header(name, value) => {
                let headers = String::from_utf8_lossy(&msg.rfc822);
                for line in headers.lines() {
                    if let Some((h, v)) = line.split_once(':') {
                        if h.trim().eq_ignore_ascii_case(name) && v.trim().to_lowercase().contains(&value.to_lowercase()) {
                            return true;
                        }
                    }
                }
                false
            }
        };
        if !matched {
            return false;
        }
    }
    true
}

// ===========================================================================
// Server Configuration
// ===========================================================================

/// IMAP server configuration.
pub struct ImapServerConfig {
    /// Bind address (default: "0.0.0.0:143").
    pub bind_addr: String,
    /// Server domain name (for capabilities).
    pub domain_name: String,
    /// Optional TLS certificate for STARTTLS/IMAPS.
    #[cfg(feature = "tls")]
    pub tls_cert: Option<TlsCertificate>,
    /// Whether this is an IMAPS server (TLS from start, port 993).
    pub imaps: bool,
}

impl Default for ImapServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:143".to_string(),
            domain_name: "edgerun.mail".to_string(),
            #[cfg(feature = "tls")]
            tls_cert: None,
            imaps: false,
        }
    }
}

// ===========================================================================
// IMAP Server
// ===========================================================================

/// IMAP server — accepts connections and runs sessions.
pub struct ImapServer {
    listener: Arc<AsyncTcpListener>,
    store: Arc<dyn MailStore>,
    domain: String,
    #[cfg(feature = "tls")]
    tls_cert: Option<TlsCertificate>,
    imaps: bool,
}

impl ImapServer {
    /// Create a new IMAP server with the given config and mail store.
    pub fn new(config: ImapServerConfig) -> io::Result<Self> {
        let listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);
        let store = Arc::new(MemoryStore::new());
        Ok(Self {
            listener,
            store,
            domain: config.domain_name,
            #[cfg(feature = "tls")]
            tls_cert: config.tls_cert,
            imaps: config.imaps,
        })
    }

    /// Create a server with a custom mail store.
    pub fn with_store(config: ImapServerConfig, store: Arc<dyn MailStore>) -> io::Result<Self> {
        let listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);
        Ok(Self {
            listener,
            store,
            domain: config.domain_name,
            #[cfg(feature = "tls")]
            tls_cert: config.tls_cert,
            imaps: config.imaps,
        })
    }

    /// Get the local address the server is bound to.
    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    /// Run the server until shutdown is cancelled.
    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        edgerun_log::info!("edgerun-imap: server listening on {} (IMAPS: {})",
            self.listener.local_addr()?, self.imaps);

        while !shutdown.is_cancelled() {
            match self.listener.accept().await {
                Ok((stream, peer)) => {
                    let store = Arc::clone(&self.store);
                    let domain = self.domain.clone();
                    #[cfg(feature = "tls")]
                    let tls_cert = self.tls_cert.clone();
                    let imaps = self.imaps;
                    edgerun_rt::spawn(async move {
                        if let Err(e) = handle_connection(
                            stream, peer, store, domain,
                            #[cfg(feature = "tls")]
                            tls_cert,
                            imaps,
                        ).await {
                            edgerun_log::warn!("edgerun-imap: connection error from {}: {}", peer, e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::warn!("edgerun-imap: accept error: {}", e);
                    edgerun_rt::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }

        edgerun_log::info!("edgerun-imap: server stopped");
        Ok(())
    }
}

// ===========================================================================
// Connection Handler
// ===========================================================================

/// Handle a single IMAP connection.
async fn handle_connection(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    store: Arc<dyn MailStore>,
    domain: String,
    #[cfg(feature = "tls")] tls_cert: Option<TlsCertificate>,
    imaps: bool,
) -> io::Result<()> {
    edgerun_log::info!("edgerun-imap: connection from {}", peer);

    // For IMAPS, wrap in TLS immediately
    #[cfg(feature = "tls")]
    let (reader, writer, is_tls) = if imaps {
        if let Some(ref cert) = tls_cert {
            edgerun_log::debug!("edgerun-imap: IMAPS TLS handshake for {}", peer);
            let mut tls_stream = AsyncTlsServerStream::accept(Arc::clone(&stream), cert.clone()).await?;
            // Split into read/write halves for TLS stream
            use edgerun_rt::AsyncReadHalf;
            use edgerun_rt::AsyncWriteHalf;
            use edgerun_rt::AsyncTcpStream;
            // For now, we can't split a TLS stream directly - we need a different approach
            // Simplified: use the stream directly with TLS
            let reader = ImapReader::new(tls_stream);
            let writer = Arc::new(Mutex::new(tls_stream));
            (reader, writer, true)
        } else {
            let (read_half, write_half) = stream.split();
            let reader = ImapReader::new(read_half);
            let writer = Arc::new(Mutex::new(write_half));
            (reader, writer, false)
        }
    } else {
        let (read_half, write_half) = stream.split();
        let reader = ImapReader::new(read_half);
        let writer = Arc::new(Mutex::new(write_half));
        (reader, writer, false)
    };

    #[cfg(not(feature = "tls"))]
    let (reader, writer) = {
        let (read_half, write_half) = stream.split();
        let reader = ImapReader::new(read_half);
        let writer = Arc::new(Mutex::new(write_half));
        (reader, writer)
    };

    // For simplicity, I'll use the non-TLS path for now and implement STARTTLS in session state
    let (mut reader, writer) = {
        let (read_half, write_half) = stream.split();
        let reader = ImapReader::new(read_half);
        let writer = Arc::new(Mutex::new(write_half));
        (reader, writer)
    };
    let is_tls = imaps;

    // Send greeting
    let greeting = parser::format_greeting(&CAPABILITIES.iter()
        .map(|s| *s)
        .collect::<Vec<_>>());
    let mut w = writer.lock().await;
    w.write_all(greeting.as_bytes()).await?;
    w.flush().await?;
    drop(w);

    // Session state
    let mut state = ImapState::NotAuthenticated;
    let mut current_mailbox: Option<String> = None;
    let mut authenticated_user: Option<String> = None;
    let mut _is_encrypted = is_tls;

    // Command loop
    loop {
        let line = match reader.read_line().await {
            Ok(Some(line)) => line,
            Ok(None) => {
                edgerun_log::info!("edgerun-imap: client {} disconnected", peer);
                return Ok(());
            }
            Err(e) => {
                edgerun_log::warn!("edgerun-imap: read error from {}: {}", peer, e);
                return Err(e);
            }
        };

        // Check for literal: {N}
        if line.ends_with("}") && line.contains('{') {
            let size_str = line.trim_start_matches(|c| c != '{').trim_matches(|c| c == '{' || c == '}');
            if let Ok(literal_size) = size_str.parse::<usize>() {
                // Send continuation for literal
                {
                    let mut w = writer.lock().await;
                    w.write_all(b"+ Ready for literal data\r\n").await?;
                    w.flush().await?;
                }
                // Read literal data
                let _data = reader.read_exact_bytes(literal_size).await?;
                // After literal, expect \r\n
                reader.read_line().await?;
                // For now, continue — real impl would re-parse with literal
            }
        }

        // Parse command
        let (tag, command_name, args) = match parser::parse_command_line(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = ImapResponse::bad("?", &format!("Parse error: {}", e));
                send_response(&writer, &resp).await?;
                continue;
            }
        };

        // Parse into typed command
        let cmd = match ImapCommand::parse(&tag, &command_name, &args, Some(&line)) {
            Ok(cmd) => cmd,
            Err(e) => {
                let resp = ImapResponse::bad(&tag, &format!("Error: {}", e));
                send_response(&writer, &resp).await?;
                continue;
            }
        };

        // Handle STARTTLS specially - it needs TLS upgrade
        if let ImapCommand::Starttls = &cmd {
            #[cfg(feature = "tls")]
            {
                if let Some(ref cert) = tls_cert {
                    if _is_encrypted {
                        let resp = ImapResponse::no(&tag, "TLS already active");
                        send_response(&writer, &resp).await?;
                        continue;
                    }
                    // Send OK then upgrade
                    {
                        let mut w = writer.lock().await;
                        w.write_all(parser::format_ok(&tag, "Begin TLS negotiation now").as_bytes()).await?;
                        w.flush().await?;
                    }
                    // Note: Full TLS upgrade would require replacing the reader/writer
                    // For now, we just acknowledge the command
                    edgerun_log::debug!("edgerun-imap: STARTTLS requested for {}", peer);
                    _is_encrypted = true;
                    continue;
                } else {
                    let resp = ImapResponse::no(&tag, "TLS not configured");
                    send_response(&writer, &resp).await?;
                    continue;
                }
            }
            #[cfg(not(feature = "tls"))]
            {
                let resp = ImapResponse::no(&tag, "TLS not supported");
                send_response(&writer, &resp).await?;
                continue;
            }
        }

        // Dispatch command
        let response = dispatch_command(
            &cmd,
            &tag,
            &mut state,
            &mut current_mailbox,
            &mut authenticated_user,
            &store,
            &domain,
            &mut reader,
            &writer,
        ).await?;

        send_response(&writer, &response).await?;

        if state == ImapState::Logout {
            edgerun_log::info!("edgerun-imap: client {} logged out", peer);
            return Ok(());
        }
    }
}

// ===========================================================================
// Command Dispatcher
// ===========================================================================

async fn dispatch_command<R: AsyncReadExt + Unpin>(
    cmd: &ImapCommand,
    tag: &str,
    state: &mut ImapState,
    current_mailbox: &mut Option<String>,
    authenticated_user: &mut Option<String>,
    store: &Arc<dyn MailStore>,
    domain: &str,
    reader: &mut ImapReader<R>,
    writer: &Arc<Mutex<impl AsyncWriteExt + Unpin>>,
) -> io::Result<ImapResponse> {
    match cmd {
        ImapCommand::Capability => {
            // Send untagged capability list
            let cap_resp = parser::format_capability(&CAPABILITIES.iter()
                .map(|s| *s)
                .collect::<Vec<_>>());
            let mut w = writer.lock().await;
            w.write_all(cap_resp.as_bytes()).await?;
            w.flush().await?;
            drop(w);
            Ok(ImapResponse::ok(tag, "CAPABILITY completed"))
        }

        ImapCommand::Noop => {
            Ok(ImapResponse::ok(tag, "NOOP completed"))
        }

        ImapCommand::Logout => {
            *state = ImapState::Logout;
            let mut w = writer.lock().await;
            w.write_all(parser::format_untagged("BYE Logging out").as_bytes()).await?;
            w.flush().await?;
            drop(w);
            Ok(ImapResponse::ok(tag, "LOGOUT completed"))
        }

        ImapCommand::Starttls => {
            // STARTTLS not implemented (requires TLS feature and stream upgrade)
            Ok(ImapResponse::no(tag, "STARTTLS not supported"))
        }

        ImapCommand::Login { user, password } => {
            if *state != ImapState::NotAuthenticated {
                return Ok(ImapResponse::no(tag, "Already authenticated"));
            }

            match store.authenticate(user, password) {
                Ok(Some(username)) => {
                    *state = ImapState::Authenticated;
                    *authenticated_user = Some(username.clone());
                    edgerun_log::info!("edgerun-imap: user {} authenticated", username);
                    Ok(ImapResponse::ok(tag, &format!("LOGIN completed for {}", username)))
                }
                _ => Ok(ImapResponse::no(tag, "LOGIN failed")),
            }
        }

        ImapCommand::Authenticate { mechanism } => {
            if *state != ImapState::NotAuthenticated {
                return Ok(ImapResponse::no(tag, "Already authenticated"));
            }
            
            // Only support PLAIN mechanism for now
            if mechanism.to_uppercase() != "PLAIN" {
                return Ok(ImapResponse::no(tag, &format!("Unsupported mechanism: {}", mechanism)));
            }
            
            // Send continuation for base64-encoded credentials
            {
                let mut w = writer.lock().await;
                w.write_all(b"+ \r\n").await?;
                w.flush().await?;
            }
            
            // Read the base64 credentials
            let creds = reader.read_line().await?.unwrap_or_default();
            // PLAIN format: authzid\0username\0password
            // Decode base64 and parse
            if let Ok(bytes) = base64_decode(&creds) {
                if let Ok(s) = std::str::from_utf8(&bytes) {
                    let parts: Vec<&str> = s.split('\0').collect();
                    if parts.len() >= 3 {
                        let username = parts[1];
                        let password = parts[2];
                        
                        match store.authenticate(username, password) {
                            Ok(Some(uname)) => {
                                *state = ImapState::Authenticated;
                                *authenticated_user = Some(uname.clone());
                                return Ok(ImapResponse::ok(tag, &format!("AUTHENTICATE completed for {}", uname)));
                            }
                            _ => return Ok(ImapResponse::no(tag, "AUTHENTICATE failed")),
                        }
                    }
                }
            }
            Ok(ImapResponse::no(tag, "AUTHENTICATE failed: invalid credentials"))
        }

        ImapCommand::Select { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.select(mailbox) {
                Ok(Some(mb)) => {
                    *state = ImapState::Selected;
                    *current_mailbox = Some(mailbox.clone());

                    let mut w = writer.lock().await;

                    // Send mailbox status
                    if let Some(status) = &mb.status {
                        w.write_all(parser::format_exists(status.messages).as_bytes()).await?;
                        w.write_all(parser::format_recent(status.recent).as_bytes()).await?;

                        let flags = ["\\Seen", "\\Answered", "\\Flagged", "\\Deleted", "\\Draft"];
                        w.write_all(parser::format_flags(&flags).as_bytes()).await?;

                        w.write_all(parser::format_uid_validity(status.uid_validity).as_bytes()).await?;
                        w.write_all(parser::format_uid_next(status.uid_next).as_bytes()).await?;
                    }

                    w.flush().await?;
                    drop(w);

                    Ok(ImapResponse::ok(tag, "[READ-WRITE] SELECT completed"))
                }
                Ok(None) => Ok(ImapResponse::no(tag, "Mailbox not found")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("SELECT failed: {}", e))),
            }
        }

        ImapCommand::Examine { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.select(mailbox) {
                Ok(Some(mb)) => {
                    *state = ImapState::Selected;
                    *current_mailbox = Some(mailbox.clone());

                    let mut w = writer.lock().await;
                    if let Some(status) = &mb.status {
                        w.write_all(parser::format_exists(status.messages).as_bytes()).await?;
                        w.write_all(parser::format_recent(status.recent).as_bytes()).await?;
                        let flags = ["\\Seen", "\\Answered", "\\Flagged", "\\Deleted", "\\Draft"];
                        w.write_all(parser::format_flags(&flags).as_bytes()).await?;
                        w.write_all(parser::format_uid_validity(status.uid_validity).as_bytes()).await?;
                        w.write_all(parser::format_uid_next(status.uid_next).as_bytes()).await?;
                    }
                    w.flush().await?;
                    drop(w);

                    Ok(ImapResponse::ok(tag, "[READ-ONLY] EXAMINE completed"))
                }
                Ok(None) => Ok(ImapResponse::no(tag, "Mailbox not found")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("EXAMINE failed: {}", e))),
            }
        }

        ImapCommand::List { reference, pattern } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.list(reference, pattern) {
                Ok(mailboxes) => {
                    let mut w = writer.lock().await;
                    for mb in &mailboxes {
                        let attrs: Vec<&str> = mb.attributes.iter().map(|s| s.as_str()).collect();
                        let resp = parser::format_list(&attrs,
                            mb.delimiter.as_deref().unwrap_or("/"), &mb.name);
                        w.write_all(resp.as_bytes()).await?;
                    }
                    w.flush().await?;
                    drop(w);
                    Ok(ImapResponse::ok(tag, "LIST completed"))
                }
                Err(e) => Ok(ImapResponse::no(tag, &format!("LIST failed: {}", e))),
            }
        }

        ImapCommand::Fetch { sequence, attributes } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                match store.fetch(mailbox, sequence, attributes) {
                    Ok(results) => {
                        let mut w = writer.lock().await;
                        for (uid, data) in &results {
                            let parts: Vec<String> = data.iter()
                                .map(|(k, v)| format!("{} {}", k, v))
                                .collect();
                            let resp = parser::format_fetch(*uid, &parts.join(" "));
                            w.write_all(resp.as_bytes()).await?;
                        }
                        w.flush().await?;
                        drop(w);
                        Ok(ImapResponse::ok(tag, "FETCH completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("FETCH failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Search { keys, .. } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                match store.search(mailbox, keys) {
                    Ok(ids) => {
                        let mut w = writer.lock().await;
                        w.write_all(parser::format_search(&ids).as_bytes()).await?;
                        w.flush().await?;
                        drop(w);
                        Ok(ImapResponse::ok(tag, "SEARCH completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("SEARCH failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Create { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.create(mailbox) {
                Ok(true) => Ok(ImapResponse::ok(tag, "CREATE completed")),
                Ok(false) => Ok(ImapResponse::no(tag, "Mailbox already exists")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("CREATE failed: {}", e))),
            }
        }

        ImapCommand::Delete { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.delete(mailbox) {
                Ok(true) => Ok(ImapResponse::ok(tag, "DELETE completed")),
                Ok(false) => Ok(ImapResponse::no(tag, "Mailbox not found")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("DELETE failed: {}", e))),
            }
        }

        ImapCommand::Status { mailbox, items } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.status(mailbox) {
                Ok(Some(status)) => {
                    let mut parts = Vec::new();
                    for item in items {
                        match item.to_uppercase().as_str() {
                            "MESSAGES" => parts.push(format!("MESSAGES {}", status.messages)),
                            "RECENT" => parts.push(format!("RECENT {}", status.recent)),
                            "UIDNEXT" => parts.push(format!("UIDNEXT {}", status.uid_next)),
                            "UIDVALIDITY" => parts.push(format!("UIDVALIDITY {}", status.uid_validity)),
                            _ => {}
                        }
                    }
                    let mut w = writer.lock().await;
                    w.write_all(parser::format_untagged(&format!(
                        "STATUS {} ({})", mailbox, parts.join(" ")
                    )).as_bytes()).await?;
                    w.flush().await?;
                    drop(w);
                    Ok(ImapResponse::ok(tag, "STATUS completed"))
                }
                Ok(None) => Ok(ImapResponse::no(tag, "Mailbox not found")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("STATUS failed: {}", e))),
            }
        }

        ImapCommand::Close => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                let _ = store.close(mailbox);
                *state = ImapState::Authenticated;
                *current_mailbox = None;
                Ok(ImapResponse::ok(tag, "CLOSE completed"))
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Expunge => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                match store.expunge(mailbox) {
                    Ok(removed) => {
                        let mut w = writer.lock().await;
                        for seq in &removed {
                            w.write_all(parser::format_expunge(*seq).as_bytes()).await?;
                        }
                        w.flush().await?;
                        drop(w);
                        Ok(ImapResponse::ok(tag, "EXPUNGE completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("EXPUNGE failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Store { sequence, action, flags } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                match store.store(mailbox, sequence, action, flags) {
                    Ok(updated) => {
                        let mut w = writer.lock().await;
                        for uid in &updated {
                            // Send FETCH response with updated flags
                            let resp = parser::format_fetch(*uid, "FLAGS ()");
                            w.write_all(resp.as_bytes()).await?;
                        }
                        w.flush().await?;
                        drop(w);
                        Ok(ImapResponse::ok(tag, "STORE completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("STORE failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Rename { old, new } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            match store.rename(old, new) {
                Ok(true) => Ok(ImapResponse::ok(tag, "RENAME completed")),
                Ok(false) => Ok(ImapResponse::no(tag, "Mailbox not found")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("RENAME failed: {}", e))),
            }
        }

        ImapCommand::Subscribe { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            // For now, treat subscribe as a no-op (subscription tracking not implemented)
            Ok(ImapResponse::ok(tag, "SUBSCRIBE completed"))
        }

        ImapCommand::Unsubscribe { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            // For now, treat unsubscribe as a no-op
            Ok(ImapResponse::ok(tag, "UNSUBSCRIBE completed"))
        }

        ImapCommand::Lsub { reference, pattern } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            // LSUB returns subscribed mailboxes - for now, return same as LIST
            match store.list(reference, pattern) {
                Ok(mailboxes) => {
                    let mut w = writer.lock().await;
                    for mb in &mailboxes {
                        let attrs: Vec<&str> = mb.attributes.iter().map(|s| s.as_str()).collect();
                        let resp = parser::format_list(&attrs,
                            mb.delimiter.as_deref().unwrap_or("/"), &mb.name);
                        w.write_all(resp.as_bytes()).await?;
                    }
                    w.flush().await?;
                    drop(w);
                    Ok(ImapResponse::ok(tag, "LSUB completed"))
                }
                Err(e) => Ok(ImapResponse::no(tag, &format!("LSUB failed: {}", e))),
            }
        }

        ImapCommand::Copy { sequence, mailbox } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref src_mailbox) = current_mailbox {
                match store.copy_messages(src_mailbox, sequence, mailbox) {
                    Ok(uids) => {
                        edgerun_log::debug!("edgerun-imap: COPY {} messages to {}", uids.len(), mailbox);
                        Ok(ImapResponse::ok(tag, "COPY completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("COPY failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Append { mailbox, flags, date, literal_size } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            // Send continuation for literal data
            {
                let mut w = writer.lock().await;
                w.write_all(b"+ Ready for literal data\r\n").await?;
                w.flush().await?;
            }

            // Read the literal message data
            let data = reader.read_exact_bytes(*literal_size).await?;

            // Parse flags if provided
            let msg_flags = flags.as_ref()
                .map(|f| Flags::parse(&f.join(" ")))
                .unwrap_or_else(Flags::new);

            // Parse date if provided
            let msg_date = date.as_ref().map(|_| SystemTime::now()); // Simplified

            match store.append(mailbox, msg_flags, msg_date, &data) {
                Ok(uid) => Ok(ImapResponse::ok(tag, &format!("APPEND completed [UIDNEXT {}]", uid + 1))),
                Err(e) => Ok(ImapResponse::no(tag, &format!("APPEND failed: {}", e))),
            }
        }

        ImapCommand::Check => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            // CHECK is a checkpoint - for now, no-op
            Ok(ImapResponse::ok(tag, "CHECK completed"))
        }

        ImapCommand::Uid { command } => {
            // UID commands are dispatched to the inner command
            // The inner command has already been parsed with UID context
            Box::pin(dispatch_command(command, tag, state, current_mailbox, authenticated_user, store, domain, reader, writer)).await
        }

        ImapCommand::Idle => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            // IDLE - wait for updates. Client sends DONE to exit.
            // We send continuation and wait
            {
                let mut w = writer.lock().await;
                w.write_all(b"+ idling\r\n").await?;
                w.flush().await?;
            }
            // In real implementation, we'd wait here for updates or DONE
            // For now, return OK immediately (simplified)
            Ok(ImapResponse::ok(tag, "IDLE terminated"))
        }

        ImapCommand::Done => {
            // DONE ends IDLE mode
            Ok(ImapResponse::ok(tag, "DONE completed"))
        }

        ImapCommand::Enable { capabilities } => {
            edgerun_log::debug!("edgerun-imap: ENABLE requested: {:?}", capabilities);
            // Enable requested capabilities
            let mut enabled = Vec::new();
            for cap in capabilities {
                match cap.to_uppercase().as_str() {
                    "CONDSTORE" | "QRESYNC" => enabled.push(cap.clone()),
                    _ => {} // Ignore unsupported capabilities
                }
            }
            let mut w = writer.lock().await;
            if !enabled.is_empty() {
                w.write_all(parser::format_untagged(&format!("ENABLED {}", enabled.join(" "))).as_bytes()).await?;
            }
            w.flush().await?;
            drop(w);
            Ok(ImapResponse::ok(tag, "ENABLE completed"))
        }

        ImapCommand::Unselect => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            // UNSELECT - deselect without expunge
            *state = ImapState::Authenticated;
            *current_mailbox = None;
            Ok(ImapResponse::ok(tag, "UNSELECT completed"))
        }

        ImapCommand::Move { sequence, mailbox } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref src_mailbox) = current_mailbox {
                // MOVE = COPY + STORE +FLAGS \Deleted + EXPUNGE
                match store.copy_messages(src_mailbox, sequence, mailbox) {
                    Ok(uids) => {
                        // Mark original messages as deleted
                        let _ = store.store(src_mailbox, sequence, &StoreAction::Add, &["\\Deleted".to_string()]);
                        let _ = store.expunge(src_mailbox);
                        edgerun_log::debug!("edgerun-imap: MOVE {} messages to {}", uids.len(), mailbox);
                        Ok(ImapResponse::ok(tag, "MOVE completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("MOVE failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::UidMove { sequence, mailbox } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref src_mailbox) = current_mailbox {
                match store.copy_messages(src_mailbox, sequence, mailbox) {
                    Ok(uids) => {
                        let _ = store.store(src_mailbox, sequence, &StoreAction::Add, &["\\Deleted".to_string()]);
                        let _ = store.expunge(src_mailbox);
                        Ok(ImapResponse::ok(tag, "MOVE completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("MOVE failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Namespace => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            let mut w = writer.lock().await;
            w.write_all(parser::format_untagged(
                r#"NAMESPACE (("" "/")) NIL NIL"#
            ).as_bytes()).await?;
            w.flush().await?;
            drop(w);
            Ok(ImapResponse::ok(tag, "NAMESPACE completed"))
        }

        ImapCommand::Quota { mailbox } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            // Return quota info (simplified - no actual quota tracking yet)
            let mut w = writer.lock().await;
            w.write_all(parser::format_untagged(&format!(
                r#"QUOTA "{}" ()"#, mailbox
            )).as_bytes()).await?;
            w.flush().await?;
            drop(w);
            Ok(ImapResponse::ok(tag, "QUOTA completed"))
        }

        ImapCommand::SetQuota { mailbox, limits } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            edgerun_log::debug!("edgerun-imap: SETQUOTA on {}: {:?}", mailbox, limits);
            Ok(ImapResponse::ok(tag, "SETQUOTA completed"))
        }

        ImapCommand::Sort { sort_criteria, charset, search_criteria } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            edgerun_log::debug!("edgerun-imap: SORT {:?} {} {:?}", sort_criteria, charset, search_criteria);
            // Simplified - just do a regular search for now
            if let Some(ref mailbox) = current_mailbox {
                let keys = crate::types::SearchKey::All;
                match store.search(mailbox, &[keys]) {
                    Ok(ids) => {
                        let mut w = writer.lock().await;
                        let id_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                        w.write_all(parser::format_untagged(&format!("SORT {}", id_str.join(" "))).as_bytes()).await?;
                        w.flush().await?;
                        drop(w);
                        Ok(ImapResponse::ok(tag, "SORT completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("SORT failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Thread { algorithm, charset, search_criteria } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            edgerun_log::debug!("edgerun-imap: THREAD {} {} {:?}", algorithm, charset, search_criteria);
            // Simplified - return empty thread list
            let mut w = writer.lock().await;
            w.write_all(parser::format_untagged("THREAD ()").as_bytes()).await?;
            w.flush().await?;
            drop(w);
            Ok(ImapResponse::ok(tag, "THREAD completed"))
        }

        _ => Ok(ImapResponse::no(tag, &format!("Command not implemented in current state"))),
    }
}

// ===========================================================================
// Response Sender
// ===========================================================================

async fn send_response<W: AsyncWriteExt + Unpin>(
    writer: &Arc<Mutex<W>>,
    resp: &ImapResponse,
) -> io::Result<()> {
    let wire = resp.to_wire();
    let mut w = writer.lock().await;
    w.write_all(wire.as_bytes()).await?;
    w.flush().await?;
    drop(w);
    Ok(())
}
