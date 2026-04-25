//! IMAP commands and responses.
//!
//! Defines the full set of IMAP4rev1 commands (RFC 3501) and their typed representations.

use std::io;

use crate::imap::types::{FetchAttr, SearchKey};

// ===========================================================================
// IMAP Commands
// ===========================================================================

/// A parsed IMAP command from a client.
#[derive(Debug, Clone)]
pub enum ImapCommand {
    // -- State: NotAuthenticated --
    /// `CAPABILITY` — List supported capabilities.
    Capability,
    /// `NOOP` — No operation (keepalive).
    Noop,
    /// `LOGOUT` — End session.
    Logout,
    /// `STARTTLS` — Upgrade to TLS (if supported).
    Starttls,
    /// `AUTHENTICATE <mechanism>` — SASL authentication.
    Authenticate { mechanism: String },
    /// `LOGIN <user> <password>` — Plain-text login.
    Login { user: String, password: String },

    // -- State: Authenticated --
    /// `SELECT <mailbox>` — Open mailbox in read-write mode.
    Select { mailbox: String },
    /// `EXAMINE <mailbox>` — Open mailbox in read-only mode.
    Examine { mailbox: String },
    /// `CREATE <mailbox>` — Create a new mailbox.
    Create { mailbox: String },
    /// `DELETE <mailbox>` — Delete a mailbox.
    Delete { mailbox: String },
    /// `RENAME <old> <new>` — Rename a mailbox.
    Rename { old: String, new: String },
    /// `SUBSCRIBE <mailbox>` — Subscribe to a mailbox.
    Subscribe { mailbox: String },
    /// `UNSUBSCRIBE <mailbox>` — Unsubscribe from a mailbox.
    Unsubscribe { mailbox: String },
    /// `LIST <ref> <pattern>` — List mailbox names.
    List { reference: String, pattern: String },
    /// `LSUB <ref> <pattern>` — List subscribed mailbox names.
    Lsub { reference: String, pattern: String },
    /// `STATUS <mailbox> (<items>)` — Get mailbox status.
    Status { mailbox: String, items: Vec<String> },
    /// `APPEND <mailbox> [<flags>] [<date>] {size}` — Append a message.
    Append {
        mailbox: String,
        flags: Option<Vec<String>>,
        date: Option<String>,
        literal_size: usize,
    },

    // -- State: Selected --
    /// `CHECK` — Checkpoint the mailbox.
    Check,
    /// `CLOSE` — Close mailbox, expunge deleted.
    Close,
    /// `EXPUNGE` — Permanently remove deleted messages.
    Expunge,
    /// `SEARCH [<charset>] <criteria>` — Search for messages.
    Search {
        charset: Option<String>,
        keys: Vec<SearchKey>,
    },
    /// `FETCH <sequence> <attributes>` — Fetch message data.
    Fetch {
        sequence: String,
        attributes: Vec<FetchAttr>,
    },
    /// `STORE <sequence> <action> <flags>` — Alter message flags.
    Store {
        sequence: String,
        action: StoreAction,
        flags: Vec<String>,
    },
    /// `COPY <sequence> <mailbox>` — Copy messages to another mailbox.
    Copy { sequence: String, mailbox: String },
    /// `UID FETCH/SEARCH/STORE/COPY` — UID-based variant.
    Uid { command: Box<ImapCommand> },

    // -- Any state --
    /// `ID` — Client identification (RFC 2971).
    Id {
        params: Vec<(String, Option<String>)>,
    },
    /// `IDLE` — Wait for mailbox updates (RFC 2177).
    Idle,
    /// `DONE` — End IDLE mode.
    Done,
    /// `ENABLE <capabilities>` — Enable extensions (RFC 5161).
    Enable { capabilities: Vec<String> },
    /// `UNSELECT` — Deselect mailbox without closing (RFC 3691).
    Unselect,
    /// `MOVE <sequence> <mailbox>` — Move messages (RFC 6851).
    Move { sequence: String, mailbox: String },
    /// `UID MOVE` — UID-based move.
    UidMove { sequence: String, mailbox: String },
    /// `NAMESPACE` — List mailbox namespaces (RFC 2342).
    Namespace,
    /// `QUOTA <mailbox>` — Get quota information (RFC 2087).
    Quota { mailbox: String },
    /// `SETQUOTA <mailbox> <limits>` — Set quota (RFC 2087).
    SetQuota {
        mailbox: String,
        limits: Vec<(String, u32)>,
    },
    /// `SORT <sort_criteria> <charset> <search_criteria>` (RFC 5256).
    Sort {
        sort_criteria: Vec<String>,
        charset: String,
        search_criteria: Vec<String>,
    },
    /// `THREAD <algorithm> <charset> <search_criteria>` (RFC 5256).
    Thread {
        algorithm: String,
        charset: String,
        search_criteria: Vec<String>,
    },
}

