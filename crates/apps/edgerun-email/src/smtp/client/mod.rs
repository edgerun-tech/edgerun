//! Async SMTP client (RFC 5321).

pub mod builder;

pub use edgerun_protocols::smtp::{EmailBuilder, MimePart};

use crate::prelude::*;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::rt::{
    AsyncRead, AsyncReadExt, AsyncTcpStream, AsyncWrite, AsyncWriteExt, ConnectFuture,
};

use crate::server::read_line;
use crate::smtp::types::{SmtpResponse, parse_response_line, parse_response_lines};
use edgerun_protocols::smtp::dot_stuffed_data;

#[cfg(feature = "tls")]
use edgerun_node::tls::AsyncTlsStream;

// ===========================================================================
// ClientTransport
// ===========================================================================

enum ClientTransport {
    Empty,
    Plain(AsyncTcpStream),
    #[cfg(feature = "tls")]
    Tls(AsyncTlsStream<AsyncTcpStream>),
}

impl ClientTransport {
    /// Create a placeholder transport. Only used during TLS upgrade to
    /// temporarily replace the real transport.
    fn placeholder() -> Self {
        ClientTransport::Empty
    }

    fn is_tls(&self) -> bool {
        match self {
            ClientTransport::Empty => false,
            ClientTransport::Plain(_) => false,
            #[cfg(feature = "tls")]
            ClientTransport::Tls(_) => true,
        }
    }

    #[cfg(feature = "tls")]
    async fn upgrade_tls(self, server_name: &str) -> io::Result<ClientTransport> {
        match self {
            ClientTransport::Empty => Err(io::Error::other("missing transport")),
            ClientTransport::Tls(_) => Err(io::Error::other("already using TLS")),
            ClientTransport::Plain(stream) => {
                let tls = AsyncTlsStream::client(stream, server_name, &[], None)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
                Ok(ClientTransport::Tls(tls))
            }
        }
    }

    /// Extract the host from the peer address for SNI.
    fn server_name(&self) -> io::Result<String> {
        // We don't have direct access to the address here,
        // the caller must provide it.
        Err(io::Error::other("server_name required for TLS upgrade"))
    }
}

