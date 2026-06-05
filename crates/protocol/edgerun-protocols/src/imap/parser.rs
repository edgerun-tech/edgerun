//! IMAP wire format parser (RFC 3501).
//!
//! IMAP is a line-based protocol with support for literals.
//! Commands: `TAG COMMAND args...\r\n`
//! Responses: `* ...` (untagged) or `TAG OK/NO/BAD ...`
//! Literals: `{N}\r\n` followed by N bytes of data.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::io;
use super::types::Mailbox;

// ===========================================================================
// IMAP Token Types
// ===========================================================================

/// A parsed IMAP token.
#[derive(Debug, Clone, PartialEq)]
pub enum ImapToken {
    /// A tagged command identifier (e.g., "A001").
    Tag(String),
    /// An untagged response marker ("*").
    Untagged,
    /// An IMAP command (e.g., "LOGIN", "SELECT").
    Command(String),
    /// An atom (unquoted string).
    Atom(String),
    /// A quoted string.
    QuotedString(String),
    /// A literal block (size, followed by data).
    Literal(usize),
    /// A parenthesized list.
    ParenList(Vec<ImapToken>),
    /// End of line.
    Eol,
}

// ===========================================================================
// Command Parser
// ===========================================================================

/// Parse an IMAP command line into tokens.
/// Format: `TAG COMMAND [args...]`
/// Handles parenthesized lists as single tokens.
pub fn parse_command_line(line: &str) -> io::Result<(String, String, Vec<String>)> {
    let tokens = tokenize_imap(line);
    if tokens.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "incomplete IMAP command",
        ));
    }

    let tag = tokens[0].to_string();
    let command = tokens[1].to_uppercase();
    let args: Vec<String> = tokens[2..].iter().map(|s| s.to_string()).collect();

    Ok((tag, command, args))
}

/// Tokenize an IMAP command line, respecting parentheses as single tokens.
fn tokenize_imap(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_paren = 0;
    let mut in_quote = false;

    for ch in line.chars() {
        if ch == '"' && in_paren == 0 {
            in_quote = !in_quote;
            current.push(ch);
        } else if in_quote {
            current.push(ch);
        } else if ch == '(' {
            in_paren += 1;
            if in_paren == 1 && !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            current.push(ch);
        } else if ch == ')' {
            current.push(ch);
            in_paren -= 1;
            if in_paren == 0 {
                tokens.push(current);
                current = String::new();
            }
        } else if ch.is_whitespace() && in_paren == 0 {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Parse a parenthesized list from a string.
/// Example: `(\Seen \Answered)` or `(UID FLAGS ENVELOPE)`
pub fn parse_paren_list(s: &str) -> io::Result<Vec<String>> {
    let trimmed = s.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected parenthesized list",
        ));
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    // Simple whitespace-split for atoms within parens
    // A full parser would handle nested parens and quoted strings
    Ok(inner.split_whitespace().map(|s| s.to_string()).collect())
}

/// Parse a sequence set (e.g., "1:5", "1,3,5", "*", "1:*").
pub fn parse_sequence_set(s: &str) -> Vec<String> {
    s.split(',').map(|s| s.trim().to_string()).collect()
}

/// Parse an untagged IMAP LIST response line.
pub fn parse_list_response(line: &str) -> Option<Mailbox> {
    if !line.starts_with("* LIST ") {
        return None;
    }
    let rest = &line[7..];
    let open_paren = rest.find('(')?;
    let close_paren = rest.find(')')?;
    let flags_str = &rest[open_paren + 1..close_paren];
    let flags: Vec<String> = flags_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let after_paren = rest[close_paren + 1..].trim_start();
    let parts: Vec<&str> = after_paren.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }
    let delimiter = parts[0].trim_matches('"');
    let delimiter = if delimiter == "NIL" {
        None
    } else {
        Some(delimiter.to_string())
    };
    let name = parts[1].trim_matches('"');
    Some(Mailbox {
        name: name.to_string(),
        attributes: flags,
        delimiter,
        status: None,
    })
}

/// Parse an untagged IMAP FETCH response line into `(sequence, attributes)`.
pub fn parse_fetch_response(line: &str) -> Option<(u32, BTreeMap<String, String>)> {
    if !line.starts_with("* ") || !line.contains(" FETCH ") {
        return None;
    }
    let seq = line[2..].split_whitespace().next()?.parse::<u32>().ok()?;
    let fetch_pos = line.find(" FETCH ")?;
    let after_fetch = &line[fetch_pos + 7..];
    if !after_fetch.starts_with('(') || !after_fetch.ends_with(')') {
        return None;
    }
    let inner = &after_fetch[1..after_fetch.len() - 1];
    let mut attrs = BTreeMap::new();
    let mut parts = inner.split_whitespace().peekable();
    while let Some(key) = parts.next() {
        if let Some(value) = parts.next() {
            if value.starts_with('(') {
                let mut val_parts = vec![value];
                for v in parts.by_ref() {
                    val_parts.push(v);
                    if v.ends_with(')') {
                        break;
                    }
                }
                attrs.insert(key.to_string(), val_parts.join(" "));
            } else {
                attrs.insert(key.to_string(), value.to_string());
            }
        }
    }
    Some((seq, attrs))
}

// ===========================================================================
// Response Formatter
// ===========================================================================

