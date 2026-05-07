//! IMAP server implementation (RFC 3501).
//!
//! Provides an async IMAP4rev1 server with:
//! - TCP listener with per-connection session handling
//! - Session state machine (NotAuthenticated → Authenticated → Selected → Logout)
//! - In-memory mailbox storage (pluggable via `MailStore` trait)
//! - STARTTLS support (with `tls` feature)

use crate::prelude::*;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;

use crate::rt::{
    AsyncRead, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWrite, AsyncWriteExt,
    CancellationToken, Mutex,
};

#[cfg(feature = "tls")]
use edgerun_tls::{AsyncTlsServerStream, CertificateAndKey};

use crate::command_middleware::{
    CommandMiddleware, ControlFlow as MwControlFlow, NextCommand, SessionExtensions,
};
use crate::imap::message::{ImapCommand, ImapResponse, ImapResult, StoreAction};
use crate::imap::parser::{self, ImapReader};
use crate::imap::types::{
    Address, Envelope, FetchAttr, Flags, ImapState, Mailbox, MailboxStatus, Message, SearchKey,
};
use crate::server::ConnectionInterceptor;

// ===========================================================================
// IMAP Session Extension Types
// ===========================================================================

/// Authenticated user identity stored in SessionExtensions.
#[derive(Clone)]
pub struct ImapUser(pub Option<String>);

/// Currently selected mailbox stored in SessionExtensions.
#[derive(Clone)]
pub struct ImapMailbox(pub Option<String>);

/// Connection state for middleware visibility.
#[derive(Clone)]
pub struct ImapConnState {
    pub state: ImapState,
    pub mailbox: Option<String>,
    pub authenticated_user: Option<String>,
}

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
// Quota Types
// ===========================================================================

/// Quota information for a mailbox.
#[derive(Debug, Clone)]
pub struct QuotaInfo {
    pub mailbox: String,
    pub storage_used: u32,  // KB used
    pub storage_limit: u32, // KB limit
    pub message_count: u32, // current messages
    pub message_limit: u32, // max messages
}

/// Internal quota tracking struct.
#[derive(Debug, Clone)]
struct Quota {
    storage_limit: u32,
    message_limit: u32,
}

// ===========================================================================
// Transport (supports in-place TLS upgrade for STARTTLS)
// ===========================================================================

/// Unified transport that can be upgraded from plain to TLS mid-session.
/// Mirrors the SMTP server's `SmtpTransport` pattern.
pub enum ImapTransport {
    Plain(AsyncTcpStream),
    #[cfg(feature = "tls")]
    Tls(AsyncTlsServerStream<AsyncTcpStream>),
}

impl ImapTransport {
    /// Whether this transport is already using TLS.
    #[cfg(feature = "tls")]
    pub fn is_tls(&self) -> bool {
        match self {
            ImapTransport::Plain(_) => false,
            ImapTransport::Tls(_) => true,
        }
    }

    #[cfg(not(feature = "tls"))]
    pub fn is_tls(&self) -> bool {
        false
    }

    /// Upgrade a plain transport to TLS in place.
    #[cfg(feature = "tls")]
    pub async fn upgrade_tls(self, cert_and_key: &CertificateAndKey) -> io::Result<ImapTransport> {
        match self {
            ImapTransport::Tls(_) => Err(io::Error::other("already using TLS")),
            ImapTransport::Plain(stream) => {
                let mut tls_stream = AsyncTlsServerStream::new(stream);
                tls_stream
                    .handshake(cert_and_key)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
                Ok(ImapTransport::Tls(tls_stream))
            }
        }
    }
}

impl AsyncRead for ImapTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            ImapTransport::Plain(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            ImapTransport::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ImapTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            ImapTransport::Plain(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            ImapTransport::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            ImapTransport::Plain(s) => Pin::new(s).poll_flush(cx),
            #[cfg(feature = "tls")]
            ImapTransport::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            ImapTransport::Plain(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            ImapTransport::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl Unpin for ImapTransport {}

struct BufferedImapTransport {
    transport: Option<ImapTransport>,
    read_buf: Vec<u8>,
    read_pos: usize,
}

impl BufferedImapTransport {
    fn new(transport: ImapTransport) -> Self {
        Self {
            transport: Some(transport),
            read_buf: Vec::with_capacity(4096),
            read_pos: 0,
        }
    }

    #[cfg(feature = "tls")]
    fn is_tls(&self) -> bool {
        self.transport
            .as_ref()
            .expect("IMAP transport missing")
            .is_tls()
    }

    fn has_buffered_read_bytes(&self) -> bool {
        self.read_pos < self.read_buf.len()
    }

    fn compact_read_buf(&mut self) {
        if self.read_pos == 0 {
            return;
        }
        if self.read_pos >= self.read_buf.len() {
            self.read_buf.clear();
            self.read_pos = 0;
        } else if self.read_pos >= 4096 {
            self.read_buf.drain(..self.read_pos);
            self.read_pos = 0;
        }
    }

    async fn read_line(&mut self) -> io::Result<Option<String>> {
        loop {
            if let Some(offset) = self.read_buf[self.read_pos..]
                .iter()
                .position(|&b| b == b'\n')
            {
                let line_end = self.read_pos + offset;
                let mut bytes = &self.read_buf[self.read_pos..line_end];
                if bytes.ends_with(b"\r") {
                    bytes = &bytes[..bytes.len() - 1];
                }
                let line = String::from_utf8_lossy(bytes).to_string();
                self.read_pos = line_end + 1;
                self.compact_read_buf();
                return Ok(Some(line));
            }

            self.compact_read_buf();
            let mut chunk = [0u8; 4096];
            let n = self
                .transport
                .as_mut()
                .expect("IMAP transport missing")
                .read(&mut chunk)
                .await?;
            if n == 0 {
                if self.has_buffered_read_bytes() {
                    let bytes = &self.read_buf[self.read_pos..];
                    let line = String::from_utf8_lossy(bytes).to_string();
                    self.read_buf.clear();
                    self.read_pos = 0;
                    return Ok(Some(line));
                }
                return Ok(None);
            }
            self.read_buf.extend_from_slice(&chunk[..n]);
        }
    }

    async fn read_exact_buffered(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        while !buf.is_empty() && self.has_buffered_read_bytes() {
            let available = self.read_buf.len() - self.read_pos;
            let to_copy = available.min(buf.len());
            buf[..to_copy].copy_from_slice(&self.read_buf[self.read_pos..self.read_pos + to_copy]);
            self.read_pos += to_copy;
            self.compact_read_buf();
            buf = &mut buf[to_copy..];
        }

        if !buf.is_empty() {
            self.transport
                .as_mut()
                .expect("IMAP transport missing")
                .read_exact(buf)
                .await?;
        }
        Ok(())
    }

    #[cfg(feature = "tls")]
    async fn upgrade_tls(&mut self, cert_and_key: &CertificateAndKey) -> io::Result<()> {
        if self.has_buffered_read_bytes() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cannot start TLS with buffered plaintext bytes",
            ));
        }

        let old = self.transport.take().expect("IMAP transport missing");
        self.transport = Some(old.upgrade_tls(cert_and_key).await?);
        self.read_buf.clear();
        self.read_pos = 0;
        Ok(())
    }
}

impl AsyncWrite for BufferedImapTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(self.transport.as_mut().expect("IMAP transport missing")).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.transport.as_mut().expect("IMAP transport missing")).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.transport.as_mut().expect("IMAP transport missing")).poll_shutdown(cx)
    }
}

impl Unpin for BufferedImapTransport {}

// ===========================================================================
// Mail Store Trait
// ===========================================================================

