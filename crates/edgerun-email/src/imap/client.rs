//! IMAP client implementation (RFC 3501).
//!
//! Auto-negotiates STARTTLS on connect. Uses the same `AsyncTlsStream::client()`
//! handshake utility as SMTP — single shared TLS implementation.

use crate::prelude::*;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::rt::{
    AsyncRead, AsyncReadExt, AsyncTcpStream, AsyncWrite, AsyncWriteExt, ConnectFuture,
};

#[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
use edgerun_tls::AsyncTlsStream;

use crate::imap::message::{ImapCommand, ImapResponse, ImapResult};
use crate::imap::types::{Envelope, FetchAttr, Flags, Mailbox, MailboxStatus, SearchKey};
use crate::server::read_line;

// ===========================================================================
// Transport enum — unified wrapper for plain TCP and TLS
// ===========================================================================

enum ImapTransport {
    Plain(AsyncTcpStream),
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    Tls(AsyncTlsStream<AsyncTcpStream>),
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    Placeholder,
}

impl ImapTransport {
    fn is_tls(&self) -> bool {
        match self {
            ImapTransport::Plain(_) => false,
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Tls(_) => true,
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Placeholder => false,
        }
    }
}

impl AsyncRead for ImapTransport {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            ImapTransport::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Tls(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Placeholder => {
                std::task::Poll::Ready(Err(io::Error::other("imap transport placeholder")))
            }
        }
    }
}

