//! IMAP client implementation (RFC 3501).
//!
//! Provides an async IMAP4rev1 client with:
//! - TCP connect via `edgerun_rt::ConnectFuture`
//! - Tag auto-generation (A0001, A0002, ...)
//! - All standard IMAP commands: LOGIN, SELECT, FETCH, SEARCH, STORE, etc.
//! - Response parsing for tagged and untagged responses

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};

use edgerun_rt::{
    AsyncReadExt, AsyncWriteExt, AsyncTcpStream, ConnectFuture,
};

use crate::message::{ImapCommand, ImapResponse, ImapResult};
use crate::parser::{self, ImapReader};
use crate::types::{Envelope, FetchAttr, Flags, Mailbox, MailboxStatus, SearchKey};

// ===========================================================================
// IMAP Client
// ===========================================================================

/// Async IMAP client.
///
/// # Example
/// ```no_run
/// use edgerun_imap::client::ImapClient;
/// use edgerun_imap::FetchAttr;
///
/// # async fn example() -> std::io::Result<()> {
/// let mut client = ImapClient::connect("mail.example.com:143").await?;
/// client.login("user@example.com", "password").await?;
/// let mailboxes = client.list("", "*").await?;
/// client.select("INBOX").await?;
/// let results = client.fetch("1", &[FetchAttr::Uid, FetchAttr::Flags]).await?;
/// client.logout().await?;
/// # Ok(())
/// # }
/// ```
pub struct ImapClient {
    stream: Arc<AsyncTcpStream>,
    reader: ImapReader<edgerun_rt::AsyncReadHalf>,
    writer: Arc<edgerun_rt::Mutex<edgerun_rt::AsyncWriteHalf>>,
    tag_counter: AtomicU16,
    /// Uncollected untagged responses.
    untagged: Vec<String>,
    /// Server capabilities (from greeting).
    capabilities: Vec<String>,
    /// Currently selected mailbox.
    selected_mailbox: Option<String>,
}

use std::sync::Arc;

impl ImapClient {
    /// Connect to an IMAP server.
    pub async fn connect(addr: &str) -> io::Result<Self> {
        // Resolve and connect
        let stream = Self::connect_tcp(addr).await?;
        let stream = Arc::new(stream);
        let (read_half, write_half) = stream.split();
        let reader = ImapReader::new(read_half);
        let writer = Arc::new(edgerun_rt::Mutex::new(write_half));

        let mut client = Self {
            stream,
            reader,
            writer,
            tag_counter: AtomicU16::new(1),
            untagged: Vec::new(),
            capabilities: Vec::new(),
            selected_mailbox: None,
        };

        // Read greeting
        client.read_greeting().await?;

        Ok(client)
    }