/// STORE flag action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreAction {
    /// `FLAGS` — Replace flags.
    Replace,
    /// `FLAGS.SILENT` — Replace flags, no response.
    ReplaceSilent,
    /// `+FLAGS` — Add flags.
    Add,
    /// `+FLAGS.SILENT` — Add flags, no response.
    AddSilent,
    /// `-FLAGS` — Remove flags.
    Remove,
    /// `-FLAGS.SILENT` — Remove flags, no response.
    RemoveSilent,
}

// ===========================================================================
// Command Parsing
// ===========================================================================

impl ImapCommand {
    /// Parse an IMAP command from a tag, command name, and argument list.
    pub fn parse(
        tag: &str,
        name: &str,
        args: &[String],
        raw_line: Option<&str>,
    ) -> io::Result<Self> {
        match name.to_uppercase().as_str() {
            "CAPABILITY" => Ok(Self::Capability),
            "NOOP" => Ok(Self::Noop),
            "LOGOUT" => Ok(Self::Logout),
            "STARTTLS" => Ok(Self::Starttls),
            "LOGIN" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "LOGIN requires user and password",
                    ));
                }
                Ok(Self::Login {
                    user: args[0].clone(),
                    password: args[1].clone(),
                })
            }
            "SELECT" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "SELECT requires mailbox name",
                    ));
                }
                Ok(Self::Select {
                    mailbox: args[0].clone(),
                })
            }
            "EXAMINE" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "EXAMINE requires mailbox name",
                    ));
                }
                Ok(Self::Examine {
                    mailbox: args[0].clone(),
                })
            }
            "CREATE" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "CREATE requires mailbox name",
                    ));
                }
                Ok(Self::Create {
                    mailbox: args[0].clone(),
                })
            }
            "DELETE" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "DELETE requires mailbox name",
                    ));
                }
                Ok(Self::Delete {
                    mailbox: args[0].clone(),
                })
            }
            "RENAME" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "RENAME requires old and new names",
                    ));
                }
                Ok(Self::Rename {
                    old: args[0].clone(),
                    new: args[1].clone(),
                })
            }
            "SUBSCRIBE" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "SUBSCRIBE requires mailbox name",
                    ));
                }
                Ok(Self::Subscribe {
                    mailbox: args[0].clone(),
                })
            }
            "UNSUBSCRIBE" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UNSUBSCRIBE requires mailbox name",
                    ));
                }
                Ok(Self::Unsubscribe {
                    mailbox: args[0].clone(),
                })
            }
            "LIST" => {
                let reference = args.first().cloned().unwrap_or_default();
                let pattern = args.get(1).cloned().unwrap_or_default();
                Ok(Self::List { reference, pattern })
            }
            "LSUB" => {
                let reference = args.first().cloned().unwrap_or_default();
                let pattern = args.get(1).cloned().unwrap_or_default();
                Ok(Self::Lsub { reference, pattern })
            }
            "STATUS" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "STATUS requires mailbox and items",
                    ));
                }
                let items = crate::imap::parser::parse_paren_list(&args[1])
                    .unwrap_or_else(|_| vec!["MESSAGES".to_string()]);
                Ok(Self::Status {
                    mailbox: args[0].clone(),
                    items,
                })
            }
            "CHECK" => Ok(Self::Check),
            "CLOSE" => Ok(Self::Close),
            "EXPUNGE" => Ok(Self::Expunge),
            "SEARCH" => {
                let mut idx = 0;
                let mut charset = None;
                if idx < args.len() && args[idx].to_uppercase() == "CHARSET" && idx + 1 < args.len()
                {
                    charset = Some(args[idx + 1].clone());
                    idx += 2;
                }
                // Remaining args are search keys
                let keys = parse_search_keys(&args[idx..]);
                Ok(Self::Search { charset, keys })
            }
            "FETCH" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "FETCH requires sequence and attributes",
                    ));
                }
                let attributes = parse_fetch_attributes(&args[1..])?;
                Ok(Self::Fetch {
                    sequence: args[0].clone(),
                    attributes,
                })
            }
            "STORE" => {
                if args.len() < 3 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "STORE requires sequence, action, and flags",
                    ));
                }
                let action = parse_store_action(&args[1])?;
                let flags = crate::imap::parser::parse_paren_list(&args[2])
                    .unwrap_or_else(|_| args[2..].to_vec());
                Ok(Self::Store {
                    sequence: args[0].clone(),
                    action,
                    flags,
                })
            }
            "COPY" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "COPY requires sequence and mailbox",
                    ));
                }
                Ok(Self::Copy {
                    sequence: args[0].clone(),
                    mailbox: args[1].clone(),
                })
            }
            "APPEND" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "APPEND requires mailbox",
                    ));
                }
                // Parse optional flags, date, and literal size
                let mailbox = args[0].clone();
                let mut flags = None;
                let mut date = None;
                let mut literal_size = 0;

                let mut i = 1;
                if i < args.len() && args[i].starts_with('(') {
                    flags = Some(crate::imap::parser::parse_paren_list(&args[i])?);
                    i += 1;
                }
                if i < args.len() && !args[i].starts_with('{') {
                    // Could be a date string (DD-Mon-YYYY format)
                    date = Some(args[i].clone());
                    i += 1;
                }
                if i < args.len() && args[i].starts_with('{') {
                    // Parse literal size from {N}
                    let size_str = args[i].trim_start_matches('{').trim_end_matches('}');
                    literal_size = size_str
                        .parse::<usize>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                }

                Ok(Self::Append {
                    mailbox,
                    flags,
                    date,
                    literal_size,
                })
            }
            "UID" => {
                // UID command wraps another command
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UID requires a sub-command",
                    ));
                }
                let sub_cmd = args[0].to_uppercase();
                let sub_args = &args[1..];

                let inner = match sub_cmd.as_str() {
                    "FETCH" => {
                        if sub_args.len() < 2 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "UID FETCH requires sequence and attributes",
                            ));
                        }
                        // sub_args = [sequence, attributes...]
                        let attributes = parse_fetch_attributes(&sub_args[1..])?;
                        ImapCommand::Fetch {
                            sequence: sub_args[0].clone(),
                            attributes,
                        }
                    }
                    "SEARCH" => {
                        let keys = parse_search_keys(sub_args);
                        ImapCommand::Search {
                            charset: None,
                            keys,
                        }
                    }
                    "STORE" => {
                        if sub_args.len() < 3 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "UID STORE requires sequence, action, and flags",
                            ));
                        }
                        let action = parse_store_action(&sub_args[1])?;
                        let flags = crate::imap::parser::parse_paren_list(&sub_args[2])
                            .unwrap_or_else(|_| sub_args[2..].to_vec());
                        ImapCommand::Store {
                            sequence: sub_args[0].clone(),
                            action,
                            flags,
                        }
                    }
                    "COPY" => {
                        if sub_args.len() < 2 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "UID COPY requires sequence and mailbox",
                            ));
                        }
                        ImapCommand::Copy {
                            sequence: sub_args[0].clone(),
                            mailbox: sub_args[1].clone(),
                        }
                    }
                    "MOVE" => {
                        if sub_args.len() < 2 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "UID MOVE requires sequence and mailbox",
                            ));
                        }
                        ImapCommand::UidMove {
                            sequence: sub_args[0].clone(),
                            mailbox: sub_args[1].clone(),
                        }
                    }
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("UID does not support {}", sub_cmd),
                        ));
                    }
                };
                Ok(Self::Uid {
                    command: Box::new(inner),
                })
            }
            "ID" => {
                let mut params = Vec::new();
                // Parse parenthesized list or NIL
                if !args.is_empty() {
                    let raw = &args[0];
                    if raw.starts_with('(') && raw.ends_with(')') {
                        let inner = &raw[1..raw.len() - 1];
                        let tokens: Vec<&str> = inner.split_whitespace().collect();
                        let mut i = 0;
                        while i + 1 < tokens.len() {
                            let key = tokens[i].to_string();
                            let value = if tokens[i + 1] == "NIL" {
                                i += 1;
                                None
                            } else {
                                Some(tokens[i + 1].trim_matches('"').to_string())
                            };
                            params.push((key, value));
                            i += 2;
                        }
                    }
                }
                Ok(Self::Id { params })
            }
            "IDLE" => Ok(Self::Idle),
            "DONE" => Ok(Self::Done),
            "ENABLE" => Ok(Self::Enable {
                capabilities: args.iter().map(|s| s.to_string()).collect(),
            }),
            "UNSELECT" => Ok(Self::Unselect),
            "MOVE" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "MOVE requires sequence and mailbox",
                    ));
                }
                Ok(Self::Move {
                    sequence: args[0].clone(),
                    mailbox: args[1].clone(),
                })
            }
            "NAMESPACE" => Ok(Self::Namespace),
            "QUOTA" => {
                if args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "QUOTA requires mailbox",
                    ));
                }
                Ok(Self::Quota {
                    mailbox: args[0].clone(),
                })
            }
            "SETQUOTA" => {
                if args.len() < 2 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "SETQUOTA requires mailbox and limits",
                    ));
                }
                // Parse quota limits from parenthesized list
                let limits = crate::imap::parser::parse_paren_list(&args[1])
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|s| {
                        let parts: Vec<&str> = s.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            if let Ok(val) = parts[1].parse() {
                                return Some((parts[0].to_string(), val));
                            }
                        }
                        None
                    })
                    .collect();
                Ok(Self::SetQuota {
                    mailbox: args[0].clone(),
                    limits,
                })
            }
            "SORT" => {
                if args.len() < 3 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "SORT requires criteria, charset, and search",
                    ));
                }
                let sort_criteria = crate::imap::parser::parse_paren_list(&args[0])
                    .unwrap_or_else(|_| vec![args[0].clone()]);
                Ok(Self::Sort {
                    sort_criteria,
                    charset: args[1].clone(),
                    search_criteria: args[2..].to_vec(),
                })
            }
            "THREAD" => {
                if args.len() < 3 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "THREAD requires algorithm, charset, and search",
                    ));
                }
                Ok(Self::Thread {
                    algorithm: args[0].clone(),
                    charset: args[1].clone(),
                    search_criteria: args[2..].to_vec(),
                })
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown command: {}", name),
            )),
        }
    }
}

