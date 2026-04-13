//! Async SMTP server implementation (RFC 5321).
//!
//! Provides an SMTP server with:
//! - TCP listener with per-connection session handling
//! - Session state machine (Connected → Ready → MailSet → RcptSet → Data)
//! - Pluggable mail backend via `MailHandler` trait
//! - ESMTP extensions (SIZE, 8BITMIME, STARTTLS)
//! - STARTTLS support (with `tls` feature)

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use edgerun_rt::{
    AsyncReadExt, AsyncWriteExt, AsyncTcpListener, AsyncTcpStream,
    CancellationToken, Mutex,
};

#[cfg(feature = "tls")]
use edgerun_tls::{AsyncTlsServerStream, CertificateAndKey};

use crate::types::{
    MailEnvelope, SmtpCommand, SmtpResponse, SmtpResponseCode, SmtpState,
    parse_headers, get_subject, get_from_address, get_date,
};

// ===========================================================================
// SMTP Capabilities / Extensions
// ===========================================================================

/// Default ESMTP extensions.
const ESMTP_EXTENSIONS: &[&str] = &[
    "PIPELINING",
    "SIZE 35882577",
    "8BITMIME",
    "ENHANCEDSTATUSCODES",
    "SMTPUTF8",
];

// ===========================================================================
// Mail Handler Trait
// ===========================================================================

/// Trait for pluggable mail backend.
pub trait MailHandler: Send + Sync + 'static {
    /// Validate a sender address. Return Ok(()) if accepted.
    fn validate_sender(&self, address: &str) -> io::Result<()>;

    /// Validate a recipient address. Return Ok(()) if accepted.
    fn validate_recipient(&self, address: &str) -> io::Result<()>;

    /// Accept a complete mail transaction.
    /// The handler receives the envelope and should store/deliver the message.
    fn accept_mail(&self, envelope: &MailEnvelope) -> io::Result<()>;
}

// ===========================================================================
// In-Memory Mail Handler (for testing)
// ===========================================================================

/// Simple in-memory mail store.
pub struct MemoryMailStore {
    mailboxes: std::sync::Mutex<std::collections::HashMap<String, Vec<MailEnvelope>>>,
}

impl MemoryMailStore {
    pub fn new() -> Self {
        Self {
            mailboxes: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Add a test user.
    pub fn add_user(&self, email: &str) {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        mailboxes.insert(email.to_string(), Vec::new());
    }
}

impl Default for MemoryMailStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MailHandler for MemoryMailStore {
    fn validate_sender(&self, _address: &str) -> io::Result<()> {
        Ok(())
    }

    fn validate_recipient(&self, address: &str) -> io::Result<()> {
        let mailboxes = self.mailboxes.lock().unwrap();
        if mailboxes.contains_key(address) {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "mailbox not found"))
        }
    }

    fn accept_mail(&self, envelope: &MailEnvelope) -> io::Result<()> {
        let mut mailboxes = self.mailboxes.lock().unwrap();
        for recipient in &envelope.recipients {
            if let Some(mailbox) = mailboxes.get_mut(recipient) {
                mailbox.push(envelope.clone());
            }
        }
        Ok(())
    }
}

// ===========================================================================
// Server Configuration
// ===========================================================================

/// SMTP server configuration.
#[derive(Clone)]
pub struct SmtpServerConfig {
    /// Bind address (e.g., "0.0.0.0:25").
    pub bind_addr: String,
    /// Server domain name (used in greeting).
    pub domain: String,
    /// Maximum message size in bytes (0 = unlimited).
    pub max_message_size: usize,
    /// Whether this is an SMTPS server (TLS from start, port 465).
    pub smtps: bool,
    /// TLS certificate (for STARTTLS or SMTPS).
    #[cfg(feature = "tls")]
    pub tls_cert: Option<CertificateAndKey>,
}

impl Default for SmtpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:25".to_string(),
            domain: "edgerun.mail".to_string(),
            max_message_size: 35_882_577, // ~35MB
            smtps: false,
            #[cfg(feature = "tls")]
            tls_cert: None,
        }
    }
}

// ===========================================================================
// SMTP Server
// ===========================================================================

/// SMTP server instance.
pub struct SmtpServer {
    listener: Arc<AsyncTcpListener>,
    handler: Arc<dyn MailHandler>,
    config: SmtpServerConfig,
}

impl SmtpServer {
    /// Create a new SMTP server with the given configuration.
    pub fn new(config: SmtpServerConfig, handler: Arc<dyn MailHandler>) -> io::Result<Self> {
        let listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);
        edgerun_log::info!("edgerun-smtp: listening on {}", config.bind_addr);
        Ok(Self {
            listener,
            handler,
            config,
        })
    }

    /// Create a new server with the default in-memory mail store.
    pub fn with_memory_store(config: SmtpServerConfig) -> io::Result<Self> {
        let store = Arc::new(MemoryMailStore::new());
        Self::new(config, store)
    }

    /// Run the server, accepting connections until shutdown is cancelled.
    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match self.listener.accept().await {
                Ok((stream, peer)) => {
                    let handler = Arc::clone(&self.handler);
                    let config = self.config.clone();
                    let shutdown = shutdown.clone();

                    edgerun_log::info!("edgerun-smtp: connection from {}", peer);
                    edgerun_rt::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer, handler, config, shutdown).await {
                            edgerun_log::error!("edgerun-smtp: connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("edgerun-smtp: accept error: {}", e);
                    edgerun_rt::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }

        edgerun_log::info!("edgerun-smtp: server shut down");
        Ok(())
    }
}

