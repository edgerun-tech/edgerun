//! Async SMTP client (RFC 5321).

pub mod builder;

pub use builder::{EmailBuilder, MimePart};

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;

use edgerun_rt::{AsyncReadExt, AsyncWriteExt, AsyncTcpStream, ConnectFuture};

use crate::protocol::SmtpReader;
use crate::types::{SmtpResponse, SmtpResponseCode};

// ===========================================================================
// SmtpClient
// ===========================================================================

/// Async SMTP client.
pub struct SmtpClient {
    stream: std::sync::Arc<AsyncTcpStream>,
    reader: SmtpReader<edgerun_rt::AsyncReadHalf>,
    writer: std::sync::Arc<edgerun_rt::Mutex<edgerun_rt::AsyncWriteHalf>>,
    pub capabilities: Vec<String>,
    pub capabilities_map: HashMap<String, Option<String>>,
    tls_active: bool,
}

impl SmtpClient {
    /// Connect to an SMTP server.
    pub async fn connect(addr: &str) -> io::Result<Self> {
        let stream = Self::connect_tcp(addr).await?;
        let stream = std::sync::Arc::new(stream);
        let (read_half, write_half) = stream.split();
        let reader = SmtpReader::new(read_half);
        let writer = std::sync::Arc::new(edgerun_rt::Mutex::new(write_half));

        let mut client = Self {
            stream,
            reader,
            writer,
            capabilities: Vec::new(),
            capabilities_map: HashMap::new(),
            tls_active: false,
        };

        let greeting = client.read_response().await?;
        if !greeting.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Server rejected: {}", greeting.message),
            ));
        }

        Ok(client)
    }

    // ── Connection ──────────────────────────────────────────────────────

    async fn connect_tcp(addr: &str) -> io::Result<AsyncTcpStream> {
        if let Ok(sock_addr) = addr.parse::<SocketAddr>() {
            let fut = ConnectFuture::new(sock_addr);
            match edgerun_rt::timeout(std::time::Duration::from_secs(10), fut).await {
                Ok(Ok(stream)) => return Ok((*stream).clone()),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
            }
        }

        let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "addr must be host:port"));
        }
        let host = parts[1];
        let port: u16 = parts[0].parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let resolved = Self::dns_resolve(host, port).await?;
        let fut = ConnectFuture::new(resolved);
        match edgerun_rt::timeout(std::time::Duration::from_secs(10), fut).await {
            Ok(Ok(stream)) => Ok((*stream).clone()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "connect timed out")),
        }
    }

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
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "DNS returned no addresses"))
                }
            }
            Err(e) => Err(e),
        }
    }

    // ── I/O ─────────────────────────────────────────────────────────────

    /// Read a response (handles multiline).
    async fn read_response(&mut self) -> io::Result<SmtpResponse> {
        let line = self.reader.read_line().await?;
        let line = match line {
            Some(l) => l,
            None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server disconnected")),
        };

        if line.len() < 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "malformed response"));
        }

        let code_str = &line[..3];
        let code = code_str.parse::<u16>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let is_multiline = line.as_bytes().get(3) == Some(&b'-');
        let message = line[4..].to_string();

        let digit1 = (code / 100) as u8;
        let digit2 = ((code / 10) % 10) as u8;
        let digit3 = (code % 10) as u8;

        let response_code = SmtpResponseCode::new(digit1, digit2, digit3);

        if is_multiline {
            let mut all_lines = vec![message.clone()];
            loop {
                let next_line = self.reader.read_line().await?;
                let next_line = match next_line {
                    Some(l) => l,
                    None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server disconnected")),
                };
                let is_final = next_line.as_bytes().get(3) != Some(&b'-');
                all_lines.push(next_line[4..].to_string());
                if is_final {
                    break;
                }
            }
            return Ok(SmtpResponse::multiline(response_code, all_lines));
        }

        Ok(SmtpResponse::new(response_code, message))
    }

    /// Send a command and read the response.
    async fn send_command(&mut self, command: &str) -> io::Result<SmtpResponse> {
        let mut w = self.writer.lock().await;
        w.write_all(format!("{}\r\n", command).as_bytes()).await?;
        w.flush().await?;
        drop(w);
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
            let param = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };
            self.capabilities_map.insert(name, param);
        }
        Ok(())
    }

    /// Send HELO (non-ESMTP).
    pub async fn helo(&mut self, domain: &str) -> io::Result<()> {
        let response = self.send_command(&format!("HELO {}", domain)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("HELO rejected: {}", response.message)));
        }
        Ok(())
    }

    /// Send MAIL FROM.
    pub async fn mail_from(&mut self, address: &str) -> io::Result<()> {
        let response = self.send_command(&format!("MAIL FROM:<{}>", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("MAIL FROM rejected: {}", response.message)));
        }
        Ok(())
    }

    /// Send RCPT TO.
    pub async fn rcpt_to(&mut self, address: &str) -> io::Result<()> {
        let response = self.send_command(&format!("RCPT TO:<{}>", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("RCPT TO rejected: {}", response.message)));
        }
        Ok(())
    }

    /// Send DATA followed by message content.
    pub async fn data(&mut self, message: &[u8]) -> io::Result<()> {
        let response = self.send_command("DATA").await?;
        if !response.code.is_continuation() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("DATA rejected: {}", response.message)));
        }

        {
            let mut w = self.writer.lock().await;
            w.write_all(message).await?;
            w.flush().await?;
        }

        let response = self.read_response().await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Message rejected: {}", response.message)));
        }
        Ok(())
    }

    /// Send RSET.
    pub async fn rset(&mut self) -> io::Result<()> {
        let response = self.send_command("RSET").await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("RSET rejected: {}", response.message)));
        }
        Ok(())
    }

    /// Send NOOP.
    pub async fn noop(&mut self) -> io::Result<()> {
        let response = self.send_command("NOOP").await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("NOOP rejected: {}", response.message)));
        }
        Ok(())
    }

    /// Send VRFY.
    pub async fn vrfy(&mut self, address: &str) -> io::Result<String> {
        let response = self.send_command(&format!("VRFY {}", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("VRFY rejected: {}", response.message)));
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
    #[cfg(feature = "tls")]
    pub async fn starttls(&mut self) -> io::Result<()> {
        if !self.capabilities_map.contains_key("STARTTLS") {
            return Err(io::Error::new(io::ErrorKind::Other, "Server does not support STARTTLS"));
        }
        let response = self.send_command("STARTTLS").await?;
        if !response.code.is_success() {
            return Err(io::Error::new(io::ErrorKind::Other, format!("STARTTLS rejected: {}", response.message)));
        }
        self.tls_active = true;
        edgerun_log::info!("edgerun-smtp-client: STARTTLS acknowledged");
        Ok(())
    }

    /// Check if the server supports a specific ESMTP extension.
    pub fn supports(&self, extension: &str) -> bool {
        self.capabilities_map.contains_key(&extension.to_uppercase())
    }

    /// Get the maximum message size from server capabilities (if advertised).
    pub fn max_message_size(&self) -> Option<usize> {
        self.capabilities_map.get("SIZE")
            .and_then(|s| s.as_ref())
            .and_then(|s| s.parse().ok())
    }
}