impl AsyncRead for ClientTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            ClientTransport::Empty => Poll::Ready(Err(io::Error::other("missing transport"))),
            ClientTransport::Plain(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            ClientTransport::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ClientTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            ClientTransport::Empty => Poll::Ready(Err(io::Error::other("missing transport"))),
            ClientTransport::Plain(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            ClientTransport::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            ClientTransport::Empty => Poll::Ready(Err(io::Error::other("missing transport"))),
            ClientTransport::Plain(s) => Pin::new(s).poll_flush(cx),
            #[cfg(feature = "tls")]
            ClientTransport::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            ClientTransport::Empty => Poll::Ready(Err(io::Error::other("missing transport"))),
            ClientTransport::Plain(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            ClientTransport::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl Unpin for ClientTransport {}

// ===========================================================================
// SmtpClient
// ===========================================================================

/// Async SMTP client.
pub struct SmtpClient {
    transport: ClientTransport,
    pub capabilities: Vec<String>,
    pub capabilities_map: HashMap<String, Option<String>>,
    server_name: String,
}

impl SmtpClient {
    /// Connect to an SMTP server and auto-negotiate STARTTLS if available.
    ///
    /// Flow:
    /// 1. TCP connect → read greeting
    /// 2. EHLO → parse capabilities
    /// 3. If STARTTLS available and `tls` feature enabled → upgrade to TLS → re-EHLO
    ///
    /// The domain used in EHLO is derived from the server address.
    pub async fn connect(addr: &str) -> io::Result<Self> {
        let server_name = Self::extract_host(addr);
        let stream = Self::connect_tcp(addr).await?;

        let transport = ClientTransport::Plain(stream);

        let mut client = Self {
            transport,
            capabilities: Vec::new(),
            capabilities_map: HashMap::new(),
            server_name: server_name.clone(),
        };

        let greeting = client.read_response().await?;
        if !greeting.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Server rejected: {}", greeting.message),
            ));
        }

        // Auto-EHLO
        client.ehlo(&server_name).await?;

        // Auto-STARTTLS if supported
        #[cfg(feature = "tls")]
        {
            let has_starttls = client.capabilities_map.contains_key("STARTTLS");
            if has_starttls {
                client.starttls().await?;
                // Re-EHLO after TLS upgrade (capabilities may change)
                client.ehlo(&server_name).await?;
                edgerun_log::info!(
                    "edgerun-smtp-client: auto-negotiated STARTTLS with {}",
                    addr
                );
            }
        }

        Ok(client)
    }

    /// Connect without STARTTLS negotiation.
    ///
    /// Use this when you need manual control over the TLS upgrade,
    /// or when connecting to a server that doesn't support STARTTLS.
    pub async fn connect_no_tls(addr: &str) -> io::Result<Self> {
        let server_name = Self::extract_host(addr);
        let stream = Self::connect_tcp(addr).await?;

        let mut client = Self {
            transport: ClientTransport::Plain(stream),
            capabilities: Vec::new(),
            capabilities_map: HashMap::new(),
            server_name: server_name.clone(),
        };

        let greeting = client.read_response().await?;
        if !greeting.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Server rejected: {}", greeting.message),
            ));
        }

        client.ehlo(&server_name).await?;
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

    // ── Connection ──────────────────────────────────────────────────────

    async fn connect_tcp(addr: &str) -> io::Result<AsyncTcpStream> {
        if let Ok(sock_addr) = addr.parse::<SocketAddr>() {
            let fut = ConnectFuture::new(sock_addr);
            match crate::rt::timeout(std::time::Duration::from_secs(10), fut).await {
                Ok(Ok(stream)) => return Ok(Arc::try_unwrap(stream).ok().unwrap()),
                Ok(Err(e)) => return Err(crate::rt::bare_io(e)),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
            }
        }

        let (host, port) = edgerun_encoding::net::parse_host_port(addr)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "addr must be host:port"))?;

        let resolved = Self::dns_resolve(&host, port).await?;
        let fut = ConnectFuture::new(resolved);
        match crate::rt::timeout(std::time::Duration::from_secs(10), fut).await {
            Ok(Ok(stream)) => Ok(Arc::try_unwrap(stream).ok().unwrap()),
            Ok(Err(e)) => Err(crate::rt::bare_io(e)),
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
                        "DNS returned no addresses",
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    // ── I/O ─────────────────────────────────────────────────────────────

    /// Read a response (handles multiline).
    async fn read_response(&mut self) -> io::Result<SmtpResponse> {
        let line = read_line(&mut self.transport).await?;
        let line = match line {
            Some(l) => l,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "server disconnected",
                ));
            }
        };

        let first = parse_response_line(&line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        if first.continued {
            let mut all_lines = vec![line];
            loop {
                let next_line = read_line(&mut self.transport).await?;
                let next_line = match next_line {
                    Some(l) => l,
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "server disconnected",
                        ));
                    }
                };
                let parsed = parse_response_line(&next_line)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                let is_final = !parsed.continued;
                all_lines.push(next_line);
                if is_final {
                    break;
                }
            }
            return parse_response_lines(&all_lines)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
        }

        parse_response_lines(&[line])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Send a command and read the response.
    async fn send_command(&mut self, command: &str) -> io::Result<SmtpResponse> {
        self.transport
            .write_all(format!("{}\r\n", command).as_bytes())
            .await?;
        self.transport.flush().await?;
        self.read_response().await
    }

    // ── Commands ────────────────────────────────────────────────────────

    /// Send EHLO and parse capabilities.
    pub async fn ehlo(&mut self, domain: &str) -> io::Result<()> {
        let response = self.send_command(&format!("EHLO {}", domain)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("EHLO rejected: {}", response.message),
            ));
        }

        self.capabilities.clear();
        self.capabilities_map.clear();

        for line in response.message.split('\n') {
            self.capabilities.push(line.to_string());
            let parts: Vec<&str> = line.splitn(2, |c: char| c.is_whitespace()).collect();
            let name = parts[0].to_uppercase();
            let param = if parts.len() > 1 {
                Some(parts[1].to_string())
            } else {
                None
            };
            self.capabilities_map.insert(name, param);
        }
        Ok(())
    }

    /// Send HELO (non-ESMTP).
    pub async fn helo(&mut self, domain: &str) -> io::Result<()> {
        let response = self.send_command(&format!("HELO {}", domain)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("HELO rejected: {}", response.message),
            ));
        }
        Ok(())
    }

    /// Send MAIL FROM.
    pub async fn mail_from(&mut self, address: &str) -> io::Result<()> {
        let response = self
            .send_command(&format!("MAIL FROM:<{}>", address))
            .await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "MAIL FROM rejected: {}",
                response.message
            )));
        }
        Ok(())
    }

    /// Send RCPT TO.
    pub async fn rcpt_to(&mut self, address: &str) -> io::Result<()> {
        let response = self.send_command(&format!("RCPT TO:<{}>", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "RCPT TO rejected: {}",
                response.message
            )));
        }
        Ok(())
    }

    /// Send DATA followed by message content.
    pub async fn data(&mut self, message: &[u8]) -> io::Result<()> {
        let response = self.send_command("DATA").await?;
        if !response.code.is_continuation() {
            return Err(io::Error::other(format!(
                "DATA rejected: {}",
                response.message
            )));
        }

        self.transport.write_all(&dot_stuffed_data(message)).await?;
        self.transport.flush().await?;

        let response = self.read_response().await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "Message rejected: {}",
                response.message
            )));
        }
        Ok(())
    }

    /// Send a BDAT chunk (RFC 3030).
    ///
    /// Sends `BDAT <size>` followed by exactly `size` bytes of data.
    /// If `last` is true, this is the final chunk and the server will
    /// attempt delivery.
    ///
    /// The server responds with `250 OK` after each chunk.
    ///
    /// Use this instead of [`Self::data`] for large messages or when
    /// the server advertises `CHUNKING`.
    pub async fn bdat(&mut self, data: &[u8], last: bool) -> io::Result<()> {
        let last_str = if last { " LAST" } else { "" };
        let cmd = format!("BDAT {}{}", data.len(), last_str);
        let response = self.send_command(&cmd).await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "BDAT rejected: {}",
                response.message
            )));
        }

        self.transport.write_all(data).await?;
        self.transport.flush().await?;

        // Server sends a final OK for the last chunk.
        if last {
            let response = self.read_response().await?;
            if !response.code.is_success() {
                return Err(io::Error::other(format!(
                    "Message rejected: {}",
                    response.message
                )));
            }
        }

        Ok(())
    }

    /// Send a complete message using BDAT (single-chunk convenience method).
    pub async fn bdat_message(&mut self, message: &[u8]) -> io::Result<()> {
        self.bdat(message, true).await
    }

    /// Send RSET.
    pub async fn rset(&mut self) -> io::Result<()> {
        let response = self.send_command("RSET").await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "RSET rejected: {}",
                response.message
            )));
        }
        Ok(())
    }

    /// Send NOOP.
    pub async fn noop(&mut self) -> io::Result<()> {
        let response = self.send_command("NOOP").await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "NOOP rejected: {}",
                response.message
            )));
        }
        Ok(())
    }

    /// Send VRFY.
    pub async fn vrfy(&mut self, address: &str) -> io::Result<String> {
        let response = self.send_command(&format!("VRFY {}", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "VRFY rejected: {}",
                response.message
            )));
        }
        Ok(response.message.clone())
    }

    /// Send QUIT.
    pub async fn quit(&mut self) -> io::Result<()> {
        let response = self.send_command("QUIT").await?;
        edgerun_log::info!("edgerun-smtp-client: QUIT response: {}", response.code);
        Ok(())
    }

    /// Start TLS (if server supports STARTTLS).
    ///
    /// Sends the STARTTLS command and performs the TLS handshake.
    /// After this, all communication is encrypted.
    #[cfg(feature = "tls")]
    pub async fn starttls(&mut self) -> io::Result<()> {
        if !self.capabilities_map.contains_key("STARTTLS") {
            return Err(io::Error::other("Server does not support STARTTLS"));
        }

        let response = self.send_command("STARTTLS").await?;
        if !response.code.is_success() {
            return Err(io::Error::other(format!(
                "STARTTLS rejected: {}",
                response.message
            )));
        }

        // Extract the transport and upgrade.
        let current = match std::mem::replace(&mut self.transport, ClientTransport::placeholder()) {
            ClientTransport::Plain(s) => s,
            ClientTransport::Empty => {
                return Err(io::Error::other("missing transport"));
            }
            ClientTransport::Tls(_) => {
                return Err(io::Error::other("already using TLS"));
            }
        };
        let server_name = self.server_name.clone();
        let tls = AsyncTlsStream::client_tls12(current, &server_name)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
        self.transport = ClientTransport::Tls(tls);

        edgerun_log::info!("edgerun-smtp-client: STARTTLS handshake complete");
        Ok(())
    }

    /// Check if the client is currently using TLS.
    pub fn is_tls_active(&self) -> bool {
        self.transport.is_tls()
    }

    /// Authenticate with the server (SASL).
    ///
    /// Sends `AUTH PLAIN <credentials>`. The credentials string should be
    /// formatted as `\0<authcid>\0<passwd>` (RFC 4616).
    pub async fn auth_plain(&mut self, credentials: &str) -> io::Result<()> {
        // Encode credentials as base64.
        let encoded = edgerun_encoding::base64::standard_encode(credentials.as_bytes());
        let response = self
            .send_command(&format!("AUTH PLAIN {}", encoded))
            .await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("AUTH PLAIN rejected: {}", response.message),
            ));
        }
        edgerun_log::info!("edgerun-smtp-client: AUTH PLAIN successful");
        Ok(())
    }

    /// Check if the server supports a specific ESMTP extension.
    pub fn supports(&self, extension: &str) -> bool {
        self.capabilities_map
            .contains_key(&extension.to_uppercase())
    }

    /// Get the maximum message size from server capabilities (if advertised).
    pub fn max_message_size(&self) -> Option<usize> {
        self.capabilities_map
            .get("SIZE")
            .and_then(|s| s.as_ref())
            .and_then(|s| s.parse().ok())
    }
}