// ===========================================================================
// Helper Parsers
// ===========================================================================

fn parse_fetch_attributes(args: &[String]) -> io::Result<Vec<FetchAttr>> {
    let mut attrs = Vec::new();

    // Check if args[0] is a parenthesized list
    if args[0].starts_with('(') {
        // Full parenthesized list
        let tokens = crate::imap::parser::parse_paren_list(&args[0])?;
        for token in &tokens {
            if let Some(attr) = FetchAttr::parse(token) {
                attrs.push(attr);
            }
        }
    } else if args[0].to_uppercase() == "ALL" {
        attrs.push(FetchAttr::Flags);
        attrs.push(FetchAttr::InternalDate);
        attrs.push(FetchAttr::Rfc822Size);
        attrs.push(FetchAttr::Envelope);
    } else if args[0].to_uppercase() == "FAST" {
        attrs.push(FetchAttr::Flags);
        attrs.push(FetchAttr::InternalDate);
        attrs.push(FetchAttr::Rfc822Size);
    } else if args[0].to_uppercase() == "FULL" {
        attrs.push(FetchAttr::Flags);
        attrs.push(FetchAttr::InternalDate);
        attrs.push(FetchAttr::Rfc822Size);
        attrs.push(FetchAttr::Envelope);
        attrs.push(FetchAttr::BodySection("".to_string()));
    } else {
        // Single attribute or first of many
        if let Some(attr) = FetchAttr::parse(&args[0]) {
            attrs.push(attr);
        }
        // Check for additional args
        for arg in &args[1..] {
            if let Some(attr) = FetchAttr::parse(arg) {
                attrs.push(attr);
            }
        }
    }

    if attrs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "no valid FETCH attributes",
        ));
    }

    Ok(attrs)
}