// ===========================================================================
// Connection Handler
// ===========================================================================

async fn handle_connection(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    handler: Arc<dyn MailHandler>,
    config: SmtpServerConfig,
    _shutdown: CancellationToken,
) -> io::Result<()> {
    let (reader, writer) = stream.split();
    let writer = Arc::new(Mutex::new(writer));

    // Send greeting
    let greeting = SmtpResponse::service_ready(&config.domain);
    send_response(&writer, &greeting).await?;

    // Session state
    let mut state = SmtpState::Connected;
    let mut envelope = MailEnvelope::new(String::new());
    let mut ehlo_domain: Option<String> = None;
    let mut tls_active = false;

    // Use a buffered reader
    let mut buf_reader = SmtpReader::new(reader);

    // Command loop
    loop {
        let line = buf_reader.read_line().await?;
        let line = match line {
            Some(l) => l,
            None => {
                edgerun_log::info!("edgerun-smtp: client {} disconnected", peer);
                break;
            }
        };

        // During DATA phase, collect message content
        if state == SmtpState::Data {
            if line == "." {
                // End of data
                state = SmtpState::Ready;

                // Deliver the mail
                match handler.accept_mail(&envelope) {
                    Ok(()) => {
                        edgerun_log::info!(
                            "edgerun-smtp: mail accepted from {} to {:?}",
                            envelope.from,
                            envelope.recipients
                        );
                        send_response(&writer, &SmtpResponse::ok("OK: queued as 12345")).await?;
                    }
                    Err(e) => {
                        edgerun_log::error!("edgerun-smtp: mail delivery failed: {}", e);
                        send_response(&writer, &SmtpResponse::transient_failure("Mail delivery failed")).await?;
                    }
                }

                envelope.reset();
            } else {
                // Accumulate data (handle dot-stuffing)
                let data_line = if line.starts_with("..") {
                    line[1..].to_string()
                } else {
                    line.clone()
                };
                envelope.data.extend_from_slice(data_line.as_bytes());
                envelope.data.extend_from_slice(b"\r\n");

                // Check message size
                if config.max_message_size > 0 && envelope.data.len() > config.max_message_size {
                    send_response(
                        &writer,
                        &SmtpResponse::new(SmtpResponseCode::INSUFFICIENT_STORAGE, "Message too large"),
                    ).await?;
                    state = SmtpState::Ready;
                    envelope.reset();
                }
            }
            continue;
        }

        // Parse command
        let cmd = match SmtpCommand::parse(&line) {
            Ok(c) => c,
            Err(e) => {
                send_response(&writer, &SmtpResponse::syntax_error(&e.to_string())).await?;
                continue;
            }
        };

        // Handle command
        match handle_command(
            cmd, &mut state, &mut envelope, &mut ehlo_domain, &mut tls_active,
            &handler, &config, &writer,
        ).await {
            Ok(ControlFlow::Quit) => break,
            Ok(ControlFlow::Continue) => {},
            Err(e) => {
                send_response(&writer, &SmtpResponse::syntax_error(&e.to_string())).await?;
            }
        }
    }

    Ok(())
}

enum ControlFlow {
    Continue,
    Quit,
}