/// Format an IMAP tagged OK response.
pub fn format_ok(tag: &str, message: &str) -> String {
    format!("{} OK {}\r\n", tag, message)
}

/// Format an IMAP tagged NO response.
pub fn format_no(tag: &str, message: &str) -> String {
    format!("{} NO {}\r\n", tag, message)
}

/// Format an IMAP tagged BAD response.
pub fn format_bad(tag: &str, message: &str) -> String {
    format!("{} BAD {}\r\n", tag, message)
}

/// Format an IMAP untagged response.
pub fn format_untagged(message: &str) -> String {
    format!("* {}\r\n", message)
}

/// Format an IMAP greeting (untagged OK).
pub fn format_greeting(capabilities: &[&str]) -> String {
    let mut resp = String::new();
    resp.push_str(&format_untagged(&format!(
        "OK [CAPABILITY {}] IMAP4rev1 Service Ready",
        capabilities.join(" ")
    )));
    resp
}

/// Format CAPABILITY response.
pub fn format_capability(capabilities: &[&str]) -> String {
    format_untagged(&format!("CAPABILITY {}", capabilities.join(" ")))
}

/// Format a FLAGS response.
pub fn format_flags(flags: &[&str]) -> String {
    format_untagged(&format!("FLAGS ({})", flags.join(" ")))
}

/// Format EXISTS response.
pub fn format_exists(count: u32) -> String {
    format_untagged(&format!("{} EXISTS", count))
}

/// Format RECENT response.
pub fn format_recent(count: u32) -> String {
    format_untagged(&format!("{} RECENT", count))
}

/// Format UIDNEXT response.
pub fn format_uid_next(next: u32) -> String {
    format_untagged(&format!("OK [UIDNEXT {}]", next))
}

/// Format UIDVALIDITY response.
pub fn format_uid_validity(validity: u32) -> String {
    format_untagged(&format!("OK [UIDVALIDITY {}]", validity))
}

/// Format a FETCH response with the given UID and data.
pub fn format_fetch(uid: u32, data: &str) -> String {
    format_untagged(&format!("{} FETCH ({})", uid, data))
}

/// Format an EXPUNGE response.
pub fn format_expunge(seq: u32) -> String {
    format_untagged(&format!("{} EXPUNGE", seq))
}

/// Format a LIST response.
pub fn format_list(attributes: &[&str], delimiter: &str, name: &str) -> String {
    let attrs = if attributes.is_empty() {
        "()".to_string()
    } else {
        format!("({})", attributes.join(" "))
    };
    let quoted = if name.contains(' ') || name.contains('"') {
        format!("\"{}\"", name.replace('"', "\\\""))
    } else {
        name.to_string()
    };
    format_untagged(&format!("LIST {} \"{}\" {}", attrs, delimiter, quoted))
}

/// Format a SEARCH response.
pub fn format_search(ids: &[u32]) -> String {
    let id_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    format_untagged(&format!("SEARCH {}", id_str.join(" ")))
}

/// Format an ENVELOPE response.
pub fn format_envelope(envelope: &super::types::Envelope) -> String {
    format_untagged(&format!("ENVELOPE {}", envelope.format_imap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_login() {
        let (tag, cmd, args) = parse_command_line("A001 LOGIN user pass").unwrap();
        assert_eq!(tag, "A001");
        assert_eq!(cmd, "LOGIN");
        assert_eq!(args, vec!["user", "pass"]);
    }

    #[test]
    fn test_parse_command_select() {
        let (tag, cmd, args) = parse_command_line("A002 SELECT INBOX").unwrap();
        assert_eq!(tag, "A002");
        assert_eq!(cmd, "SELECT");
        assert_eq!(args, vec!["INBOX"]);
    }

    #[test]
    fn test_parse_paren_list_flags() {
        let tokens = parse_paren_list("(\\Seen \\Answered)").unwrap();
        assert_eq!(tokens, vec!["\\Seen", "\\Answered"]);
    }

    #[test]
    fn test_parse_list_response() {
        let mailbox = parse_list_response(r#"* LIST (\HasNoChildren) "/" "INBOX""#).unwrap();
        assert_eq!(mailbox.name, "INBOX");
        assert_eq!(mailbox.delimiter, Some("/".to_string()));
        assert_eq!(mailbox.attributes, vec!["\\HasNoChildren"]);
    }

    #[test]
    fn test_parse_fetch_response() {
        let (seq, attrs) = parse_fetch_response("* 7 FETCH (UID 42 FLAGS (\\Seen))").unwrap();
        assert_eq!(seq, 7);
        assert_eq!(attrs.get("UID"), Some(&"42".to_string()));
        assert_eq!(attrs.get("FLAGS"), Some(&"(\\Seen)".to_string()));
    }

    #[test]
    fn test_format_ok() {
        let resp = format_ok("A001", "LOGIN completed");
        assert_eq!(resp, "A001 OK LOGIN completed\r\n");
    }

    #[test]
    fn test_format_untagged() {
        let resp = format_untagged("1 EXISTS");
        assert_eq!(resp, "* 1 EXISTS\r\n");
    }

    #[test]
    fn test_format_capability() {
        let resp = format_capability(&["IMAP4rev1", "UIDPLUS"]);
        assert_eq!(resp, "* CAPABILITY IMAP4rev1 UIDPLUS\r\n");
    }
}