/// Trait for a pluggable mailbox backend.
pub trait MailStore: Send + Sync + 'static {
    /// Authenticate a user. Returns the username on success.
    fn authenticate(&self, user: &str, token: &str) -> io::Result<Option<String>>;

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
    fn fetch(
        &self,
        mailbox: &str,
        sequence: &str,
        attrs: &[FetchAttr],
    ) -> io::Result<Vec<(u32, HashMap<String, String>)>>;

    /// Store flags on messages.
    fn store(
        &self,
        mailbox: &str,
        sequence: &str,
        action: &StoreAction,
        flags: &[String],
    ) -> io::Result<Vec<u32>>;

    /// Search for messages matching criteria.
    fn search(&self, mailbox: &str, keys: &[SearchKey]) -> io::Result<Vec<u32>>;

    /// Sort messages matching search criteria, returning UIDs in sorted order.
    /// Each criterion can optionally be reversed (descending).
    fn sort(
        &self,
        mailbox: &str,
        keys: &[SearchKey],
        criteria: &[(String, bool)],
    ) -> io::Result<Vec<u32>>;

    /// Expunge deleted messages.
    fn expunge(&self, mailbox: &str) -> io::Result<Vec<u32>>;

    /// Append a message to a mailbox.
    fn append(
        &self,
        mailbox: &str,
        flags: Flags,
        date: Option<SystemTime>,
        data: &[u8],
    ) -> io::Result<u32>;

    /// Copy messages to another mailbox.
    fn copy_messages(&self, mailbox: &str, sequence: &str, dest: &str) -> io::Result<Vec<u32>>;

    /// Close a mailbox (expunge and deselect).
    fn close(&self, mailbox: &str) -> io::Result<()>;

    /// Subscribe to a mailbox.
    fn subscribe(&self, mailbox: &str) -> io::Result<bool>;

    /// Unsubscribe from a mailbox.
    fn unsubscribe(&self, mailbox: &str) -> io::Result<bool>;

    /// List subscribed mailboxes.
    fn list_subscribed(&self, reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>>;

    /// Get quota for a mailbox.
    fn get_quota(&self, mailbox: &str) -> io::Result<Option<QuotaInfo>>;

    /// Set quota for a mailbox.
    fn set_quota(&self, mailbox: &str, limits: Vec<(&str, u32)>) -> io::Result<QuotaInfo>;

    /// Check/sync mailbox.
    fn check(&self, mailbox: &str) -> io::Result<()>;
}

// ===========================================================================
// In-Memory Mail Store
// ===========================================================================

/// Simple in-memory mailbox implementation.
pub struct MemoryStore {
    mailboxes: std::sync::Mutex<HashMap<String, Vec<Message>>>,
    users: std::sync::Mutex<HashMap<String, String>>, // username -> token
    subscriptions: std::sync::Mutex<std::collections::HashSet<String>>, // subscribed mailboxes
    quotas: std::sync::Mutex<HashMap<String, Quota>>, // mailbox -> quota info
    next_uid: std::sync::Mutex<u32>,
}

impl MemoryStore {
    pub fn new() -> Self {
        let mut store = Self {
            mailboxes: std::sync::Mutex::new(HashMap::new()),
            users: std::sync::Mutex::new(HashMap::new()),
            subscriptions: std::sync::Mutex::new(std::collections::HashSet::new()),
            quotas: std::sync::Mutex::new(HashMap::new()),
            next_uid: std::sync::Mutex::new(1),
        };
        // Create default INBOX
        store
            .mailboxes
            .lock()
            .unwrap()
            .insert("INBOX".to_string(), Vec::new());
        // Add default user
        store
            .users
            .lock()
            .unwrap()
            .insert("user".to_string(), "pass".to_string());
        store
    }

