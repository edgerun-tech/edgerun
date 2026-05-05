//! IMAP wire format parser (RFC 3501).
//!
//! IMAP is a line-based protocol with support for literals.
//! Commands: `TAG COMMAND args...\r\n`
//! Responses: `* ...` (untagged) or `TAG OK/NO/BAD ...`
//! Literals: `{N}\r\n` followed by N bytes of data.

use crate::prelude::*;

#[cfg(target_os = "none")]
use edgerun_encoding::io;
#[cfg(not(target_os = "none"))]
use std::io;

#[cfg(not(target_os = "none"))]
use crate::rt::AsyncReadExt;

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
// IMAP Line Reader
// ===========================================================================

/// Reads IMAP commands line-by-line from an async reader.
/// Handles literal blocks by reading the specified number of bytes after `{N}\r\n`.
#[cfg(not(target_os = "none"))]
pub struct ImapReader<R> {
    reader: R,
    line_buf: String,
}

#[cfg(not(target_os = "none"))]
impl<R: crate::rt::AsyncRead + Unpin> ImapReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }

    /// Read the next line (until \r\n).
    pub async fn read_line(&mut self) -> io::Result<Option<String>> {
        self.line_buf.clear();
        loop {
            let mut buf = [0u8; 1];
            let n = match self.reader.read(&mut buf).await {
                Ok(0) => {
                    if self.line_buf.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(std::mem::take(&mut self.line_buf)));
                }
                Ok(n) => n,
                Err(e) => return Err(e),
            };
            if n == 0 {
                continue;
            }
            if buf[0] == b'\n' {
                // Remove trailing \r if present
                if self.line_buf.ends_with('\r') {
                    self.line_buf.pop();
                }
                return Ok(Some(std::mem::take(&mut self.line_buf)));
            }
            self.line_buf.push(buf[0] as char);
        }
    }

    /// Read exactly `n` bytes (for literal data).
    pub async fn read_exact_bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Read exactly `n` bytes as a UTF-8 string.
    pub async fn read_exact_string(&mut self, n: usize) -> io::Result<String> {
        let bytes = self.read_exact_bytes(n).await?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
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
pub fn format_envelope(envelope: &crate::imap::types::Envelope) -> String {
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