fn parse_store_action(s: &str) -> io::Result<StoreAction> {
    match s.to_uppercase().as_str() {
        "FLAGS" => Ok(StoreAction::Replace),
        "FLAGS.SILENT" => Ok(StoreAction::ReplaceSilent),
        "+FLAGS" => Ok(StoreAction::Add),
        "+FLAGS.SILENT" => Ok(StoreAction::AddSilent),
        "-FLAGS" => Ok(StoreAction::Remove),
        "-FLAGS.SILENT" => Ok(StoreAction::RemoveSilent),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown STORE action: {}", s),
        )),
    }
}

/// Parse a single search key atom (used by OR).
fn parse_single_search_key(s: &str) -> Option<SearchKey> {
    match s.to_uppercase().as_str() {
        "ALL" => Some(SearchKey::All),
        "ANSWERED" => Some(SearchKey::Answered),
        "DELETED" => Some(SearchKey::Deleted),
        "UNDELETED" => Some(SearchKey::Undeleted),
        "DRAFT" => Some(SearchKey::Draft),
        "FLAGGED" => Some(SearchKey::Flagged),
        "RECENT" => Some(SearchKey::Recent),
        "NEW" => Some(SearchKey::New),
        "OLD" => Some(SearchKey::Old),
        "SEEN" => Some(SearchKey::Seen),
        "UNSEEN" => Some(SearchKey::Unseen),
        _ => None,
    }
}