impl AsyncWrite for ImapTransport {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            ImapTransport::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Tls(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Placeholder => {
                std::task::Poll::Ready(Err(io::Error::other("imap transport placeholder")))
            }
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match &mut *self {
            ImapTransport::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Tls(s) => std::pin::Pin::new(s).poll_flush(cx),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Placeholder => {
                std::task::Poll::Ready(Err(io::Error::other("imap transport placeholder")))
            }
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match &mut *self {
            ImapTransport::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Tls(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
            ImapTransport::Placeholder => {
                std::task::Poll::Ready(Err(io::Error::other("imap transport placeholder")))
            }
        }
    }
}

impl Unpin for ImapTransport {}

// ===========================================================================
// IMAP Client
// ===========================================================================

pub struct ImapClient {
    transport: ImapTransport,
    tag_counter: AtomicU16,
    untagged: Vec<String>,
    capabilities: Vec<String>,
    selected_mailbox: Option<String>,
}

impl ImapClient {
    /// Connect to an IMAP server and auto-negotiate STARTTLS if available.
    pub async fn connect(addr: &str) -> io::Result<Self> {
        let stream = Self::connect_tcp(addr).await?;
        let server_name = Self::extract_host(addr);

        let mut client = Self {
            transport: ImapTransport::Plain(stream),
            tag_counter: AtomicU16::new(1),
            untagged: Vec::new(),
            capabilities: Vec::new(),
            selected_mailbox: None,
        };

        // Read greeting
        client.read_greeting().await?;

        // Auto-CAPABILITY
        client.do_capability().await?;

        // Auto-STARTTLS if supported
        #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
        {
            let has_starttls = client.capabilities.iter().any(|c| c == "STARTTLS");
            if has_starttls {
                client.do_starttls(&server_name).await?;
                client.do_capability().await?;
                edgerun_log::info!(
                    "edgerun-imap-client: auto-negotiated STARTTLS with {}",
                    addr
                );
            }
        }

        Ok(client)
    }

    /// Connect without STARTTLS negotiation.
    pub async fn connect_no_tls(addr: &str) -> io::Result<Self> {
        let stream = Self::connect_tcp(addr).await?;

        let mut client = Self {
            transport: ImapTransport::Plain(stream),
            tag_counter: AtomicU16::new(1),
            untagged: Vec::new(),
            capabilities: Vec::new(),
            selected_mailbox: None,
        };

        client.read_greeting().await?;
        client.do_capability().await?;
        Ok(client)
    }

    fn extract_host(addr: &str) -> String {
        if let Ok(sock_addr) = addr.parse::<SocketAddr>() {
            return sock_addr.ip().to_string();
        }
        if let Some(idx) = addr.rfind(':') {
            return addr[..idx].to_string();
        }
        addr.to_string()
    }

    async fn connect_tcp(addr: &str) -> io::Result<AsyncTcpStream> {
        if let Ok(sock_addr) = addr.parse::<SocketAddr>() {
            let fut = ConnectFuture::new(sock_addr);
            match crate::rt::timeout(std::time::Duration::from_secs(10), fut).await {
                Ok(Ok(stream)) => return Ok(std::sync::Arc::try_unwrap(stream).ok().unwrap()),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
            }
        }

        let (host, port) = edgerun_encoding::net::parse_host_port(addr)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "addr must be host:port"))?;

        let resolved = Self::dns_resolve(&host, port).await?;
        let fut = ConnectFuture::new(resolved);
        match crate::rt::timeout(std::time::Duration::from_secs(10), fut).await {
            Ok(Ok(stream)) => Ok(std::sync::Arc::try_unwrap(stream).ok().unwrap()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
        }
    }

    async fn dns_resolve(host: &str, port: u16) -> io::Result<SocketAddr> {
        let host_str = host.to_string();
        let result = crate::rt::spawn_blocking(move || {
            use std::net::ToSocketAddrs;
            format!("{}:{}", host_str, port).to_socket_addrs()
        })
        .await
        .map_err(io::Error::other)?;

        match result {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    Ok(addr)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "DNS resolution returned no addresses",
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn read_greeting(&mut self) -> io::Result<()> {
        let line = read_line(&mut self.transport).await?;
        if let Some(line) = line {
            if line.starts_with("*") {
                if let Some(cap_pos) = line.find("[CAPABILITY ") {
                    let cap_end = line[cap_pos..].find(']').unwrap_or(line.len() - cap_pos);
                    let cap_str = &line[cap_pos + 12..cap_pos + cap_end - 1];
                    self.capabilities = cap_str.split_whitespace().map(|s| s.to_string()).collect();
                }
            }
        }
        Ok(())
    }

    fn next_tag(&self) -> String {
        let tag = self.tag_counter.fetch_add(1, Ordering::SeqCst);
        format!("A{:04}", tag)
    }

    async fn do_capability(&mut self) -> io::Result<()> {
        let resp = self.send_command("CAPABILITY").await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                message,
                ..
            } => {
                // Re-parse capabilities from untagged responses
                for line in &self.untagged {
                    if let Some(cap_str) = line.strip_prefix("* CAPABILITY ") {
                        self.capabilities =
                            cap_str.split_whitespace().map(|s| s.to_string()).collect();
                        break;
                    }
                }
                if self.capabilities.is_empty() {
                    return Err(io::Error::other(message));
                }
                Ok(())
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    async fn do_starttls(&mut self, server_name: &str) -> io::Result<()> {
        let resp = self.send_command("STARTTLS").await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {}
            ImapResponse::Tagged { message, .. } => {
                return Err(io::Error::other(format!("STARTTLS rejected: {}", message)));
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unexpected response",
                ))
            }
        }

        // Upgrade transport to TLS
        let current = match std::mem::replace(&mut self.transport, ImapTransport::placeholder()) {
            ImapTransport::Plain(s) => s,
            ImapTransport::Tls(_) => {
                return Err(io::Error::other("already using TLS"));
            }
            ImapTransport::Placeholder => {
                return Err(io::Error::other("imap transport placeholder"));
            }
        };

        let tls = AsyncTlsStream::client(current, server_name, &[], None)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
        self.transport = ImapTransport::Tls(tls);

        Ok(())
    }

    async fn send_command(&mut self, command: &str) -> io::Result<ImapResponse> {
        let tag = self.next_tag();
        let full_cmd = format!("{} {}\r\n", tag, command);

        self.transport.write_all(full_cmd.as_bytes()).await?;
        self.transport.flush().await?;

        loop {
            let line = read_line(&mut self.transport).await?;
            let line = match line {
                Some(l) => l,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "server disconnected",
                    ))
                }
            };

