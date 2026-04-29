//! Core IMAP types — mailboxes, flags, envelopes, and session state.

use crate::prelude::*;
use std::net::SocketAddr;
use std::time::SystemTime;

// ===========================================================================
// Session State
// ===========================================================================

/// IMAP session states per RFC 3501 §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImapState {
    /// Initial state — no authentication yet.
    NotAuthenticated,
    /// Authenticated — mailbox not selected.
    Authenticated,
    /// Mailbox selected — can fetch/store messages.
    Selected,
    /// Connection is shutting down.
    Logout,
}

// ===========================================================================
// Message Flags
// ===========================================================================

/// Standard IMAP message flags (RFC 3501 §2.3.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flags {
    pub seen: bool,
    pub answered: bool,
    pub flagged: bool,
    pub deleted: bool,
    pub draft: bool,
    /// Recent flag — set by server, not by clients.
    pub recent: bool,
    /// Custom (keyword) flags.
    pub keywords: Vec<String>,
}

impl Flags {
    pub fn new() -> Self {
        Self::default()
    }

    /// Format flags as an IMAP parenthesized list.
    pub fn format(&self) -> String {
        let mut parts = Vec::new();
        if self.seen {
            parts.push("\\Seen");
        }
        if self.answered {
            parts.push("\\Answered");
        }
        if self.flagged {
            parts.push("\\Flagged");
        }
        if self.deleted {
            parts.push("\\Deleted");
        }
        if self.draft {
            parts.push("\\Draft");
        }
        if self.recent {
            parts.push("\\Recent");
        }
        for kw in &self.keywords {
            parts.push(kw.as_str());
        }
        if parts.is_empty() {
            "()".to_string()
        } else {
            format!("({})", parts.join(" "))
        }
    }

    /// Parse flags from an IMAP parenthesized list.
    pub fn parse(s: &str) -> Self {
        let mut flags = Flags::new();
        let trimmed = s.trim().trim_start_matches('(').trim_end_matches(')');
        if trimmed.is_empty() {
            return flags;
        }
        for token in trimmed.split_whitespace() {
            match token {
                "\\Seen" => flags.seen = true,
                "\\Answered" => flags.answered = true,
                "\\Flagged" => flags.flagged = true,
                "\\Deleted" => flags.deleted = true,
                "\\Draft" => flags.draft = true,
                "\\Recent" => flags.recent = true,
                kw => flags.keywords.push(kw.to_string()),
            }
        }
        flags
    }
}

// ===========================================================================
// Envelope
// ===========================================================================

/// RFC 2822 / RFC 3501 envelope structure.
#[derive(Debug, Clone, Default)]
pub struct Envelope {
    /// Message date (RFC 2822 format).
    pub date: Option<String>,
    /// Message subject.
    pub subject: Option<String>,
    /// From header (sender mailbox list).
    pub from: Vec<Address>,
    /// Sender header (single mailbox).
    pub sender: Option<Address>,
    /// Reply-To header.
    pub reply_to: Option<Address>,
    /// To header (recipient list).
    pub to: Vec<Address>,
    /// Cc header (recipient list).
    pub cc: Vec<Address>,
    /// Bcc header (recipient list).
    pub bcc: Vec<Address>,
    /// In-Reply-To header.
    pub in_reply_to: Option<String>,
    /// Message-ID header.
    pub message_id: Option<String>,
}

impl Envelope {
    /// Format as an RFC 3501 ENVELOPE body without the response name.
    pub fn format_imap(&self) -> String {
        let date = format_nstring(&self.date);
        let subject = format_nstring(&self.subject);
        let from = format_address_list(&self.from);
        let sender = format_address_list_opt(&self.sender);
        let reply_to = format_address_list_opt(&self.reply_to);
        let to = format_address_list(&self.to);
        let cc = format_address_list(&self.cc);
        let bcc = format_address_list(&self.bcc);
        let in_reply_to = format_nstring(&self.in_reply_to);
        let message_id = format_nstring(&self.message_id);

        format!(
            "({} {} {} {} {} {} {} {} {} {})",
            date, subject, from, sender, reply_to, to, cc, bcc, in_reply_to, message_id
        )
    }
}