    /// TCP connection with DNS resolution.
    async fn connect_tcp(addr: &str) -> io::Result<AsyncTcpStream> {
        // Try parsing as SocketAddr first
        if let Ok(sock_addr) = addr.parse::<SocketAddr>() {
            let fut = ConnectFuture::new(sock_addr);
            match edgerun_rt::timeout(std::time::Duration::from_secs(10), fut).await {
                Ok(Ok(stream)) => return Ok((*stream).clone()),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
            }
        }

        // Parse host:port
        let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "addr must be host:port"));
        }
        let host = parts[1];
        let port: u16 = parts[0].parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        // Resolve DNS
        let resolved = Self::dns_resolve(host, port).await?;
        let fut = ConnectFuture::new(resolved);
        match edgerun_rt::timeout(std::time::Duration::from_secs(10), fut).await {
            Ok(Ok(stream)) => Ok((*stream).clone()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
        }
    }

    /// DNS resolution via getaddrinfo.
    async fn dns_resolve(host: &str, port: u16) -> io::Result<SocketAddr> {
        let host_str = host.to_string();
        let result = edgerun_rt::spawn_blocking(move || {
            use std::net::ToSocketAddrs;
            format!("{}:{}", host_str, port).to_socket_addrs()
        }).await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        match result {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    Ok(addr)
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "DNS resolution returned no addresses"))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Read the server greeting.
    async fn read_greeting(&mut self) -> io::Result<()> {
        let line = self.reader.read_line().await?;
        if let Some(line) = line {
            if line.starts_with("*") {
                // Parse capabilities from greeting
                if let Some(cap_pos) = line.find("[CAPABILITY ") {
                    let cap_end = line[cap_pos..].find(']').unwrap_or(line.len() - cap_pos);
                    let cap_str = &line[cap_pos + 12..cap_pos + cap_end - 1];
                    self.capabilities = cap_str.split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                }
            }
        }
        Ok(())
    }

    /// Generate the next command tag.
    fn next_tag(&self) -> String {
        let tag = self.tag_counter.fetch_add(1, Ordering::SeqCst);
        format!("A{:04}", tag)
    }

    /// Send a command and read the tagged response.
    async fn send_command(&mut self, command: &str) -> io::Result<ImapResponse> {
        let tag = self.next_tag();
        let full_cmd = format!("{} {}\r\n", tag, command);

        // Send command
        {
            let mut w = self.writer.lock().await;
            w.write_all(full_cmd.as_bytes()).await?;
            w.flush().await?;
        }

        // Read responses until tagged response with our tag
        loop {
            let line = self.reader.read_line().await?;
            let line = match line {
                Some(l) => l,
                None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server disconnected")),
            };

            if line.starts_with("* ") {
                // Untagged response — store it
                self.untagged.push(line.clone());
                edgerun_log::debug!("edgerun-imap: untagged: {}", line);
            } else if line.starts_with("+ ") {
                // Continuation request
                return Ok(ImapResponse::continuation(&line[2..]));
            } else if line.starts_with(&tag) {
                // Tagged response — parse result
                let result = if line.contains(" OK ") {
                    ImapResult::Ok
                } else if line.contains(" NO ") {
                    ImapResult::No
                } else if line.contains(" BAD ") {
                    ImapResult::Bad
                } else {
                    ImapResult::Ok // Default
                };

                let message = line.splitn(3, |c| c == ' ')
                    .last()
                    .unwrap_or("")
                    .to_string();

                return Ok(ImapResponse::Tagged {
                    tag,
                    result,
                    message,
                });
            }
        }
    }

    /// Send a command with literal data.
    async fn send_command_with_literal(&mut self, command: &str, data: &[u8]) -> io::Result<ImapResponse> {
        let tag = self.next_tag();
        let size = data.len();
        let cmd_line = format!("{} {} {{{}}}\r\n", tag, command, size);

        // Send command with literal size
        {
            let mut w = self.writer.lock().await;
            w.write_all(cmd_line.as_bytes()).await?;
            w.flush().await?;
        }

        // Read continuation
        let cont = self.reader.read_line().await?;
        if !cont.as_ref().map(|s| s.starts_with("+")).unwrap_or(false) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "expected continuation"));
        }

        // Send literal data
        {
            let mut w = self.writer.lock().await;
            w.write_all(data).await?;
            w.write_all(b"\r\n").await?;
            w.flush().await?;
        }

        // Read tagged response
        loop {
            let line = self.reader.read_line().await?;
            let line = match line {
                Some(l) => l,
                None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server disconnected")),
            };

            if line.starts_with("* ") {
                self.untagged.push(line.clone());
            } else if line.starts_with(&tag) {
                let result = if line.contains(" OK ") {
                    ImapResult::Ok
                } else if line.contains(" NO ") {
                    ImapResult::No
                } else {
                    ImapResult::Bad
                };

                let message = line.splitn(3, |c| c == ' ')
                    .last()
                    .unwrap_or("")
                    .to_string();

                return Ok(ImapResponse::Tagged { tag, result, message });
            }
        }
    }

    // ===================================================================
    // IMAP Commands
    // ===================================================================

    /// Get server capabilities.
    pub async fn capability(&mut self) -> io::Result<Vec<String>> {
        let resp = self.send_command("CAPABILITY").await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                Ok(self.capabilities.clone())
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Authenticate with username and password.
    pub async fn login(&mut self, user: &str, password: &str) -> io::Result<()> {
        let cmd = format!("LOGIN {} {}", user, password);
        let resp = self.send_command(&cmd).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => Ok(()),
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Logout from the server.
    pub async fn logout(&mut self) -> io::Result<()> {
        let _ = self.send_command("LOGOUT").await?;
        Ok(())
    }

    /// Select a mailbox (read-write).
    pub async fn select(&mut self, mailbox: &str) -> io::Result<MailboxStatus> {
        let resp = self.send_command(&format!("SELECT {}", mailbox)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                // Parse untagged responses for mailbox status
                let status = self.parse_mailbox_status();
                self.selected_mailbox = Some(mailbox.to_string());
                Ok(status)
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Examine a mailbox (read-only).
    pub async fn examine(&mut self, mailbox: &str) -> io::Result<MailboxStatus> {
        let resp = self.send_command(&format!("EXAMINE {}", mailbox)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                let status = self.parse_mailbox_status();
                self.selected_mailbox = Some(mailbox.to_string());
                Ok(status)
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// List mailboxes.
    pub async fn list(&mut self, reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>> {
        let resp = self.send_command(&format!("LIST {} {}", reference, pattern)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                // Parse untagged LIST responses
                let mut mailboxes = Vec::new();
                for line in &self.untagged {
                    if line.starts_with("* LIST ") || line.contains(" LIST ") {
                        if let Some(name) = extract_list_name(line) {
                            mailboxes.push(Mailbox::new(name));
                        }
                    }
                }
                self.untagged.clear();
                Ok(mailboxes)
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Fetch message data.
    pub async fn fetch(&mut self, sequence: &str, attributes: &[FetchAttr]) -> io::Result<Vec<HashMap<String, String>>> {
        let attr_str = format_fetch_attrs(attributes);
        let resp = self.send_command(&format!("FETCH {} {}", sequence, attr_str)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                // Parse FETCH responses
                let mut results = Vec::new();
                for line in &self.untagged {
                    if line.contains("FETCH") {
                        if let Some(data) = parse_fetch_data(line) {
                            results.push(data);
                        }
                    }
                }
                self.untagged.clear();
                Ok(results)
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Search for messages.
    pub async fn search(&mut self, keys: &[SearchKey]) -> io::Result<Vec<u32>> {
        let key_str = format_search_keys(keys);
        let resp = self.send_command(&format!("SEARCH {}", key_str)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                // Parse SEARCH response from untagged
                for line in &self.untagged {
                    if line.starts_with("* SEARCH") {
                        let ids: Vec<u32> = line["* SEARCH ".len()..]
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        self.untagged.clear();
                        return Ok(ids);
                    }
                }
                self.untagged.clear();
                Ok(Vec::new())
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Store flags on messages.
    pub async fn store(&mut self, sequence: &str, action: &str, flags: &Flags) -> io::Result<()> {
        let resp = self.send_command(&format!(
            "STORE {} {} {}",
            sequence, action, flags.format()
        )).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => Ok(()),
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Create a mailbox.
    pub async fn create(&mut self, mailbox: &str) -> io::Result<()> {
        let resp = self.send_command(&format!("CREATE {}", mailbox)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => Ok(()),
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Delete a mailbox.
    pub async fn delete(&mut self, mailbox: &str) -> io::Result<()> {
        let resp = self.send_command(&format!("DELETE {}", mailbox)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => Ok(()),
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Get mailbox status.
    pub async fn status(&mut self, mailbox: &str, items: &[&str]) -> io::Result<HashMap<String, u32>> {
        let items_str = items.join(" ");
        let resp = self.send_command(&format!("STATUS {} ({})", mailbox, items_str)).await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                // Parse STATUS from untagged
                for line in &self.untagged {
                    if line.contains("STATUS") {
                        let status = parse_status(line);
                        self.untagged.clear();
                        return Ok(status);
                    }
                }
                self.untagged.clear();
                Ok(HashMap::new())
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Close the selected mailbox.
    pub async fn close(&mut self) -> io::Result<()> {
        let resp = self.send_command("CLOSE").await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                self.selected_mailbox = None;
                Ok(())
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// Expunge deleted messages.
    pub async fn expunge(&mut self) -> io::Result<Vec<u32>> {
        let resp = self.send_command("EXPUNGE").await?;
        match resp {
            ImapResponse::Tagged { result: ImapResult::Ok, .. } => {
                let mut removed = Vec::new();
                for line in &self.untagged {
                    if line.contains("EXPUNGE") {
                        if let Some(seq) = parse_expunge(line) {
                            removed.push(seq);
                        }
                    }
                }
                self.untagged.clear();
                Ok(removed)
            }
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::Other, message))
            }
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected response")),
        }
    }

    /// No-operation (keepalive).
    pub async fn noop(&mut self) -> io::Result<()> {
        let _ = self.send_command("NOOP").await?;
        Ok(())
    }

    // ===================================================================
    // Helpers
    // ===================================================================

    /// Parse mailbox status from untagged responses.
    fn parse_mailbox_status(&mut self) -> MailboxStatus {
        let mut status = MailboxStatus::default();
        for line in &self.untagged {
            if let Some(count) = line.strip_suffix(" EXISTS") {
                if let Ok(n) = count.parse() {
                    status.messages = n;
                }
            }
            if let Some(count) = line.strip_suffix(" RECENT") {
                if let Ok(n) = count.parse() {
                    status.recent = n;
                }
            }
            if line.contains("UIDNEXT") {
                if let Some(n) = extract_number(line, "UIDNEXT") {
                    status.uid_next = n;
                }
            }
            if line.contains("UIDVALIDITY") {
                if let Some(n) = extract_number(line, "UIDVALIDITY") {
                    status.uid_validity = n;
                }
            }
        }
        self.untagged.clear();
        status
    }
}

// ===========================================================================
// Helper Functions
// ===========================================================================

fn format_fetch_attrs(attrs: &[FetchAttr]) -> String {
    let parts: Vec<&str> = attrs.iter().map(|a| match a {
        FetchAttr::Uid => "UID",
        FetchAttr::Flags => "FLAGS",
        FetchAttr::Rfc822Size => "RFC822.SIZE",
        FetchAttr::Rfc822 => "RFC822",
        FetchAttr::Rfc822Header => "RFC822.HEADER",
        FetchAttr::Rfc822Text => "RFC822.TEXT",
        FetchAttr::Envelope => "ENVELOPE",
        FetchAttr::InternalDate => "INTERNALDATE",
        FetchAttr::BodyStructure => "BODYSTRUCTURE",
        FetchAttr::BodySection(_) => "BODY[]",
        FetchAttr::MsgSize => "RFC822.SIZE",
    }).collect();
    format!("({})", parts.join(" "))
}

fn format_search_keys(keys: &[SearchKey]) -> String {
    keys.iter().map(|k| match k {
        SearchKey::All => "ALL".to_string(),
        SearchKey::Answered => "ANSWERED".to_string(),
        SearchKey::Deleted => "DELETED".to_string(),
        SearchKey::Draft => "DRAFT".to_string(),
        SearchKey::Flagged => "FLAGGED".to_string(),
        SearchKey::Recent => "RECENT".to_string(),
        SearchKey::New => "NEW".to_string(),
        SearchKey::Old => "OLD".to_string(),
        SearchKey::Seen => "SEEN".to_string(),
        SearchKey::Unseen => "UNSEEN".to_string(),
        SearchKey::Subject(s) => format!("SUBJECT {}", s),
        SearchKey::From(s) => format!("FROM {}", s),
        SearchKey::To(s) => format!("TO {}", s),
        SearchKey::Body(s) => format!("BODY {}", s),
        SearchKey::SeqSet(s) => s.clone(),
        SearchKey::UidSet(s) => format!("UID {}", s),
        SearchKey::Smaller(n) => format!("SMALLER {}", n),
        SearchKey::Larger(n) => format!("LARGER {}", n),
        _ => "ALL".to_string(),
    }).collect::<Vec<_>>().join(" ")
}

fn extract_list_name(line: &str) -> Option<String> {
    // Parse: * LIST (attributes) "delimiter" "name"
    // Simplified — just find the last quoted string
    let mut iter = line.rsplitn(2, '"');
    if let Some(name) = iter.next() {
        if let Some(_) = iter.next() {
            return Some(name.to_string());
        }
    }
    None
}

fn parse_fetch_data(line: &str) -> Option<HashMap<String, String>> {
    // Parse: * N FETCH (UID 1 FLAGS (\Seen))
    // Simplified — extract key-value pairs
    let mut data = HashMap::new();
    if let Some(fetch_start) = line.find("FETCH (") {
        let inner = &line[fetch_start + 7..];
        let inner = inner.trim_end_matches(')');
        let parts: Vec<&str> = inner.split_whitespace().collect();
        let mut i = 0;
        while i + 1 < parts.len() {
            data.insert(parts[i].to_string(), parts[i + 1].to_string());
            i += 2;
        }
    }
    if !data.is_empty() {
        Some(data)
    } else {
        None
    }
}

fn parse_status(line: &str) -> HashMap<String, u32> {
    let mut result = HashMap::new();
    if let Some(start) = line.find('(') {
        if let Some(end) = line.rfind(')') {
            let inner = &line[start + 1..end];
            let parts: Vec<&str> = inner.split_whitespace().collect();
            let mut i = 0;
            while i + 1 < parts.len() {
                if let Ok(n) = parts[i + 1].parse() {
                    result.insert(parts[i].to_string(), n);
                }
                i += 2;
            }
        }
    }
    result
}

fn parse_expunge(line: &str) -> Option<u32> {
    // Parse: * N EXPUNGE
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
}

fn extract_number(line: &str, key: &str) -> Option<u32> {
    if let Some(pos) = line.find(key) {
        let rest = &line[pos + key.len()..];
        rest.split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
    } else {
        None
    }
}