    pub fn add_user(&self, username: &str, token: &str) {
        self.users
            .lock()
            .unwrap()
            .insert(username.to_string(), token.to_string());
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MailStore for MemoryStore {
    fn authenticate(&self, user: &str, token: &str) -> io::Result<Option<String>> {
        let users = self.users.lock().unwrap();
        if let Some(stored_pass) = users.get(user) {
            if stored_pass == token {
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

    fn fetch(
        &self,
        mailbox: &str,
        sequence: &str,
        attrs: &[FetchAttr],
    ) -> io::Result<Vec<(u32, HashMap<String, String>)>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get(mailbox) {
            let mut results = Vec::new();
            let seq_set = parser::parse_sequence_set(sequence);

            for seq_str in &seq_set {
                // Handle ranges like "1:5" or "*"
                let seq_nums = if seq_str.contains(':') {
                    let parts: Vec<&str> = seq_str.splitn(2, ':').collect();
                    let start = if parts[0] == "*" {
                        msgs.len() as u32
                    } else {
                        parts[0].parse::<u32>().unwrap_or(0)
                    };
                    let end = if parts[1] == "*" {
                        msgs.len() as u32
                    } else {
                        parts[1].parse::<u32>().unwrap_or(0)
                    };
                    let (s, e) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
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
                                    let header_end = msg
                                        .rfc822
                                        .windows(4)
                                        .position(|w| w == b"\r\n\r\n")
                                        .unwrap_or(msg.rfc822.len());
                                    let headers =
                                        String::from_utf8_lossy(&msg.rfc822[..header_end]);
                                    data.insert("RFC822.HEADER".to_string(), headers.into_owned());
                                }
                                FetchAttr::Rfc822Text => {
                                    // Extract body (everything after first blank line)
                                    let header_end = msg
                                        .rfc822
                                        .windows(4)
                                        .position(|w| w == b"\r\n\r\n")
                                        .map(|p| p + 4)
                                        .unwrap_or(0);
                                    let body = String::from_utf8_lossy(&msg.rfc822[header_end..]);
                                    data.insert("RFC822.TEXT".to_string(), body.into_owned());
                                }
                                FetchAttr::Envelope => {
                                    data.insert(
                                        "ENVELOPE".to_string(),
                                        format_envelope_imap(&msg.envelope),
                                    );
                                }
                                FetchAttr::InternalDate => {
                                    // Format as IMAP internal date: DD-Mon-YYYY HH:MM:SS +ZZZZ
                                    let date_str = format_internal_date(msg.internal_date);
                                    data.insert("INTERNALDATE".to_string(), date_str);
                                }
                                FetchAttr::BodySection(section) => {
                                    if section.is_empty() {
                                        // BODY[] = full message
                                        data.insert(
                                            "BODY[]".to_string(),
                                            format!("{{{}}}", msg.size),
                                        );
                                        data.insert(
                                            "__BODY_DATA__".to_string(),
                                            String::from_utf8_lossy(&msg.rfc822).into_owned(),
                                        );
                                    } else if section.to_uppercase() == "HEADER" {
                                        let header_end = msg
                                            .rfc822
                                            .windows(4)
                                            .position(|w| w == b"\r\n\r\n")
                                            .unwrap_or(msg.rfc822.len());
                                        let headers =
                                            String::from_utf8_lossy(&msg.rfc822[..header_end]);
                                        data.insert(
                                            "BODY[HEADER]".to_string(),
                                            headers.into_owned(),
                                        );
                                    } else if section.to_uppercase() == "TEXT" {
                                        let header_end = msg
                                            .rfc822
                                            .windows(4)
                                            .position(|w| w == b"\r\n\r\n")
                                            .map(|p| p + 4)
                                            .unwrap_or(0);
                                        let body =
                                            String::from_utf8_lossy(&msg.rfc822[header_end..]);
                                        data.insert("BODY[TEXT]".to_string(), body.into_owned());
                                    }
                                }
                                FetchAttr::BodyStructure => {
                                    data.insert(
                                        "BODYSTRUCTURE".to_string(),
                                        format_body_structure(msg),
                                    );
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

    fn store(
        &self,
        mailbox: &str,
        sequence: &str,
        action: &StoreAction,
        flags: &[String],
    ) -> io::Result<Vec<u32>> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get_mut(mailbox) {
            let mut updated = Vec::new();
            let seq_set = parser::parse_sequence_set(sequence);

            for seq_str in &seq_set {
                let seq_nums = if seq_str.contains(':') {
                    let parts: Vec<&str> = seq_str.splitn(2, ':').collect();
                    let start = if parts[0] == "*" {
                        msgs.len() as u32
                    } else {
                        parts[0].parse::<u32>().unwrap_or(0)
                    };
                    let end = if parts[1] == "*" {
                        msgs.len() as u32
                    } else {
                        parts[1].parse::<u32>().unwrap_or(0)
                    };
                    let (s, e) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
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
                                msg.flags
                                    .keywords
                                    .retain(|k| !new_flags.keywords.contains(k));
                            }
                            StoreAction::RemoveSilent => {
                                msg.flags.seen &= !new_flags.seen;
                                msg.flags.answered &= !new_flags.answered;
                                msg.flags.flagged &= !new_flags.flagged;
                                msg.flags.deleted &= !new_flags.deleted;
                                msg.flags.draft &= !new_flags.draft;
                                msg.flags
                                    .keywords
                                    .retain(|k| !new_flags.keywords.contains(k));
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

    fn sort(
        &self,
        mailbox: &str,
        keys: &[SearchKey],
        criteria: &[(String, bool)],
    ) -> io::Result<Vec<u32>> {
        let mailboxes = self.mailboxes.lock().unwrap();
        let Some(msgs) = mailboxes.get(mailbox) else {
            return Ok(Vec::new());
        };

        // Filter by search keys
        let mut matching: Vec<&Message> = msgs
            .iter()
            .filter(|m| matches_keys(m, keys, msgs))
            .collect();

        if matching.is_empty() || criteria.is_empty() {
            return Ok(matching.iter().map(|m| m.uid).collect());
        }

        // Sort by criteria (last criterion is primary, like RFC 5256)
        for (criterion, reversed) in criteria.iter().rev() {
            let rev = *reversed;
            match criterion.to_uppercase().as_str() {
                "ARRIVAL" => {
                    matching.sort_by(|a, b| {
                        let ord = a.internal_date.cmp(&b.internal_date);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "DATE" | "SENT" => {
                    matching.sort_by(|a, b| {
                        let ord = a.internal_date.cmp(&b.internal_date);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "SUBJECT" => {
                    matching.sort_by(|a, b| {
                        let subj_a = a.envelope.subject.as_deref().unwrap_or("");
                        let subj_b = b.envelope.subject.as_deref().unwrap_or("");
                        let ord = subj_a.cmp(subj_b);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "FROM" => {
                    matching.sort_by(|a, b| {
                        let from_a = a
                            .envelope
                            .from
                            .first()
                            .and_then(|addr| addr.name.as_deref())
                            .unwrap_or("");
                        let from_b = b
                            .envelope
                            .from
                            .first()
                            .and_then(|addr| addr.name.as_deref())
                            .unwrap_or("");
                        let ord = from_a.cmp(from_b);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "TO" => {
                    matching.sort_by(|a, b| {
                        let to_a = a
                            .envelope
                            .to
                            .first()
                            .and_then(|addr| addr.name.as_deref())
                            .unwrap_or("");
                        let to_b = b
                            .envelope
                            .to
                            .first()
                            .and_then(|addr| addr.name.as_deref())
                            .unwrap_or("");
                        let ord = to_a.cmp(to_b);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "CC" => {
                    matching.sort_by(|a, b| {
                        let cc_a = a
                            .envelope
                            .cc
                            .first()
                            .and_then(|addr| addr.name.as_deref())
                            .unwrap_or("");
                        let cc_b = b
                            .envelope
                            .cc
                            .first()
                            .and_then(|addr| addr.name.as_deref())
                            .unwrap_or("");
                        let ord = cc_a.cmp(cc_b);
                        if rev {
                            ord.reverse()
                        } else {
                            ord
                        }
                    });
                }
                "SIZE" => {
                    matching.sort_by(|a, b| {
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

        Ok(matching.iter().map(|m| m.uid).collect())
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

    fn append(
        &self,
        mailbox: &str,
        flags: Flags,
        date: Option<SystemTime>,
        data: &[u8],
    ) -> io::Result<u32> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        if let Some(msgs) = mailboxes.get_mut(mailbox) {
            let mut next_uid = self.next_uid.lock().unwrap();
            let uid = *next_uid;
            *next_uid += 1;

            let seq = msgs.len() as u32 + 1;
            let msg_date = system_time_to_unix_secs(date.unwrap_or_else(SystemTime::now));
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
                    let start = if parts[0] == "*" {
                        src_msgs.len() as u32
                    } else {
                        parts[0].parse::<u32>().unwrap_or(0)
                    };
                    let end = if parts[1] == "*" {
                        src_msgs.len() as u32
                    } else {
                        parts[1].parse::<u32>().unwrap_or(0)
                    };
                    let (s, e) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
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
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "destination mailbox not found",
                ))
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

    fn subscribe(&self, mailbox: &str) -> io::Result<bool> {
        let mut subs = self.subscriptions.lock().unwrap();
        Ok(subs.insert(mailbox.to_string()))
    }

    fn unsubscribe(&self, mailbox: &str) -> io::Result<bool> {
        let mut subs = self.subscriptions.lock().unwrap();
        Ok(subs.remove(mailbox))
    }

    fn list_subscribed(&self, reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>> {
        let subs = self.subscriptions.lock().unwrap();
        let mailboxes = self.mailboxes.lock().unwrap();
        let mut result = Vec::new();
        for name in subs.iter() {
            if mailboxes.contains_key(name) && name_matches_pattern(name, reference, pattern) {
                let msgs = &mailboxes[name];
                result.push(Mailbox {
                    name: name.clone(),
                    attributes: vec!["\\Subscribed".to_string()],
                    delimiter: Some("/".to_string()),
                    status: Some(MailboxStatus {
                        messages: msgs.len() as u32,
                        recent: msgs.iter().filter(|m| m.flags.recent).count() as u32,
                        uid_next: *self.next_uid.lock().unwrap(),
                        uid_validity: 1,
                        uid_not_stored: 0,
                    }),
                });
            }
        }
        Ok(result)
    }

    fn get_quota(&self, mailbox: &str) -> io::Result<Option<QuotaInfo>> {
        let quotas = self.quotas.lock().unwrap();
        let mailboxes = self.mailboxes.lock().unwrap();
        let msgs = mailboxes.get(mailbox);
        let msg_count = msgs.map(|m| m.len() as u32).unwrap_or(0);
        let storage_used = msgs
            .map(|m| m.iter().map(|m| m.size).sum::<usize>() as u32)
            .unwrap_or(0);

        if let Some(q) = quotas.get(mailbox) {
            Ok(Some(QuotaInfo {
                mailbox: mailbox.to_string(),
                storage_used: storage_used / 1024,
                storage_limit: q.storage_limit,
                message_count: msg_count,
                message_limit: q.message_limit,
            }))
        } else {
            // Default: unlimited quota
            Ok(Some(QuotaInfo {
                mailbox: mailbox.to_string(),
                storage_used: storage_used / 1024,
                storage_limit: u32::MAX,
                message_count: msg_count,
                message_limit: u32::MAX,
            }))
        }
    }

    fn set_quota(&self, mailbox: &str, limits: Vec<(&str, u32)>) -> io::Result<QuotaInfo> {
        let mut quotas = self.quotas.lock().unwrap();
        let mut storage_limit = u32::MAX;
        let mut message_limit = u32::MAX;
        for (key, val) in &limits {
            match *key {
                "STORAGE" => storage_limit = *val,
                "MESSAGES" => message_limit = *val,
                _ => {}
            }
        }
        quotas.insert(
            mailbox.to_string(),
            Quota {
                storage_limit,
                message_limit,
            },
        );
        let mailboxes = self.mailboxes.lock().unwrap();
        let msgs = mailboxes.get(mailbox);
        let msg_count = msgs.map(|m| m.len() as u32).unwrap_or(0);
        let storage_used = msgs
            .map(|m| m.iter().map(|m| m.size).sum::<usize>() as u32)
            .unwrap_or(0);
        drop(mailboxes);

        Ok(QuotaInfo {
            mailbox: mailbox.to_string(),
            storage_used: storage_used / 1024,
            storage_limit,
            message_count: msg_count,
            message_limit,
        })
    }

    fn check(&self, mailbox: &str) -> io::Result<()> {
        // CHECK is a no-op in memory store — data is already consistent
        drop(self.mailboxes.lock().unwrap()); // Verify mailbox exists
        Ok(())
    }
}

// ===========================================================================
// Helper Functions
// ===========================================================================

/// Check if a mailbox name matches an IMAP wildcard pattern (* = any chars, % = any except /).
fn name_matches_pattern(name: &str, _reference: &str, pattern: &str) -> bool {
    if pattern == "*" || pattern == "%" {
        return true;
    }
    // Simple glob matching (no recursive pattern matching for simplicity)
    // Handle leading separator in pattern
    let pat = pattern.trim_start_matches(['"', '\\']);
    if pat == "*" || pat == "%" {
        return true;
    }
    // Check if pattern is a prefix match
    if pat.ends_with('*') || pat.ends_with('%') {
        return name.starts_with(&pat[..pat.len() - 1]);
    }
    name == pat
}

pub fn format_envelope_imap(env: &Envelope) -> String {
    env.format_imap()
}

pub fn system_time_to_unix_secs(t: SystemTime) -> u64 {
    use std::time::UNIX_EPOCH;
    t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

pub fn format_internal_date(t: u64) -> String {
    let total_secs = t as i64;
    let time_secs = total_secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs = time_secs % 60;

    // Approximate date calculation
    let days = total_secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mon = MONTHS.get((month - 1) as usize).copied().unwrap_or("Jan");

    format!(
        "{:02}-{}-{} {:02}:{:02}:{:02} +0000",
        day.min(28),
        mon,
        year,
        hours,
        mins,
        secs
    )
}

fn format_body_structure(msg: &Message) -> String {
    let line_count = msg.rfc822.split(|&b| b == b'\n').count() as u32;
    format!(
        r#"("text" "plain" ("charset" "utf-8") NIL NIL "7bit" {} {})"#,
        msg.size, line_count
    )
}

fn parse_imap_date(s: &str) -> Result<u64, ()> {
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = s
        .split_whitespace()
        .next()
        .unwrap_or(s)
        .split('-')
        .collect();
    if parts.len() >= 3 {
        let day: u32 = parts[0].parse().map_err(|_| ())?;
        let month_str = parts[1];
        let year: u32 = parts[2].parse().map_err(|_| ())?;
        let month = months
            .iter()
            .position(|&m| m.eq_ignore_ascii_case(month_str))
            .ok_or(())? as u32
            + 1;

        let days_from_year = (year - 1970) as u64 * 365 + (year - 1969) as u64 / 4;
        let days_in_months: u64 = match month {
            1 => 0,
            2 => 31,
            3 => 59,
            4 => 90,
            5 => 120,
            6 => 151,
            7 => 181,
            8 => 212,
            9 => 243,
            10 => 273,
            11 => 304,
            12 => 334,
            _ => 0,
        };
        let total_days = days_from_year + days_in_months + (day as u64 - 1);
        Ok(total_days * 86400)
    } else {
        Err(())
    }
}

pub fn parse_envelope_from_rfc822(data: &[u8]) -> Envelope {
    let mut env = Envelope::default();
    let raw = String::from_utf8_lossy(data);

    // First, unfold headers per RFC 5322 §2.2.3: continuation lines start with whitespace
    let mut unfolded = String::with_capacity(raw.len());
    for line in raw.lines() {
        if line.starts_with([' ', '\t']) && !unfolded.is_empty() {
            // Continuation line — append to previous line
            unfolded.push(' ');
            unfolded.push_str(line.trim());
        } else {
            if !unfolded.is_empty() {
                unfolded.push('\n');
            }
            unfolded.push_str(line);
        }
    }

    for line in unfolded.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let value = value.trim();
            match key.to_uppercase().as_str() {
                "SUBJECT" => env.subject = Some(unfold_encoded_word(value)),
                "DATE" => env.date = Some(value.to_string()),
                "MESSAGE-ID" => {
                    let id = value.trim_matches(|c| c == '<' || c == '>' || c == ' ');
                    if !id.is_empty() {
                        env.message_id = Some(id.to_string());
                    }
                }
                "IN-REPLY-TO" => {
                    let id = value.trim_matches(|c| c == '<' || c == '>' || c == ' ');
                    if !id.is_empty() {
                        env.in_reply_to = Some(id.to_string());
                    }
                }
                "FROM" => {
                    for addr in parse_imap_address_list(value) {
                        env.from.push(addr);
                    }
                }
                "SENDER" => {
                    if let Some(addr) = parse_imap_address_first(value) {
                        env.sender = Some(addr);
                    }
                }
                "REPLY-TO" => {
                    if let Some(addr) = parse_imap_address_first(value) {
                        env.reply_to = Some(addr);
                    }
                }
                "TO" => {
                    for addr in parse_imap_address_list(value) {
                        env.to.push(addr);
                    }
                }
                "CC" => {
                    for addr in parse_imap_address_list(value) {
                        env.cc.push(addr);
                    }
                }
                "BCC" => {
                    for addr in parse_imap_address_list(value) {
                        env.bcc.push(addr);
                    }
                }
                _ => {}
            }
        }
    }
    env
}

/// Decode RFC 2047 encoded-words in a header value.
/// Handles =?charset?Q?...?= and =?charset?B?...?=
fn unfold_encoded_word(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut remaining = s;

    while let Some(start) = remaining.find("=?") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + 2..];

        if let Some(end) = remaining.find("?=") {
            let encoded = &remaining[..end];
            remaining = &remaining[end + 2..];

            // Parse: charset?encoding?value
            let parts: Vec<&str> = encoded.splitn(3, '?').collect();
            if parts.len() == 3 {
                let encoding = parts[1].to_uppercase();
                let value = parts[2];

                if encoding == "Q" {
                    // Decode Q-encoded: _ for space, =XX for hex
                    let mut decoded = Vec::new();
                    let mut i = 0;
                    let bytes = value.as_bytes();
                    while i < bytes.len() {
                        if bytes[i] == b'_' {
                            decoded.push(b' ');
                            i += 1;
                        } else if bytes[i] == b'=' && i + 2 < bytes.len() {
                            if let Some(val) = edgerun_encoding::hex::parse_hex_int::<u8>(
                                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("00"),
                            ) {
                                decoded.push(val);
                            }
                            i += 3;
                        } else {
                            decoded.push(bytes[i]);
                            i += 1;
                        }
                    }
                    result.push_str(&String::from_utf8_lossy(&decoded));
                } else if encoding == "B" {
                    // B encoding is base64
                    if let Ok(decoded) = base64_decode(value) {
                        result.push_str(&String::from_utf8_lossy(&decoded));
                    } else {
                        result.push_str(value);
                    }
                } else {
                    // Unknown encoding, keep original
                    result.push_str(&format!("?={}", encoded));
                }
            } else {
                result.push_str("=?");
            }
        } else {
            // No closing ?= found, stop
            result.push_str("=?");
            result.push_str(remaining);
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Parse a list of email addresses from a header value (From, To, Cc, etc.)
fn parse_imap_address_list(s: &str) -> Vec<Address> {
    let mut addresses = Vec::new();

    // Remove group syntax: "group: addr1, addr2;"
    let s = s.trim();
    if s.contains(':') && s.ends_with(';') {
        // Group syntax — extract addresses between : and ;
        if let Some(colon) = s.find(':') {
            let inner = &s[colon + 1..s.len() - 1];
            for part in inner.split(',') {
                if let Some(addr) = parse_imap_address(part.trim()) {
                    addresses.push(addr);
                }
            }
        }
        return addresses;
    }

    // Simple comma-separated addresses
    for part in s.split(',') {
        if let Some(addr) = parse_imap_address(part.trim()) {
            addresses.push(addr);
        }
    }
    addresses
}

/// Parse just the first address from a header (for Sender).
fn parse_imap_address_first(s: &str) -> Option<Address> {
    parse_imap_address_list(s).into_iter().next()
}

fn parse_imap_address(s: &str) -> Option<Address> {
    if s.is_empty() || s == "NIL" {
        return None;
    }

    // Remove display name and quoted-string prefix
    let s = s.trim();

    // Handle: "Display Name" <email@domain>
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

    // Handle: email@domain (bare address)
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
    edgerun_encoding::base64::standard_decode(s).map_err(|_| ())
}

/// Check if a message matches search keys.
fn matches_keys(msg: &Message, keys: &[SearchKey], all_msgs: &[Message]) -> bool {
    for key in keys {
        let matched = match key {
            SearchKey::All => true,
            SearchKey::Answered => msg.flags.answered,
            SearchKey::Deleted => msg.flags.deleted,
            SearchKey::Undeleted => !msg.flags.deleted,
            SearchKey::Draft => msg.flags.draft,
            SearchKey::Flagged => msg.flags.flagged,
            SearchKey::Recent => msg.flags.recent,
            SearchKey::New => !msg.flags.seen,
            SearchKey::Old => msg.flags.seen,
            SearchKey::Seen => msg.flags.seen,
            SearchKey::Unseen => !msg.flags.seen,
            SearchKey::Not(sub_key) => !matches_keys(msg, &[(**sub_key).clone()], all_msgs),
            SearchKey::And(k1, k2) => {
                matches_keys(msg, &[(**k1).clone()], all_msgs)
                    && matches_keys(msg, &[(**k2).clone()], all_msgs)
            }
            SearchKey::Or(k1, k2) => {
                matches_keys(msg, &[(**k1).clone()], all_msgs)
                    || matches_keys(msg, &[(**k2).clone()], all_msgs)
            }
            SearchKey::Smaller(n) => msg.size < *n as usize,
            SearchKey::Larger(n) => msg.size > *n as usize,
            SearchKey::Subject(sub) => msg
                .envelope
                .subject
                .as_ref()
                .map(|s| s.to_lowercase().contains(&sub.to_lowercase()))
                .unwrap_or(false),
            SearchKey::From(addr) => msg.envelope.from.iter().any(|a| {
                a.mailbox
                    .as_ref()
                    .map(|m| m.to_lowercase().contains(&addr.to_lowercase()))
                    .unwrap_or(false)
                    || a.host
                        .as_ref()
                        .map(|h| h.to_lowercase().contains(&addr.to_lowercase()))
                        .unwrap_or(false)
                    || a.name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&addr.to_lowercase()))
                        .unwrap_or(false)
            }),
            SearchKey::To(addr) => msg.envelope.to.iter().any(|a| {
                a.mailbox
                    .as_ref()
                    .map(|m| m.to_lowercase().contains(&addr.to_lowercase()))
                    .unwrap_or(false)
                    || a.host
                        .as_ref()
                        .map(|h| h.to_lowercase().contains(&addr.to_lowercase()))
                        .unwrap_or(false)
            }),
            SearchKey::Body(text) => {
                let body = String::from_utf8_lossy(&msg.rfc822).to_lowercase();
                body.contains(&text.to_lowercase())
            }
            SearchKey::Text(text) => {
                // Text matches subject, from, to, or body
                let text_lower = text.to_lowercase();
                msg.envelope
                    .subject
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(&text_lower))
                    .unwrap_or(false)
                    || msg.envelope.from.iter().any(|a| {
                        a.mailbox
                            .as_ref()
                            .map(|m| m.to_lowercase().contains(&text_lower))
                            .unwrap_or(false)
                    })
                    || String::from_utf8_lossy(&msg.rfc822)
                        .to_lowercase()
                        .contains(&text_lower)
            }
            SearchKey::SeqSet(seq_str) => {
                // Match sequence numbers
                let seqs = crate::imap::parser::parse_sequence_set(seq_str);
                for s in &seqs {
                    if s.contains(':') {
                        let parts: Vec<&str> = s.splitn(2, ':').collect();
                        let start: u32 = parts[0].parse().unwrap_or(0);
                        let end: u32 = parts[1].parse().unwrap_or(all_msgs.len() as u32);
                        let (lo, hi) = if start <= end {
                            (start, end)
                        } else {
                            (end, start)
                        };
                        if msg.seq >= lo && msg.seq <= hi {
                            return true;
                        }
                    } else if s == "*" {
                        if msg.seq == all_msgs.len() as u32 {
                            return true;
                        }
                    } else if let Ok(n) = s.parse::<u32>() {
                        if msg.seq == n {
                            return true;
                        }
                    }
                }
                false
            }
            SearchKey::UidSet(uid_str) => {
                // Match UIDs
                let uids = crate::imap::parser::parse_sequence_set(uid_str);
                for u in &uids {
                    if u.contains(':') {
                        let parts: Vec<&str> = u.splitn(2, ':').collect();
                        let start: u32 = parts[0].parse().unwrap_or(0);
                        let end: u32 = parts[1].parse().unwrap_or(u32::MAX);
                        let (lo, hi) = if start <= end {
                            (start, end)
                        } else {
                            (end, start)
                        };
                        if msg.uid >= lo && msg.uid <= hi {
                            return true;
                        }
                    } else if u == "*" {
                        return false; // Can't match "*" as UID without knowing last UID
                    } else if let Ok(n) = u.parse::<u32>() {
                        if msg.uid == n {
                            return true;
                        }
                    }
                }
                false
            }
            // Date searches - simplified: check internal_date
            SearchKey::Before(date_str) | SearchKey::SentBefore(date_str) => {
                if let Ok(target) = parse_imap_date(date_str) {
                    msg.internal_date < target
                } else {
                    true
                }
            }
            SearchKey::Since(date_str) | SearchKey::SentSince(date_str) => {
                if let Ok(target) = parse_imap_date(date_str) {
                    msg.internal_date >= target
                } else {
                    true
                }
            }
            SearchKey::On(date_str) | SearchKey::SentOn(date_str) => {
                if let Ok(target) = parse_imap_date(date_str) {
                    let next_day = target.saturating_add(86400);
                    msg.internal_date >= target && msg.internal_date < next_day
                } else {
                    true
                }
            }
            SearchKey::Header(name, value) => {
                let headers = String::from_utf8_lossy(&msg.rfc822);
                for line in headers.lines() {
                    if let Some((h, v)) = line.split_once(':') {
                        if h.trim().eq_ignore_ascii_case(name)
                            && v.trim().to_lowercase().contains(&value.to_lowercase())
                        {
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
    pub tls_cert: Option<CertificateAndKey>,
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
    tls_cert: Option<CertificateAndKey>,
    imaps: bool,
    /// Command middleware layers. Composed with the store at runtime
    /// so middleware can capture mutable session state.
    command_middleware: Vec<Arc<dyn CommandMiddleware<ImapCommand, ImapResponse>>>,
    /// Connection interceptor (IP filter, rate limit, etc.).
    connection_interceptor: Option<Arc<dyn ConnectionInterceptor>>,
}

impl ImapServer {
    /// Create a new IMAP server with the given config and mail store.
    pub fn new(config: ImapServerConfig) -> io::Result<Self> {
        let listener =
            Arc::new(AsyncTcpListener::bind(&config.bind_addr).map_err(crate::rt::bare_io)?);
        let store = Arc::new(MemoryStore::new());
        Ok(Self {
            listener,
            store,
            domain: config.domain_name,
            #[cfg(feature = "tls")]
            tls_cert: config.tls_cert,
            imaps: config.imaps,
            command_middleware: Vec::new(),
            connection_interceptor: None,
        })
    }

    /// Add a command middleware layer.
    ///
    /// Middleware runs in order: first added = outermost (sees command first).
    pub fn with_command_middleware<M: CommandMiddleware<ImapCommand, ImapResponse>>(
        mut self,
        mw: M,
    ) -> Self {
        self.command_middleware.push(Arc::new(mw));
        self
    }

    /// Set the connection interceptor.
    ///
    /// The interceptor runs on every new TCP connection before protocol
    /// parsing. It can reject connections (IP filter, rate limit) or
    /// pass them through to the IMAP handler.
    pub fn with_connection_interceptor(
        mut self,
        interceptor: Arc<dyn ConnectionInterceptor>,
    ) -> Self {
        self.connection_interceptor = Some(interceptor);
        self
    }

    /// Create a server with a custom mail store.
    pub fn with_store(config: ImapServerConfig, store: Arc<dyn MailStore>) -> io::Result<Self> {
        let listener =
            Arc::new(AsyncTcpListener::bind(&config.bind_addr).map_err(crate::rt::bare_io)?);
        Ok(Self {
            listener,
            store,
            domain: config.domain_name,
            #[cfg(feature = "tls")]
            tls_cert: config.tls_cert,
            imaps: config.imaps,
            command_middleware: Vec::new(),
            connection_interceptor: None,
        })
    }

    /// Get the local address the server is bound to.
    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {
        self.listener.local_addr().map_err(crate::rt::bare_io)
    }

    /// Run the server until shutdown is cancelled.
    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        edgerun_log::info!(
            "edgerun-imap: server listening on {} (IMAPS: {})",
            self.listener.local_addr().map_err(crate::rt::bare_io)?,
            self.imaps
        );

        while !shutdown.is_cancelled() {
            match self.listener.accept().await {
                Ok((stream, peer)) => {
                    // Connection interceptor (IP filter, rate limit, etc.)
                    if let Some(ref interceptor) = self.connection_interceptor {
                        let interceptor = Arc::clone(interceptor);
                        let peer_addr = peer;
                        let stream_ref = Arc::clone(&stream);
                        match interceptor.intercept(peer_addr, stream_ref).await {
                            Ok(()) => {}
                            Err(e) => {
                                edgerun_log::info!(
                                    "edgerun-imap: connection from {} rejected: {}",
                                    peer,
                                    e
                                );
                                continue;
                            }
                        }
                    }

                    let store = Arc::clone(&self.store);
                    let domain = self.domain.clone();
                    #[cfg(feature = "tls")]
                    let tls_cert = self.tls_cert.clone();
                    let imaps = self.imaps;
                    let command_middleware = self.command_middleware.clone();
                    crate::rt::spawn(async move {
                        if let Err(e) = handle_connection(
                            stream,
                            peer,
                            store,
                            domain,
                            #[cfg(feature = "tls")]
                            tls_cert,
                            imaps,
                            command_middleware,
                        )
                        .await
                        {
                            edgerun_log::warn!(
                                "edgerun-imap: connection error from {}: {}",
                                peer,
                                e
                            );
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::warn!("edgerun-imap: accept error: {}", e);
                    crate::rt::sleep(std::time::Duration::from_millis(10)).await;
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
    #[cfg(feature = "tls")] tls_cert: Option<CertificateAndKey>,
    imaps: bool,
    command_middleware: Vec<Arc<dyn CommandMiddleware<ImapCommand, ImapResponse>>>,
) -> io::Result<()> {
    edgerun_log::info!("edgerun-imap: connection from {}", peer);

    // Unwrap the Arc — we need the owned AsyncTcpStream for the transport.
    let stream = match Arc::try_unwrap(stream) {
        Ok(s) => s,
        Err(_) => {
            edgerun_log::error!("edgerun-imap: non-exclusive Arc for connection");
            return Err(io::Error::other("connection reference error"));
        }
    };

    // IMAPS: wrap in TLS immediately
    #[cfg(feature = "tls")]
    let mut transport = if imaps {
        if let Some(ref cert) = tls_cert {
            edgerun_log::debug!("edgerun-imap: IMAPS TLS handshake for {}", peer);
            let mut tls_stream = AsyncTlsServerStream::new(stream);
            tls_stream
                .handshake(cert)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
            ImapTransport::Tls(tls_stream)
        } else {
            ImapTransport::Plain(stream)
        }
    } else {
        ImapTransport::Plain(stream)
    };

    #[cfg(not(feature = "tls"))]
    let mut transport = ImapTransport::Plain(stream);
    let _ = imaps;

    let mut transport = BufferedImapTransport::new(transport);

    // Send greeting
    let greeting = parser::format_greeting(CAPABILITIES);
    transport.write_all(greeting.as_bytes()).await?;
    transport.flush().await?;

    // Session state
    let mut state = ImapState::NotAuthenticated;
    let mut current_mailbox: Option<String> = None;
    let mut authenticated_user: Option<String> = None;

    // Command loop — read lines directly from transport
    loop {
        let line = match transport.read_line().await {
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

        // Parse command
        let (tag, command_name, args) = match parser::parse_command_line(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = ImapResponse::bad("?", &format!("Parse error: {}", e));
                write_response(&mut transport, &resp).await?;
                continue;
            }
        };

        // Parse into typed command
        let cmd = match ImapCommand::parse(&tag, &command_name, &args, Some(&line)) {
            Ok(cmd) => cmd,
            Err(e) => {
                let resp = ImapResponse::bad(&tag, &format!("Error: {}", e));
                write_response(&mut transport, &resp).await?;
                continue;
            }
        };

        // Handle STARTTLS: upgrade the transport in place
        if let ImapCommand::Starttls = &cmd {
            #[cfg(feature = "tls")]
            {
                if transport.is_tls() {
                    let resp = ImapResponse::no(&tag, "TLS already active");
                    write_response(&mut transport, &resp).await?;
                    continue;
                }
                if tls_cert.is_none() {
                    let resp = ImapResponse::no(&tag, "TLS not configured");
                    write_response(&mut transport, &resp).await?;
                    continue;
                }
                // Send OK response, then upgrade
                let resp = ImapResponse::ok(&tag, "Begin TLS negotiation now");
                write_response(&mut transport, &resp).await?;

                let cert = tls_cert.as_ref().unwrap();
                match transport.upgrade_tls(cert).await {
                    Ok(()) => {
                        edgerun_log::info!(
                            "edgerun-imap: STARTTLS handshake complete for {}",
                            peer
                        );
                    }
                    Err(e) => {
                        edgerun_log::warn!(
                            "edgerun-imap: STARTTLS handshake failed for {}: {}",
                            peer,
                            e
                        );
                        return Err(e);
                    }
                }
                continue;
            }
            #[cfg(not(feature = "tls"))]
            {
                let resp = ImapResponse::no(&tag, "TLS not supported");
                write_response(&mut transport, &resp).await?;
                continue;
            }
        }

        // ── Middleware pre-filter (if configured) ──────────────────
        if !command_middleware.is_empty() {
            let session = SessionExtensions::new();
            session
                .insert(ImapConnState {
                    state,
                    mailbox: current_mailbox.clone(),
                    authenticated_user: authenticated_user.clone(),
                })
                .await;

            let mut blocked = false;
            for mw in &command_middleware {
                let mw = Arc::clone(mw);
                let cmd_for_mw = cmd.clone();
                let session_for_mw = session.clone();
                let next = NextCommand::new(|_cmd, _session| {
                    Box::pin(async move { Ok(MwControlFlow::Continue) })
                });

                match mw.handle(cmd_for_mw, session_for_mw, next).await {
                    Ok(MwControlFlow::Respond(resp)) => {
                        write_response(&mut transport, &resp).await?;
                        blocked = true;
                        break;
                    }
                    Ok(MwControlFlow::Continue) => {
                        if let Some(conn) = session.get::<ImapConnState>().await {
                            state = conn.state;
                            current_mailbox = conn.mailbox;
                            authenticated_user = conn.authenticated_user;
                        }
                    }
                    Err(e) => {
                        let resp = ImapResponse::bad(&tag, &format!("Error: {}", e));
                        write_response(&mut transport, &resp).await?;
                        blocked = true;
                        break;
                    }
                }
            }
            if blocked {
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
            &mut transport,
        )
        .await?;

        write_response(&mut transport, &response).await?;

        if state == ImapState::Logout {
            edgerun_log::info!("edgerun-imap: client {} logged out", peer);
            return Ok(());
        }
    }
}

/// Write a response to the transport.
async fn write_response(
    transport: &mut BufferedImapTransport,
    resp: &ImapResponse,
) -> io::Result<()> {
    let wire = resp.to_wire();
    transport.write_all(wire.as_bytes()).await?;
    transport.flush().await?;
    Ok(())
}

fn scoped_mailbox(authenticated_user: Option<&str>, mailbox: &str) -> String {
    match authenticated_user {
        Some(user) if mailbox.eq_ignore_ascii_case("INBOX") => format!("{user}/INBOX"),
        _ => mailbox.to_string(),
    }
}

// ===========================================================================
// Command Dispatcher
// ===========================================================================

async fn dispatch_command(
    cmd: &ImapCommand,
    tag: &str,
    state: &mut ImapState,
    current_mailbox: &mut Option<String>,
    authenticated_user: &mut Option<String>,
    store: &Arc<dyn MailStore>,
    domain: &str,
    transport: &mut BufferedImapTransport,
) -> io::Result<ImapResponse> {
    match cmd {
        ImapCommand::Capability => {
            // Send untagged capability list
            let cap_resp = parser::format_capability(CAPABILITIES);
            transport.write_all(cap_resp.as_bytes()).await?;
            transport.flush().await?;

            Ok(ImapResponse::ok(tag, "CAPABILITY completed"))
        }

        ImapCommand::Noop => Ok(ImapResponse::ok(tag, "NOOP completed")),

        ImapCommand::Logout => {
            *state = ImapState::Logout;
            transport
                .write_all(parser::format_untagged("BYE Logging out").as_bytes())
                .await?;
            transport.flush().await?;

            Ok(ImapResponse::ok(tag, "LOGOUT completed"))
        }

        ImapCommand::Authenticate { mechanism } => {
            if *state != ImapState::NotAuthenticated {
                return Ok(ImapResponse::no(tag, "Already authenticated"));
            }

            let mech = mechanism.to_uppercase();

            if mech == "PLAIN" || mech == "LOGIN" {
                Ok(ImapResponse::no(tag, "AUTHENTICATE mechanism disabled"))
            } else {
                Ok(ImapResponse::no(
                    tag,
                    &format!("Unsupported mechanism: {}", mechanism),
                ))
            }
        }

        ImapCommand::Select { mailbox } => {
            if *state != ImapState::Authenticated {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            let store_mailbox = scoped_mailbox(authenticated_user.as_deref(), mailbox);

            match store.select(&store_mailbox) {
                Ok(Some(mb)) => {
                    *state = ImapState::Selected;
                    *current_mailbox = Some(store_mailbox);

                    // Send mailbox status
                    if let Some(status) = &mb.status {
                        transport
                            .write_all(parser::format_exists(status.messages).as_bytes())
                            .await?;
                        transport
                            .write_all(parser::format_recent(status.recent).as_bytes())
                            .await?;

                        let flags = ["\\Seen", "\\Answered", "\\Flagged", "\\Deleted", "\\Draft"];
                        transport
                            .write_all(parser::format_flags(&flags).as_bytes())
                            .await?;

                        transport
                            .write_all(parser::format_uid_validity(status.uid_validity).as_bytes())
                            .await?;
                        transport
                            .write_all(parser::format_uid_next(status.uid_next).as_bytes())
                            .await?;
                    }

                    transport.flush().await?;

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
            let store_mailbox = scoped_mailbox(authenticated_user.as_deref(), mailbox);

            match store.select(&store_mailbox) {
                Ok(Some(mb)) => {
                    *state = ImapState::Selected;
                    *current_mailbox = Some(store_mailbox);

                    if let Some(status) = &mb.status {
                        transport
                            .write_all(parser::format_exists(status.messages).as_bytes())
                            .await?;
                        transport
                            .write_all(parser::format_recent(status.recent).as_bytes())
                            .await?;
                        let flags = ["\\Seen", "\\Answered", "\\Flagged", "\\Deleted", "\\Draft"];
                        transport
                            .write_all(parser::format_flags(&flags).as_bytes())
                            .await?;
                        transport
                            .write_all(parser::format_uid_validity(status.uid_validity).as_bytes())
                            .await?;
                        transport
                            .write_all(parser::format_uid_next(status.uid_next).as_bytes())
                            .await?;
                    }
                    transport.flush().await?;

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
                    for mb in &mailboxes {
                        let attrs: Vec<&str> = mb.attributes.iter().map(|s| s.as_str()).collect();
                        let resp = parser::format_list(
                            &attrs,
                            mb.delimiter.as_deref().unwrap_or("/"),
                            &mb.name,
                        );
                        transport.write_all(resp.as_bytes()).await?;
                    }
                    transport.flush().await?;

                    Ok(ImapResponse::ok(tag, "LIST completed"))
                }
                Err(e) => Ok(ImapResponse::no(tag, &format!("LIST failed: {}", e))),
            }
        }

        ImapCommand::Fetch {
            sequence,
            attributes,
        } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                match store.fetch(mailbox, sequence, attributes) {
                    Ok(results) => {
                        for (uid, data) in &results {
                            let parts: Vec<String> =
                                data.iter().map(|(k, v)| format!("{} {}", k, v)).collect();
                            let resp = parser::format_fetch(*uid, &parts.join(" "));
                            transport.write_all(resp.as_bytes()).await?;
                        }
                        transport.flush().await?;

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
                        transport
                            .write_all(parser::format_search(&ids).as_bytes())
                            .await?;
                        transport.flush().await?;

                        Ok(ImapResponse::ok(tag, "SEARCH completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("SEARCH failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Create { mailbox } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
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
                            "UIDVALIDITY" => {
                                parts.push(format!("UIDVALIDITY {}", status.uid_validity))
                            }
                            _ => {}
                        }
                    }
                    transport
                        .write_all(
                            parser::format_untagged(&format!(
                                "STATUS {} ({})",
                                mailbox,
                                parts.join(" ")
                            ))
                            .as_bytes(),
                        )
                        .await?;
                    transport.flush().await?;

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
                        for seq in &removed {
                            transport
                                .write_all(parser::format_expunge(*seq).as_bytes())
                                .await?;
                        }
                        transport.flush().await?;

                        Ok(ImapResponse::ok(tag, "EXPUNGE completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("EXPUNGE failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Store {
            sequence,
            action,
            flags,
        } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                match store.store(mailbox, sequence, action, flags) {
                    Ok(updated) => {
                        for uid in &updated {
                            // Send FETCH response with updated flags
                            let resp = parser::format_fetch(*uid, "FLAGS ()");
                            transport.write_all(resp.as_bytes()).await?;
                        }
                        transport.flush().await?;

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
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            match store.subscribe(mailbox) {
                Ok(true) => Ok(ImapResponse::ok(tag, "SUBSCRIBE completed")),
                Ok(false) => Ok(ImapResponse::no(tag, "Already subscribed")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("SUBSCRIBE failed: {}", e))),
            }
        }

        ImapCommand::Unsubscribe { mailbox } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            match store.unsubscribe(mailbox) {
                Ok(true) => Ok(ImapResponse::ok(tag, "UNSUBSCRIBE completed")),
                Ok(false) => Ok(ImapResponse::no(tag, "Not subscribed")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("UNSUBSCRIBE failed: {}", e))),
            }
        }

        ImapCommand::Lsub { reference, pattern } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            match store.list_subscribed(reference, pattern) {
                Ok(mailboxes) => {
                    for mb in &mailboxes {
                        let attrs = if mb.attributes.is_empty() {
                            vec!["\\Noselect"]
                        } else {
                            mb.attributes.iter().map(|s| s.as_str()).collect()
                        };
                        let resp = parser::format_list(
                            &attrs,
                            mb.delimiter.as_deref().unwrap_or("/"),
                            &mb.name,
                        );
                        transport.write_all(resp.as_bytes()).await?;
                    }
                    transport.flush().await?;

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
                        edgerun_log::debug!(
                            "edgerun-imap: COPY {} messages to {}",
                            uids.len(),
                            mailbox
                        );
                        Ok(ImapResponse::ok(tag, "COPY completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("COPY failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Append {
            mailbox,
            flags,
            date,
            literal_size,
        } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }

            // Send continuation for literal data
            {
                transport.write_all(b"+ Ready for literal data\r\n").await?;
                transport.flush().await?;
            }

            // Read the literal message data
            let data = read_imap_exact(transport, *literal_size).await?;

            // Consume trailing \r\n after literal data (it's a blank line)
            transport.read_line().await?;

            // Parse flags if provided
            let msg_flags = flags
                .as_ref()
                .map(|f| Flags::parse(&f.join(" ")))
                .unwrap_or_else(Flags::new);

            // Parse date if provided
            let msg_date = date.as_ref().map(|_| SystemTime::now()); // Simplified

            match store.append(mailbox, msg_flags, msg_date, &data) {
                Ok(uid) => Ok(ImapResponse::ok(
                    tag,
                    &format!("APPEND completed [UIDNEXT {}]", uid + 1),
                )),
                Err(e) => Ok(ImapResponse::no(tag, &format!("APPEND failed: {}", e))),
            }
        }

        ImapCommand::Check => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            if let Some(ref mailbox) = current_mailbox {
                match store.check(mailbox) {
                    Ok(()) => Ok(ImapResponse::ok(tag, "CHECK completed")),
                    Err(e) => Ok(ImapResponse::no(tag, &format!("CHECK failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Uid { command } => {
            // UID commands are dispatched to the inner command
            // The inner command has already been parsed with UID context
            Box::pin(dispatch_command(
                command,
                tag,
                state,
                current_mailbox,
                authenticated_user,
                store,
                domain,
                transport,
            ))
            .await
        }

        ImapCommand::Idle => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }
            // IDLE - send continuation, then wait for DONE from client
            {
                transport.write_all(b"+ idling\r\n").await?;
                transport.flush().await?;
            }

            // Read lines until we get DONE
            loop {
                let line = match transport.read_line().await {
                    Ok(Some(l)) => l,
                    Ok(None) => {
                        edgerun_log::info!("edgerun-imap: client disconnected during IDLE");
                        return Ok(ImapResponse::ok(tag, "IDLE terminated"));
                    }
                    Err(e) => {
                        edgerun_log::warn!("edgerun-imap: read error during IDLE: {}", e);
                        return Err(e);
                    }
                };

                if line.to_uppercase() == "DONE" {
                    // Send any pending EXISTS updates
                    if let Some(ref mailbox) = current_mailbox {
                        let msg_count = store
                            .status(mailbox)
                            .ok()
                            .flatten()
                            .map(|s| s.messages)
                            .unwrap_or(0);
                        transport
                            .write_all(
                                parser::format_untagged(&format!("EXISTS {}", msg_count))
                                    .as_bytes(),
                            )
                            .await?;
                        transport.flush().await?;
                    }
                    return Ok(ImapResponse::ok(tag, "IDLE completed"));
                }
                // Ignore any other input during IDLE per RFC 2177
            }
        }

        ImapCommand::Done => {
            // DONE outside IDLE context is an error
            Ok(ImapResponse::bad(tag, "DONE only valid during IDLE"))
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
            if !enabled.is_empty() {
                transport
                    .write_all(
                        parser::format_untagged(&format!("ENABLED {}", enabled.join(" ")))
                            .as_bytes(),
                    )
                    .await?;
            }
            transport.flush().await?;

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
                        let _ = store.store(
                            src_mailbox,
                            sequence,
                            &StoreAction::Add,
                            &["\\Deleted".to_string()],
                        );
                        let _ = store.expunge(src_mailbox);
                        edgerun_log::debug!(
                            "edgerun-imap: MOVE {} messages to {}",
                            uids.len(),
                            mailbox
                        );
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
                        let _ = store.store(
                            src_mailbox,
                            sequence,
                            &StoreAction::Add,
                            &["\\Deleted".to_string()],
                        );
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
            transport
                .write_all(parser::format_untagged(r#"NAMESPACE (("" "/")) NIL NIL"#).as_bytes())
                .await?;
            transport.flush().await?;

            Ok(ImapResponse::ok(tag, "NAMESPACE completed"))
        }

        ImapCommand::Quota { mailbox } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            match store.get_quota(mailbox) {
                Ok(Some(qi)) => {
                    transport
                        .write_all(
                            parser::format_untagged(&format!(
                                r#"QUOTA "{}" (STORAGE {} {} MESSAGES {} {})"#,
                                qi.mailbox,
                                qi.storage_used,
                                qi.storage_limit,
                                qi.message_count,
                                qi.message_limit
                            ))
                            .as_bytes(),
                        )
                        .await?;
                    transport.flush().await?;

                    Ok(ImapResponse::ok(tag, "QUOTA completed"))
                }
                Ok(None) => Ok(ImapResponse::no(tag, "Mailbox not found")),
                Err(e) => Ok(ImapResponse::no(tag, &format!("QUOTA failed: {}", e))),
            }
        }

        ImapCommand::SetQuota { mailbox, limits } => {
            if *state != ImapState::Authenticated && *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "Not authenticated"));
            }
            match store.set_quota(
                mailbox,
                limits.iter().map(|(k, v)| (k.as_str(), *v)).collect(),
            ) {
                Ok(qi) => {
                    transport
                        .write_all(
                            parser::format_untagged(&format!(
                                r#"QUOTA "{}" (STORAGE {} {} MESSAGES {} {})"#,
                                qi.mailbox,
                                qi.storage_used,
                                qi.storage_limit,
                                qi.message_count,
                                qi.message_limit
                            ))
                            .as_bytes(),
                        )
                        .await?;
                    transport.flush().await?;

                    Ok(ImapResponse::ok(tag, "SETQUOTA completed"))
                }
                Err(e) => Ok(ImapResponse::no(tag, &format!("SETQUOTA failed: {}", e))),
            }
        }

        ImapCommand::Sort {
            sort_criteria,
            charset,
            search_criteria,
        } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                let keys = parse_search_keys_simple(search_criteria);
                // Parse sort criteria with optional REVERSE prefix
                let parsed_criteria: Vec<(String, bool)> = sort_criteria
                    .iter()
                    .map(|c| {
                        let upper = c.to_uppercase();
                        if let Some(stripped) = upper.strip_prefix("REVERSE ") {
                            (stripped.to_string(), true)
                        } else {
                            (upper, false)
                        }
                    })
                    .collect();

                match store.sort(mailbox, &keys, &parsed_criteria) {
                    Ok(ids) => {
                        let id_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                        transport
                            .write_all(
                                parser::format_untagged(&format!("SORT {}", id_str.join(" ")))
                                    .as_bytes(),
                            )
                            .await?;
                        transport.flush().await?;

                        Ok(ImapResponse::ok(tag, "SORT completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("SORT failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Thread {
            algorithm,
            charset,
            search_criteria,
        } => {
            if *state != ImapState::Selected {
                return Ok(ImapResponse::no(tag, "No mailbox selected"));
            }

            if let Some(ref mailbox) = current_mailbox {
                let keys = parse_search_keys_simple(search_criteria);
                match store.search(mailbox, &keys) {
                    Ok(ids) => {
                        // Simple threading: group by In-Reply-To / References
                        // For now, return each message as its own thread
                        let thread_str: Vec<String> =
                            ids.iter().map(|i| format!("({})", i)).collect();
                        transport
                            .write_all(
                                parser::format_untagged(&format!(
                                    "THREAD ({} {})",
                                    algorithm.to_uppercase(),
                                    thread_str.join(") (")
                                ))
                                .as_bytes(),
                            )
                            .await?;
                        transport.flush().await?;

                        Ok(ImapResponse::ok(tag, "THREAD completed"))
                    }
                    Err(e) => Ok(ImapResponse::no(tag, &format!("THREAD failed: {}", e))),
                }
            } else {
                Ok(ImapResponse::no(tag, "No mailbox selected"))
            }
        }

        ImapCommand::Id { params } => {
            edgerun_log::debug!("edgerun-imap: ID params: {:?}", params);
            transport.write_all(parser::format_untagged(
                r#"ID ("name" "edgerun-imap" "version" "0.1.0" "os" "linux" "os-version" "x86_64" "vendor" "edgerun")"#
            ).as_bytes()).await?;
            transport.flush().await?;

            Ok(ImapResponse::ok(tag, "ID completed"))
        }

        _ => Ok(ImapResponse::no(
            tag,
            "Command not implemented in current state",
        )),
    }
}

/// Parse simple search criteria strings into SearchKey vec.
fn parse_search_keys_simple(criteria: &[String]) -> Vec<crate::imap::types::SearchKey> {
    let mut keys = Vec::new();
    for c in criteria {
        match c.to_uppercase().as_str() {
            "ALL" => keys.push(crate::imap::types::SearchKey::All),
            "ANSWERED" => keys.push(crate::imap::types::SearchKey::Answered),
            "DELETED" => keys.push(crate::imap::types::SearchKey::Deleted),
            "DRAFT" => keys.push(crate::imap::types::SearchKey::Draft),
            "FLAGGED" => keys.push(crate::imap::types::SearchKey::Flagged),
            "RECENT" => keys.push(crate::imap::types::SearchKey::Recent),
            "NEW" => keys.push(crate::imap::types::SearchKey::New),
            "OLD" => keys.push(crate::imap::types::SearchKey::Old),
            "SEEN" => keys.push(crate::imap::types::SearchKey::Seen),
            "UNSEEN" => keys.push(crate::imap::types::SearchKey::Unseen),
            _ => keys.push(crate::imap::types::SearchKey::All), // Fallback
        }
    }
    if keys.is_empty() {
        keys.push(crate::imap::types::SearchKey::All);
    }
    keys
}

// ===========================================================================
// Response Sender
/// Read exactly `n` bytes from the transport (for APPEND literal data).
async fn read_imap_exact(transport: &mut BufferedImapTransport, n: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    transport.read_exact_buffered(&mut buf).await?;
    Ok(buf)
}