/// RFC 2822 mailbox address.
#[derive(Debug, Clone, Default)]
pub struct Address {
    /// Personal name (display name).
    pub name: Option<String>,
    /// SMTP at-domain list (source route, usually None).
    pub adl: Option<String>,
    /// Mailbox name (local part).
    pub mailbox: Option<String>,
    /// Host name (domain part).
    pub host: Option<String>,
}

impl Address {
    /// Format as a display string: "Name <user@host>" or "user@host".
    pub fn format(&self) -> String {
        let email = match (&self.mailbox, &self.host) {
            (Some(m), Some(h)) => format!("{}@{}", m, h),
            (Some(m), None) => m.clone(),
            (None, Some(h)) => h.clone(),
            _ => String::new(),
        };
        match &self.name {
            Some(n) if !n.is_empty() => format!("{} <{}>", n, email),
            _ => email,
        }
    }
}

/// Format an IMAP nstring, escaping quoted-string metacharacters.
pub fn format_nstring(s: &Option<String>) -> String {
    match s {
        Some(s) if !s.is_empty() => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        _ => "NIL".to_string(),
    }
}

/// Format an IMAP envelope address list.
pub fn format_address_list(addrs: &[Address]) -> String {
    if addrs.is_empty() {
        return "NIL".to_string();
    }
    let parts: Vec<String> = addrs.iter().map(format_address).collect();
    format!("({})", parts.join(" "))
}

/// Format a single optional address as an IMAP envelope address list.
pub fn format_address_list_opt(addr: &Option<Address>) -> String {
    match addr {
        Some(a) => format!("({})", format_address(a)),
        None => "NIL".to_string(),
    }
}

/// Format a single RFC 3501 envelope address tuple.
pub fn format_address(addr: &Address) -> String {
    let name = format_nstring(&addr.name);
    let adl = format_nstring(&addr.adl);
    let mailbox = format_nstring(&addr.mailbox);
    let host = format_nstring(&addr.host);
    format!("({} {} {} {})", name, adl, mailbox, host)
}

// ===========================================================================
// Mailbox
// ===========================================================================

/// Mailbox status information (from SELECT/EXAMINE/STATUS).
#[derive(Debug, Clone, Default)]
pub struct MailboxStatus {
    /// Number of messages in the mailbox.
    pub messages: u32,
    /// Number of messages with \Recent flag.
    pub recent: u32,
    /// Next UID to be assigned.
    pub uid_next: u32,
    /// UID validity value for the mailbox.
    pub uid_validity: u32,
    /// Number of messages with \Deleted flag set.
    pub uid_not_stored: u32,
}

/// Mailbox descriptor.
#[derive(Debug, Clone)]
pub struct Mailbox {
    /// Mailbox name (e.g., "INBOX", "Sent").
    pub name: String,
    /// Mailbox flags (subscribed, noselect, etc.).
    pub attributes: Vec<String>,
    /// Hierarchy delimiter (e.g., '/' for "INBOX/Sent").
    pub delimiter: Option<String>,
    /// Status information (populated after SELECT/STATUS).
    pub status: Option<MailboxStatus>,
}

impl Mailbox {
    pub fn new(name: String) -> Self {
        Self {
            name,
            attributes: Vec::new(),
            delimiter: Some("/".to_string()),
            status: None,
        }
    }
}

// ===========================================================================
// Message
// ===========================================================================

/// An IMAP message stored in a mailbox.
#[derive(Debug, Clone)]
pub struct Message {
    /// Unique identifier within the mailbox (monotonically increasing).
    pub uid: u32,
    /// Sequence number (position in mailbox, changes on expunge).
    pub seq: u32,
    /// System and keyword flags.
    pub flags: Flags,
    /// Internal date/time (when the message was received/appended).
    pub internal_date: SystemTime,
    /// Size in bytes (RFC 822 size).
    pub size: usize,
    /// Parsed envelope.
    pub envelope: Envelope,
    /// Full RFC 822 message body.
    pub rfc822: Vec<u8>,
}

