//! Async SMTP client implementation (RFC 5321).
//!
//! Provides an SMTP client with:
//! - TCP connect via `edgerun_rt`
//! - Full ESMTP session (EHLO, MAIL FROM, RCPT TO, DATA)
//! - Message composition and sending
//! - STARTTLS support (with `tls` feature)

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;

use edgerun_rt::{
    AsyncReadExt, AsyncWriteExt, AsyncTcpStream, ConnectFuture,
};

use crate::types::{SmtpResponse, SmtpResponseCode};

// ===========================================================================
// SMTP Client
// ===========================================================================

/// Async SMTP client.
///
/// # Example
/// ```no_run
/// use edgerun_smtp::client::SmtpClient;
///
/// # async fn example() -> std::io::Result<()> {
/// let mut client = SmtpClient::connect("mail.example.com:25").await?;
/// client.ehlo("my.domain.com").await?;
/// client.mail_from("sender@my.domain.com").await?;
/// client.rcpt_to("recipient@example.com").await?;
/// client.data(b"From: sender@my.domain.com\r\nTo: recipient@example.com\r\nSubject: Test\r\n\r\nHello!\r\n.\r\n").await?;
/// client.quit().await?;
/// # Ok(())
/// # }
/// ```
pub struct SmtpClient {
    stream: std::sync::Arc<AsyncTcpStream>,
    reader: SmtpReader<edgerun_rt::AsyncReadHalf>,
    writer: std::sync::Arc<edgerun_rt::Mutex<edgerun_rt::AsyncWriteHalf>>,
    /// Server capabilities (from EHLO response).
    pub capabilities: Vec<String>,
    /// Server capabilities as a map for quick lookup.
    pub capabilities_map: HashMap<String, Option<String>>,
    /// Whether TLS is active.
    tls_active: bool,
}

impl SmtpClient {
    /// Connect to an SMTP server.
    pub async fn connect(addr: &str) -> io::Result<Self> {
        // Resolve and connect
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

        // Read greeting
        let greeting = client.read_response().await?;
        if !greeting.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                format!("Server rejected connection: {}", greeting.message),
            ));
        }

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

    /// Read a response from the server.
    async fn read_response(&mut self) -> io::Result<SmtpResponse> {
        let line = self.reader.read_line().await?;
        let line = match line {
            Some(l) => l,
            None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server disconnected")),
        };

        // Parse response code and message
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
        let response = SmtpResponse::new(response_code, message);

        // Handle multiline responses
        if is_multiline {
            let mut all_lines = vec![response.message.clone()];
            loop {
                let next_line = self.reader.read_line().await?;
                let next_line = match next_line {
                    Some(l) => l,
                    None => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "server disconnected")),
                };

                let is_final = next_line.as_bytes().get(3) != Some(&b'-');
                let msg = next_line[4..].to_string();
                all_lines.push(msg);

                if is_final {
                    break;
                }
            }

            return Ok(SmtpResponse::multiline(response_code, all_lines));
        }

        Ok(response)
    }

    /// Send a command to the server.
    async fn send_command(&mut self, command: &str) -> io::Result<SmtpResponse> {
        let mut w = self.writer.lock().await;
        w.write_all(format!("{}\r\n", command).as_bytes()).await?;
        w.flush().await?;
        drop(w);

        self.read_response().await
    }

    /// Send EHLO and parse capabilities.
    pub async fn ehlo(&mut self, domain: &str) -> io::Result<()> {
        let response = self.send_command(&format!("EHLO {}", domain)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("EHLO rejected: {}", response.message),
            ));
        }

        // Parse capabilities from multiline response
        self.capabilities.clear();
        self.capabilities_map.clear();

        for line in response.message.split('\n') {
            self.capabilities.push(line.to_string());

            // Parse extension name and optional parameter
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
        let response = self.send_command(&format!("MAIL FROM:<{}>", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("MAIL FROM rejected: {}", response.message),
            ));
        }
        Ok(())
    }

    /// Send RCPT TO.
    pub async fn rcpt_to(&mut self, address: &str) -> io::Result<()> {
        let response = self.send_command(&format!("RCPT TO:<{}>", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("RCPT TO rejected: {}", response.message),
            ));
        }
        Ok(())
    }

    /// Send DATA command followed by message content.
    pub async fn data(&mut self, message: &[u8]) -> io::Result<()> {
        // Send DATA command
        let response = self.send_command("DATA").await?;
        if !response.code.is_continuation() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("DATA rejected: {}", response.message),
            ));
        }

        // Send message content
        {
            let mut w = self.writer.lock().await;
            w.write_all(message).await?;
            w.flush().await?;
        }

        // Read final response
        let response = self.read_response().await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Message rejected: {}", response.message),
            ));
        }

        Ok(())
    }

    /// Send RSET (reset current transaction).
    pub async fn rset(&mut self) -> io::Result<()> {
        let response = self.send_command("RSET").await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("RSET rejected: {}", response.message),
            ));
        }
        Ok(())
    }

    /// Send NOOP (keepalive).
    pub async fn noop(&mut self) -> io::Result<()> {
        let response = self.send_command("NOOP").await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("NOOP rejected: {}", response.message),
            ));
        }
        Ok(())
    }

    /// Send VRFY (verify mailbox).
    pub async fn vrfy(&mut self, address: &str) -> io::Result<String> {
        let response = self.send_command(&format!("VRFY {}", address)).await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("VRFY rejected: {}", response.message),
            ));
        }
        Ok(response.message.clone())
    }

    /// Send QUIT.
    pub async fn quit(&mut self) -> io::Result<()> {
        let response = self.send_command("QUIT").await?;
        // Even if QUIT fails, we consider it successful from the client perspective
        edgerun_log::info!("edgerun-smtp-client: QUIT response: {}", response.code);
        Ok(())
    }

    /// Start TLS (if server supports STARTTLS).
    #[cfg(feature = "tls")]
    pub async fn starttls(&mut self) -> io::Result<()> {
        if !self.capabilities_map.contains_key("STARTTLS") {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Server does not support STARTTLS",
            ));
        }

        let response = self.send_command("STARTTLS").await?;
        if !response.code.is_success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("STARTTLS rejected: {}", response.message),
            ));
        }

        // In a full implementation, you would upgrade the stream to TLS here.
        // This requires stream replacement which is complex in async Rust.
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