fn parse_search_keys(args: &[String]) -> Vec<SearchKey> {
    let mut keys = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let key = match args[i].to_uppercase().as_str() {
            "ALL" => SearchKey::All,
            "ANSWERED" => SearchKey::Answered,
            "DELETED" => SearchKey::Deleted,
            "UNDELETED" => SearchKey::Undeleted,
            "DRAFT" => SearchKey::Draft,
            "FLAGGED" => SearchKey::Flagged,
            "RECENT" => SearchKey::Recent,
            "NEW" => SearchKey::New,
            "OLD" => SearchKey::Old,
            "SEEN" => SearchKey::Seen,
            "UNSEEN" => SearchKey::Unseen,
            "NOT" => {
                if i + 1 < args.len() {
                    // Simple NOT implementation — just wrap next key
                    let sub = parse_search_keys(&[args[i + 1].clone()]);
                    if let Some(first) = sub.into_iter().next() {
                        i += 1;
                        SearchKey::Not(Box::new(first))
                    } else {
                        i += 1;
                        continue;
                    }
                } else {
                    i += 1;
                    continue;
                }
            }
            "SMALLER" => {
                if i + 1 < args.len() {
                    if let Ok(size) = args[i + 1].parse() {
                        i += 1;
                        SearchKey::Smaller(size)
                    } else {
                        i += 1;
                        continue;
                    }
                } else {
                    i += 1;
                    continue;
                }
            }
            "LARGER" => {
                if i + 1 < args.len() {
                    if let Ok(size) = args[i + 1].parse() {
                        i += 1;
                        SearchKey::Larger(size)
                    } else {
                        i += 1;
                        continue;
                    }
                } else {
                    i += 1;
                    continue;
                }
            }
            "SUBJECT" => {
                if i + 1 < args.len() {
                    i += 1;
                    let val = args[i].trim_matches('"').to_string();
                    SearchKey::Subject(val)
                } else {
                    i += 1;
                    continue;
                }
            }
            "FROM" => {
                if i + 1 < args.len() {
                    i += 1;
                    let val = args[i].trim_matches('"').to_string();
                    SearchKey::From(val)
                } else {
                    i += 1;
                    continue;
                }
            }
            "TO" => {
                if i + 1 < args.len() {
                    i += 1;
                    let val = args[i].trim_matches('"').to_string();
                    SearchKey::To(val)
                } else {
                    i += 1;
                    continue;
                }
            }
            "BODY" => {
                if i + 1 < args.len() {
                    i += 1;
                    let val = args[i].trim_matches('"').to_string();
                    SearchKey::Body(val)
                } else {
                    i += 1;
                    continue;
                }
            }
            "UID" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::UidSet(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "BEFORE" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::Before(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "SINCE" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::Since(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "ON" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::On(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "SENTBEFORE" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::SentBefore(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "SENTSINCE" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::SentSince(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "SENTON" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::SentOn(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "TEXT" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::Text(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "HEADER" => {
                if i + 2 < args.len() {
                    let name = args[i + 1].clone();
                    let value = args[i + 2].clone();
                    i += 2;
                    SearchKey::Header(name, value)
                } else {
                    i += 1;
                    continue;
                }
            }
            "CC" => {
                if i + 1 < args.len() {
                    i += 1;
                    SearchKey::To(args[i].clone())
                } else {
                    i += 1;
                    continue;
                }
            }
            "BCC" => {
                if i + 1 < args.len() {
                    i += 1;
                }
                SearchKey::All // Simplified: BCC always matches all
            }
            "OR" => {
                if i + 2 < args.len() {
                    i += 1;
                    let key1 = parse_single_search_key(&args[i]).unwrap_or(SearchKey::All);
                    i += 1;
                    let key2 = parse_single_search_key(&args[i]).unwrap_or(SearchKey::All);
                    SearchKey::Or(Box::new(key1), Box::new(key2))
                } else {
                    i += 1;
                    continue;
                }
            }
            _ => {
                // Treat as a sequence set
                SearchKey::SeqSet(args[i].clone())
            }
        };
        keys.push(key);
        i += 1;
    }

    keys
}

// ===========================================================================
// IMAP Response
// ===========================================================================

/// An IMAP response to send to the client.
#[derive(Debug, Clone)]
pub enum ImapResponse {
    /// Tagged response (final response to a command).
    Tagged {
        tag: String,
        result: ImapResult,
        message: String,
    },
    /// Untagged data response (* ...).
    Untagged(String),
    /// Continuation request (+ ...).
    Continuation(String),
}

/// IMAP result codes for tagged responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImapResult {
    Ok,
    No,
    Bad,
}

impl ImapResponse {
    /// Format the response as an IMAP wire-format string.
    pub fn to_wire(&self) -> String {
        match self {
            Self::Tagged {
                tag,
                result,
                message,
            } => {
                let code = match result {
                    ImapResult::Ok => "OK",
                    ImapResult::No => "NO",
                    ImapResult::Bad => "BAD",
                };
                format!("{} {} {}\r\n", tag, code, message)
            }
            Self::Untagged(data) => {
                format!("* {}\r\n", data)
            }
            Self::Continuation(text) => {
                format!("+ {}\r\n", text)
            }
        }
    }

    /// Create an OK response.
    pub fn ok(tag: &str, message: &str) -> Self {
        Self::Tagged {
            tag: tag.to_string(),
            result: ImapResult::Ok,
            message: message.to_string(),
        }
    }

    /// Create a NO response.
    pub fn no(tag: &str, message: &str) -> Self {
        Self::Tagged {
            tag: tag.to_string(),
            result: ImapResult::No,
            message: message.to_string(),
        }
    }

    /// Create a BAD response.
    pub fn bad(tag: &str, message: &str) -> Self {
        Self::Tagged {
            tag: tag.to_string(),
            result: ImapResult::Bad,
            message: message.to_string(),
        }
    }

    /// Create an untyped untagged response.
    pub fn untagaged(data: &str) -> Self {
        Self::Untagged(data.to_string())
    }

    /// Create a continuation response.
    pub fn continuation(text: &str) -> Self {
        Self::Continuation(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_login() {
        let cmd = ImapCommand::parse(
            "A001",
            "LOGIN",
            &["user".to_string(), "pass".to_string()],
            None,
        )
        .unwrap();
        match cmd {
            ImapCommand::Login { user, password } => {
                assert_eq!(user, "user");
                assert_eq!(password, "pass");
            }
            _ => panic!("expected Login"),
        }
    }

    #[test]
    fn test_parse_select() {
        let cmd = ImapCommand::parse("A002", "SELECT", &["INBOX".to_string()], None).unwrap();
        match cmd {
            ImapCommand::Select { mailbox } => {
                assert_eq!(mailbox, "INBOX");
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn test_response_ok() {
        let resp = ImapResponse::ok("A001", "LOGIN completed");
        assert_eq!(resp.to_wire(), "A001 OK LOGIN completed\r\n");
    }

    #[test]
    fn test_response_untagged() {
        let resp = ImapResponse::untagaged("1 EXISTS");
        assert_eq!(resp.to_wire(), "* 1 EXISTS\r\n");
    }
}