            if line.starts_with("* ") {
                self.untagged.push(line.clone());
                edgerun_log::debug!("edgerun-imap: untagged: {}", line);
            } else if let Some(stripped) = line.strip_prefix("+ ") {
                return Ok(ImapResponse::continuation(stripped));
            } else if line.starts_with(&tag) {
                let result = if line.contains(" OK ") {
                    ImapResult::Ok
                } else if line.contains(" NO ") {
                    ImapResult::No
                } else if line.contains(" BAD ") {
                    ImapResult::Bad
                } else {
                    ImapResult::Ok
                };

                let message = line.splitn(3, ' ').last().unwrap_or("").to_string();

                return Ok(ImapResponse::Tagged {
                    tag,
                    result,
                    message,
                });
            }
        }
    }

    async fn send_command_with_literal(
        &mut self,
        command: &str,
        data: &[u8],
    ) -> io::Result<ImapResponse> {
        let tag = self.next_tag();
        let size = data.len();
        let cmd_line = format!("{} {} {{{}}}\r\n", tag, command, size);

        self.transport.write_all(cmd_line.as_bytes()).await?;
        self.transport.flush().await?;

        let cont = read_line(&mut self.transport).await?;
        if !cont.as_ref().map(|s| s.starts_with("+")).unwrap_or(false) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected continuation",
            ));
        }

        self.transport.write_all(data).await?;
        self.transport.write_all(b"\r\n").await?;
        self.transport.flush().await?;

        loop {
            let line = read_line(&mut self.transport).await?;
            let line = match line {
                Some(l) => l,
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "server disconnected",
                    ))
                }
            };

            if line.starts_with("* ") {
                self.untagged.push(line.clone());
            } else if let Some(stripped) = line.strip_prefix("+ ") {
                return Ok(ImapResponse::continuation(stripped));
            } else if line.starts_with(&tag) {
                let result = if line.contains(" OK ") {
                    ImapResult::Ok
                } else if line.contains(" NO ") {
                    ImapResult::No
                } else {
                    ImapResult::Bad
                };
                let message = line.splitn(3, ' ').last().unwrap_or("").to_string();
                return Ok(ImapResponse::Tagged {
                    tag,
                    result,
                    message,
                });
            }
        }
    }

    // ── IMAP Commands ─────────────────────────────────────────────

    pub fn is_tls(&self) -> bool {
        self.transport.is_tls()
    }

    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    pub fn selected_mailbox(&self) -> Option<&str> {
        self.selected_mailbox.as_deref()
    }

    pub async fn login(&mut self, user: &str, password: &str) -> io::Result<()> {
        let cmd = format!("LOGIN {} {}", user, password);
        let resp = self.send_command(&cmd).await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => Ok(()),
            ImapResponse::Tagged { message, .. } => {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, message))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn logout(&mut self) -> io::Result<()> {
        let _ = self.send_command("LOGOUT").await?;
        Ok(())
    }

    pub async fn select(&mut self, mailbox: &str) -> io::Result<MailboxStatus> {
        let resp = self.send_command(&format!("SELECT {}", mailbox)).await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut status = MailboxStatus::default();
                for line in &self.untagged {
                    if line.starts_with("* ") && line.contains(" EXISTS") {
                        if let Some(n) = line.split_whitespace().nth(1) {
                            status.messages = n.parse().unwrap_or(0);
                        }
                    } else if line.starts_with("* ") && line.contains(" RECENT") {
                        if let Some(n) = line.split_whitespace().nth(1) {
                            status.recent = n.parse().unwrap_or(0);
                        }
                    } else if line.starts_with("* OK [UIDVALIDITY ") {
                        if let Some(n) = line.split_whitespace().nth(2) {
                            status.uid_validity = n.trim_end_matches(']').parse().unwrap_or(0);
                        }
                    } else if line.starts_with("* OK [UIDNEXT ") {
                        if let Some(n) = line.split_whitespace().nth(2) {
                            status.uid_next = n.trim_end_matches(']').parse().unwrap_or(0);
                        }
                    }
                }
                self.selected_mailbox = Some(mailbox.to_string());
                Ok(status)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn list(&mut self, reference: &str, pattern: &str) -> io::Result<Vec<Mailbox>> {
        let resp = self
            .send_command(&format!("LIST {} {}", reference, pattern))
            .await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut mailboxes = Vec::new();
                for line in &self.untagged {
                    if let Some(mb) = parse_list_response(line) {
                        mailboxes.push(mb);
                    }
                }
                Ok(mailboxes)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn fetch(
        &mut self,
        sequence: &str,
        attrs: &[FetchAttr],
    ) -> io::Result<Vec<(u32, HashMap<String, String>)>> {
        let attr_str = attrs
            .iter()
            .map(|a| match a {
                FetchAttr::Uid => "UID".to_string(),
                FetchAttr::Flags => "FLAGS".to_string(),
                FetchAttr::Rfc822 => "RFC822".to_string(),
                FetchAttr::Rfc822Header => "RFC822.HEADER".to_string(),
                FetchAttr::Rfc822Size => "RFC822.SIZE".to_string(),
                FetchAttr::Rfc822Text => "RFC822.TEXT".to_string(),
                FetchAttr::Envelope => "ENVELOPE".to_string(),
                FetchAttr::BodySection(section) => format!("BODY[{}]", section),
                FetchAttr::InternalDate => "INTERNALDATE".to_string(),
                FetchAttr::BodyStructure => "BODYSTRUCTURE".to_string(),
                FetchAttr::MsgSize => "RFC822.SIZE".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");

        let resp = self
            .send_command(&format!("FETCH {} ({})", sequence, attr_str))
            .await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut results = Vec::new();
                for line in &self.untagged {
                    if let Some((seq, attrs)) = parse_fetch_response(line) {
                        results.push((seq, attrs));
                    }
                }
                Ok(results)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn search(&mut self, keys: &[SearchKey]) -> io::Result<Vec<u32>> {
        let key_str = keys
            .iter()
            .map(|k| match k {
                SearchKey::All => "ALL".to_string(),
                SearchKey::Answered => "ANSWERED".to_string(),
                SearchKey::Deleted => "DELETED".to_string(),
                SearchKey::Undeleted => "UNDELETED".to_string(),
                SearchKey::Draft => "DRAFT".to_string(),
                SearchKey::Flagged => "FLAGGED".to_string(),
                SearchKey::Recent => "RECENT".to_string(),
                SearchKey::New => "NEW".to_string(),
                SearchKey::Old => "OLD".to_string(),
                SearchKey::Seen => "SEEN".to_string(),
                SearchKey::Unseen => "UNSEEN".to_string(),
                SearchKey::Not(inner) => format!("NOT {}", Self::search_key_to_str(inner)),
                SearchKey::And(a, b) => format!(
                    "{} {}",
                    Self::search_key_to_str(a),
                    Self::search_key_to_str(b)
                ),
                SearchKey::Or(a, b) => format!(
                    "OR {} {}",
                    Self::search_key_to_str(a),
                    Self::search_key_to_str(b)
                ),
                SearchKey::UidSet(uids) => format!("UID {}", uids),
                SearchKey::SeqSet(seq) => seq.clone(),
                SearchKey::SentBefore(d) => format!("SENTBEFORE \"{}\"", d),
                SearchKey::SentOn(d) => format!("SENTON \"{}\"", d),
                SearchKey::SentSince(d) => format!("SENTSINCE \"{}\"", d),
                SearchKey::Before(d) => format!("BEFORE \"{}\"", d),
                SearchKey::On(d) => format!("ON \"{}\"", d),
                SearchKey::Since(d) => format!("SINCE \"{}\"", d),
                SearchKey::Smaller(n) => format!("SMALLER {}", n),
                SearchKey::Larger(n) => format!("LARGER {}", n),
                SearchKey::Subject(s) => format!("SUBJECT \"{}\"", s),
                SearchKey::From(f) => format!("FROM \"{}\"", f),
                SearchKey::To(t) => format!("TO \"{}\"", t),
                SearchKey::Body(b) => format!("BODY \"{}\"", b),
                SearchKey::Text(t) => format!("TEXT \"{}\"", t),
                SearchKey::Header(h, v) => format!("HEADER \"{}\" \"{}\"", h, v),
            })
            .collect::<Vec<_>>()
            .join(" ");

        let resp = self.send_command(&format!("SEARCH {}", key_str)).await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut results = Vec::new();
                for line in &self.untagged {
                    if let Some(stripped) = line.strip_prefix("* SEARCH") {
                        for num in stripped.split_whitespace() {
                            if let Ok(n) = num.parse::<u32>() {
                                results.push(n);
                            }
                        }
                    }
                }
                Ok(results)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    fn search_key_to_str(key: &SearchKey) -> String {
        match key {
            SearchKey::All => "ALL".to_string(),
            SearchKey::Answered => "ANSWERED".to_string(),
            SearchKey::Deleted => "DELETED".to_string(),
            SearchKey::Undeleted => "UNDELETED".to_string(),
            SearchKey::Draft => "DRAFT".to_string(),
            SearchKey::Flagged => "FLAGGED".to_string(),
            SearchKey::Recent => "RECENT".to_string(),
            SearchKey::New => "NEW".to_string(),
            SearchKey::Old => "OLD".to_string(),
            SearchKey::Seen => "SEEN".to_string(),
            SearchKey::Unseen => "UNSEEN".to_string(),
            SearchKey::Not(inner) => format!("NOT {}", Self::search_key_to_str(inner)),
            SearchKey::And(a, b) => format!(
                "{} {}",
                Self::search_key_to_str(a),
                Self::search_key_to_str(b)
            ),
            SearchKey::Or(a, b) => format!(
                "OR {} {}",
                Self::search_key_to_str(a),
                Self::search_key_to_str(b)
            ),
            SearchKey::UidSet(uids) => format!("UID {}", uids),
            SearchKey::SeqSet(seq) => seq.clone(),
            SearchKey::SentBefore(d) => format!("SENTBEFORE \"{}\"", d),
            SearchKey::SentOn(d) => format!("SENTON \"{}\"", d),
            SearchKey::SentSince(d) => format!("SENTSINCE \"{}\"", d),
            SearchKey::Before(d) => format!("BEFORE \"{}\"", d),
            SearchKey::On(d) => format!("ON \"{}\"", d),
            SearchKey::Since(d) => format!("SINCE \"{}\"", d),
            SearchKey::Smaller(n) => format!("SMALLER {}", n),
            SearchKey::Larger(n) => format!("LARGER {}", n),
            SearchKey::Subject(s) => format!("SUBJECT \"{}\"", s),
            SearchKey::From(f) => format!("FROM \"{}\"", f),
            SearchKey::To(t) => format!("TO \"{}\"", t),
            SearchKey::Body(b) => format!("BODY \"{}\"", b),
            SearchKey::Text(t) => format!("TEXT \"{}\"", t),
            SearchKey::Header(h, v) => format!("HEADER \"{}\" \"{}\"", h, v),
        }
    }

    pub async fn store(
        &mut self,
        sequence: &str,
        action: &str,
        flags: &[String],
    ) -> io::Result<Vec<u32>> {
        let flags_str = flags.join(" ");
        let resp = self
            .send_command(&format!(
                "STORE {} {}FLAGS ({})",
                sequence, action, flags_str
            ))
            .await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut seqs = Vec::new();
                for line in &self.untagged {
                    if line.starts_with("* ") && line.contains(" FETCH") {
                        if let Some(n) = line[2..].split_whitespace().next() {
                            if let Ok(seq) = n.parse::<u32>() {
                                seqs.push(seq);
                            }
                        }
                    }
                }
                Ok(seqs)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn expunge(&mut self) -> io::Result<Vec<u32>> {
        let resp = self.send_command("EXPUNGE").await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut seqs = Vec::new();
                for line in &self.untagged {
                    if line.starts_with("* ") && line.contains(" EXPUNGE") {
                        if let Some(n) = line.split_whitespace().nth(1) {
                            if let Ok(seq) = n.parse::<u32>() {
                                seqs.push(seq);
                            }
                        }
                    }
                }
                Ok(seqs)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn append(&mut self, mailbox: &str, data: &[u8]) -> io::Result<u32> {
        let resp = self
            .send_command_with_literal(&format!("APPEND {}", mailbox), data)
            .await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let uid = self
                    .untagged
                    .iter()
                    .find(|l| l.contains("APPENDUID"))
                    .and_then(|l| l.split_whitespace().find(|w| w.parse::<u32>().is_ok()))
                    .and_then(|w| w.parse::<u32>().ok());
                Ok(uid.unwrap_or(0))
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn copy_messages(&mut self, sequence: &str, dest: &str) -> io::Result<()> {
        let resp = self
            .send_command(&format!("COPY {} {}", sequence, dest))
            .await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => Ok(()),
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn status(&mut self, mailbox: &str) -> io::Result<MailboxStatus> {
        let resp = self
            .send_command(&format!(
                "STATUS {} (MESSAGES UNSEEN UIDNEXT UIDVALIDITY)",
                mailbox
            ))
            .await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                let mut status = MailboxStatus::default();
                for line in &self.untagged {
                    if line.starts_with("* ") && line.contains(" STATUS") {
                        // Parse STATUS response
                        if let Some(n) = line.split_whitespace().nth(1) {
                            if n == mailbox {
                                // Parse the parenthesized data
                                if let Some(data_start) = line.find('(') {
                                    let data = &line[data_start..];
                                    if let Some(pos) = data.find("MESSAGES ") {
                                        let n = &data[pos + 9..]
                                            .split_whitespace()
                                            .next()
                                            .unwrap_or("0");
                                        status.messages =
                                            n.trim_end_matches(')').parse().unwrap_or(0);
                                    }
                                    if let Some(pos) = data.find("UNSEEN ") {
                                        let n = &data[pos + 7..]
                                            .split_whitespace()
                                            .next()
                                            .unwrap_or("0");
                                        status.recent =
                                            n.trim_end_matches(')').parse().unwrap_or(0);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(status)
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }

    pub async fn close(&mut self) -> io::Result<()> {
        let resp = self.send_command("CLOSE").await?;
        match resp {
            ImapResponse::Tagged {
                result: ImapResult::Ok,
                ..
            } => {
                self.selected_mailbox = None;
                Ok(())
            }
            ImapResponse::Tagged { message, .. } => Err(io::Error::other(message)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected response",
            )),
        }
    }
}

impl ImapTransport {
    #[cfg(all(feature = "tls", not(target_arch = "wasm32")))]
    fn placeholder() -> Self {
        ImapTransport::Placeholder
    }
}

fn parse_list_response(line: &str) -> Option<Mailbox> {
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

fn parse_fetch_response(line: &str) -> Option<(u32, HashMap<String, String>)> {
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
    let mut attrs = HashMap::new();
    let mut parts = inner.split_whitespace().peekable();
    while let Some(key) = parts.next() {
        if let Some(&value) = parts.peek() {
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