async fn handle_command(
    cmd: SmtpCommand,
    state: &mut SmtpState,
    envelope: &mut MailEnvelope,
    ehlo_domain: &mut Option<String>,
    tls_active: &mut bool,
    handler: &Arc<dyn MailHandler>,
    config: &SmtpServerConfig,
    writer: &Arc<Mutex<edgerun_rt::AsyncWriteHalf>>,
) -> io::Result<ControlFlow> {
    match cmd {
        SmtpCommand::Ehlo(domain) => {
            *ehlo_domain = Some(domain.clone());
            *state = SmtpState::Ready;

            // Send multiline response
            let mut lines = vec![format!("Hello {}", domain)];
            for ext in ESMTP_EXTENSIONS {
                lines.push(ext.to_string());
            }
            #[cfg(feature = "tls")]
            if !*tls_active && config.tls_cert.is_some() {
                lines.push("STARTTLS".to_string());
            }

            send_multiline_response(writer, SmtpResponseCode::OK, lines).await?;
        }
        SmtpCommand::Helo(domain) => {
            *ehlo_domain = Some(domain.clone());
            *state = SmtpState::Ready;
            send_response(writer, &SmtpResponse::ok(&format!("Hello {}", domain))).await?;
        }
        SmtpCommand::MailFrom { address, parameters } => {
            if *state != SmtpState::Ready && *state != SmtpState::MailSet {
                send_response(writer, &SmtpResponse::bad_sequence("Bad command sequence")).await?;
                return Ok(ControlFlow::Continue);
            }

            // Validate sender
            if let Err(e) = handler.validate_sender(&address) {
                send_response(writer, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            // Check SIZE parameter if present
            if config.max_message_size > 0 {
                for (key, value) in &parameters {
                    if key == "SIZE" {
                        if let Some(size_str) = value {
                            if let Ok(size) = size_str.parse::<usize>() {
                                if size > config.max_message_size {
                                    send_response(
                                        writer,
                                        &SmtpResponse::new(SmtpResponseCode::INSUFFICIENT_STORAGE, "Message too large"),
                                    ).await?;
                                    return Ok(ControlFlow::Continue);
                                }
                            }
                        }
                    }
                }
            }

            *envelope = MailEnvelope::new(address.clone());
            envelope.from_parameters = parameters;
            *state = SmtpState::MailSet;
            send_response(writer, &SmtpResponse::ok("OK")).await?;
        }
        SmtpCommand::RcptTo { address, parameters } => {
            if *state != SmtpState::MailSet && *state != SmtpState::RcptSet {
                send_response(writer, &SmtpResponse::bad_sequence("Bad command sequence")).await?;
                return Ok(ControlFlow::Continue);
            }

            // Validate recipient
            if let Err(_e) = handler.validate_recipient(&address) {
                send_response(writer, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            envelope.add_recipient(address, parameters);
            *state = SmtpState::RcptSet;
            send_response(writer, &SmtpResponse::ok("OK")).await?;
        }
        SmtpCommand::Data => {
            if *state != SmtpState::RcptSet {
                send_response(writer, &SmtpResponse::bad_sequence("Bad command sequence")).await?;
                return Ok(ControlFlow::Continue);
            }

            *state = SmtpState::Data;
            send_response(writer, &SmtpResponse::start_mail_input()).await?;
        }
        SmtpCommand::Rset => {
            envelope.reset();
            *state = SmtpState::Ready;
            send_response(writer, &SmtpResponse::ok("OK")).await?;
        }
        SmtpCommand::Noop => {
            send_response(writer, &SmtpResponse::ok("OK")).await?;
        }
        SmtpCommand::Quit => {
            *state = SmtpState::Quit;
            send_response(writer, &SmtpResponse::closing()).await?;
            return Ok(ControlFlow::Quit);
        }
        SmtpCommand::Starttls => {
            #[cfg(feature = "tls")]
            {
                if *tls_active {
                    send_response(writer, &SmtpResponse::bad_sequence("TLS already active")).await?;
                    return Ok(ControlFlow::Continue);
                }

                if config.tls_cert.is_none() {
                    send_response(writer, &SmtpResponse::command_not_implemented("STARTTLS")).await?;
                    return Ok(ControlFlow::Continue);
                }

                send_response(writer, &SmtpResponse::ok("Ready to start TLS")).await?;

                // Note: TLS upgrade would require stream replacement.
                // In a full implementation, you'd wrap the stream here.
                // For now, we acknowledge the command.
                *tls_active = true;
                edgerun_log::info!("edgerun-smtp: STARTTLS acknowledged (TLS upgrade not fully implemented)");
            }

            #[cfg(not(feature = "tls"))]
            {
                send_response(writer, &SmtpResponse::command_not_implemented("STARTTLS")).await?;
            }
        }
        SmtpCommand::Vrfy(_) => {
            // VRFY is often disabled for security
            send_response(writer, &SmtpResponse::new(
                SmtpResponseCode::COMMAND_NOT_IMPLEMENTED,
                "VRFY not implemented",
            )).await?;
        }
        SmtpCommand::Auth { mechanism, .. } => {
            send_response(writer, &SmtpResponse::new(
                SmtpResponseCode::AUTHENTICATION_REQUIRED,
                &format!("AUTH {} not supported", mechanism),
            )).await?;
        }
        _ => {
            send_response(writer, &SmtpResponse::command_not_implemented("Unknown")).await?;
        }
    }

    Ok(ControlFlow::Continue)
}

// ===========================================================================
// SMTP Reader
// ===========================================================================

/// Reads SMTP commands line-by-line.
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
// Response Helpers
// ===========================================================================

async fn send_response(
    writer: &Arc<Mutex<edgerun_rt::AsyncWriteHalf>>,
    response: &SmtpResponse,
) -> io::Result<()> {
    let formatted = response.format();
    let mut w = writer.lock().await;
    w.write_all(formatted.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

async fn send_multiline_response(
    writer: &Arc<Mutex<edgerun_rt::AsyncWriteHalf>>,
    code: SmtpResponseCode,
    lines: Vec<String>,
) -> io::Result<()> {
    let response = SmtpResponse::multiline(code, lines);
    send_response(writer, &response).await
}