impl Message {
    pub fn new(uid: u32, seq: u32, rfc822: Vec<u8>, internal_date: SystemTime) -> Self {
        let size = rfc822.len();
        Self {
            uid,
            seq,
            flags: Flags::new(),
            internal_date,
            size,
            envelope: Envelope::default(),
            rfc822,
        }
    }
}

// ===========================================================================
// Fetch Attributes
// ===========================================================================

/// Attributes that can be requested via FETCH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchAttr {
    /// Unique identifier.
    Uid,
    /// Message flags.
    Flags,
    /// RFC 822 size (body size without headers).
    Rfc822Size,
    /// Full RFC 822 message.
    Rfc822,
    /// RFC 822 header only.
    Rfc822Header,
    /// RFC 822 text only (body).
    Rfc822Text,
    /// Envelope structure.
    Envelope,
    /// Internal date.
    InternalDate,
    /// Body structure (MIME).
    BodyStructure,
    /// Specific body section (e.g., BODY[TEXT], BODY[HEADER]).
    BodySection(String),
    /// Message sequence number.
    MsgSize,
}

impl FetchAttr {
    /// Parse a fetch attribute from an IMAP atom.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "UID" => Some(Self::Uid),
            "FLAGS" => Some(Self::Flags),
            "RFC822.SIZE" => Some(Self::Rfc822Size),
            "RFC822" => Some(Self::Rfc822),
            "RFC822.HEADER" => Some(Self::Rfc822Header),
            "RFC822.TEXT" => Some(Self::Rfc822Text),
            "ENVELOPE" => Some(Self::Envelope),
            "INTERNALDATE" => Some(Self::InternalDate),
            "BODYSTRUCTURE" => Some(Self::BodyStructure),
            "BODY" => Some(Self::BodySection("".to_string())),
            "MSG_SIZE" | "SIZE" => Some(Self::MsgSize),
            s if s.starts_with("BODY[") && s.ends_with(']') => {
                // Extract the section from BODY[section]
                let section = &s[5..s.len() - 1];
                Some(Self::BodySection(section.to_string()))
            }
            _ => None,
        }
    }
}

// ===========================================================================
// Search Keys
// ===========================================================================

/// IMAP SEARCH command keys (RFC 3501 §6.4.4).
#[derive(Debug, Clone)]
pub enum SearchKey {
    /// All messages.
    All,
    /// Messages with \Answered flag.
    Answered,
    /// Messages with \Deleted flag.
    Deleted,
    /// Messages without \Deleted flag.
    Undeleted,
    /// Messages with \Draft flag.
    Draft,
    /// Messages with \Flagged flag.
    Flagged,
    /// Messages with \Recent flag.
    Recent,
    /// Messages without \Seen flag.
    New,
    /// Messages with \Seen flag.
    Old,
    /// Messages with \Seen flag.
    Seen,
    /// Messages without \Recent flag.
    Unseen,
    /// Messages that do NOT match the given key.
    Not(Box<SearchKey>),
    /// Messages that match BOTH keys.
    And(Box<SearchKey>, Box<SearchKey>),
    /// Messages that match EITHER key.
    Or(Box<SearchKey>, Box<SearchKey>),
    /// Messages matching the given UID range.
    UidSet(String),
    /// Messages with the given sequence number range.
    SeqSet(String),
    /// Messages sent before the given date.
    SentBefore(String),
    /// Messages sent on the given date.
    SentOn(String),
    /// Messages sent after the given date.
    SentSince(String),
    /// Messages received before the given date.
    Before(String),
    /// Messages received on the given date.
    On(String),
    /// Messages received after the given date.
    Since(String),
    /// Messages smaller than the given size in octets.
    Smaller(u32),
    /// Messages larger than the given size in octets.
    Larger(u32),
    /// Messages with the given substring in the envelope subject.
    Subject(String),
    /// Messages from the given mailbox (substring match).
    From(String),
    /// Messages to the given mailbox (substring match).
    To(String),
    /// Messages with the given substring in the body.
    Body(String),
    /// Messages with the given substring in the header.
    Text(String),
    /// Messages in the given mailbox name.
    Header(String, String),
}