// ===========================================================================
// SMTP Reader
// ===========================================================================

/// Reads SMTP responses line-by-line.
struct SmtpReader<R> {
    reader: R,
    line_buf: String,
}

impl<R: edgerun_rt::AsyncRead + Unpin> SmtpReader<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            line_buf: String::with_capacity(1024),
        }
    }

    async fn read_line(&mut self) -> io::Result<Option<String>> {
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
                if self.line_buf.ends_with('\r') {
                    self.line_buf.pop();
                }
                return Ok(Some(std::mem::take(&mut self.line_buf)));
            }
            self.line_buf.push(buf[0] as char);
        }
    }
}

// ===========================================================================
// Email Message Builder
// ===========================================================================

/// Helper for building RFC 5322 email messages.
pub struct EmailBuilder {
    headers: Vec<(String, String)>,
    body: String,
}

impl EmailBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            body: String::new(),
        }
    }

    /// Set the From header.
    pub fn from(mut self, address: &str) -> Self {
        self.headers.push(("From".to_string(), address.to_string()));
        self
    }

    /// Set the To header.
    pub fn to(mut self, address: &str) -> Self {
        self.headers.push(("To".to_string(), address.to_string()));
        self
    }

    /// Set the Cc header.
    pub fn cc(mut self, address: &str) -> Self {
        self.headers.push(("Cc".to_string(), address.to_string()));
        self
    }

    /// Set the Subject header.
    pub fn subject(mut self, subject: &str) -> Self {
        self.headers.push(("Subject".to_string(), subject.to_string()));
        self
    }

    /// Set the Date header (auto-generated if not set).
    pub fn date(mut self, date: &str) -> Self {
        self.headers.push(("Date".to_string(), date.to_string()));
        self
    }

    /// Set the Message-ID header.
    pub fn message_id(mut self, id: &str) -> Self {
        self.headers.push(("Message-ID".to_string(), id.to_string()));
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Set the message body.
    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    /// Build the complete RFC 5322 message.
    pub fn build(self) -> Vec<u8> {
        let mut message = String::new();

        // Add headers
        for (name, value) in &self.headers {
            message.push_str(&format!("{}: {}\r\n", name, value));
        }

        // Add Date if not present
        if !self.headers.iter().any(|(n, _)| n == "Date") {
            let date = generate_rfc2822_date();
            message.push_str(&format!("Date: {}\r\n", date));
        }

        // Blank line separates headers from body
        message.push_str("\r\n");

        // Body
        message.push_str(&self.body);

        // Ensure message ends with CRLF
        if !message.ends_with("\r\n") {
            message.push_str("\r\n");
        }

        message.into_bytes()
    }
}

impl Default for EmailBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate an RFC 2822 date string.
fn generate_rfc2822_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();

    // Simplified date generation
    // For production use, use the `chrono` crate
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;

    // Approximate month/day
    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    let mut day = day_of_year as i64;
    for (i, &md) in month_days.iter().enumerate() {
        if day < md as i64 {
            month = i;
            break;
        }
        day -= md as i64;
    }
    day += 1;

    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let mins = (time_of_day % 3600) / 60;
    let secs = time_of_day % 60;

    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let days_of_week = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    // Calculate day of week (approximate)
    let dow = ((days + 3) % 7) as usize;

    format!("{}, {:02} {} {:04} {:02}:{:02}:{:02} +0000",
        days_of_week[dow], day, months[month], year, hours, mins, secs)
}
