//! Per-connection session handling — command loop, state machine, SASL AUTH, STARTTLS upgrade.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use edgerun_rt::{
    AsyncRead, AsyncReadExt, AsyncTcpStream, AsyncWrite, AsyncWriteExt, CancellationToken,
};

use crate::protocol::read_smtp_line;
use crate::server::handler::{AuthCredentials, AuthResult, MailHandler};
use crate::types::command::{extract_dsn_envid, extract_dsn_notify, extract_dsn_orcpt, extract_dsn_ret};
use crate::types::{
    EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse, SmtpResponseCode,
    SmtpState,
};

#[cfg(feature = "tls")]
use edgerun_tls::{AsyncTlsServerStream, CertificateAndKey};

// ===========================================================================
// Server Configuration
// ===========================================================================

#[derive(Clone)]
pub struct SmtpServerConfig {
    pub bind_addr: String,
    pub domain: String,
    pub limits: ServerLimits,
    pub smtps: bool,
    /// Comma-separated list of supported AUTH mechanisms (e.g. "PLAIN,LOGIN").
    pub auth_mechanisms: Vec<String>,
    #[cfg(feature = "tls")]
    pub tls_cert: Option<CertificateAndKey>,
}

impl Default for SmtpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:25".to_string(),
            domain: "edgerun.mail".to_string(),
            limits: ServerLimits::default(),
            smtps: false,
            auth_mechanisms: vec!["PLAIN".to_string(), "LOGIN".to_string()],
            #[cfg(feature = "tls")]
            tls_cert: None,
        }
    }
}

// ===========================================================================
// Transport enum — unified wrapper for plain TCP and TLS
// ===========================================================================

/// Wraps either a plain TCP connection or a TLS stream so the SMTP
/// session loop can read/write transparently and upgrade mid-session.
pub enum SmtpTransport {
    Plain(AsyncTcpStream),
    #[cfg(feature = "tls")]
    Tls(AsyncTlsServerStream<AsyncTcpStream>),
}

impl SmtpTransport {
    /// Check whether this transport is already using TLS.
    pub fn is_tls(&self) -> bool {
        match self {
            SmtpTransport::Plain(_) => false,
            #[cfg(feature = "tls")]
            SmtpTransport::Tls(_) => true,
        }
    }

    /// Upgrade a plain transport to TLS in place.
    ///
    /// Consumes `self` and returns a new `SmtpTransport::Tls` after the
    /// handshake completes.
    #[cfg(feature = "tls")]
    pub async fn upgrade_tls(
        self,
        cert_and_key: &CertificateAndKey,
    ) -> io::Result<SmtpTransport> {
        match self {
            SmtpTransport::Tls(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "already using TLS",
                ));
            }
            SmtpTransport::Plain(stream) => {
                let fd = stream.into_fd();
                let tcp = AsyncTcpStream::from_raw(fd);
                let mut tls_stream = AsyncTlsServerStream::new(tcp);
                tls_stream
                    .handshake(cert_and_key)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
                Ok(SmtpTransport::Tls(tls_stream))
            }
        }
    }
}

impl AsyncRead for SmtpTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            SmtpTransport::Plain(s) => Pin::new(s).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            SmtpTransport::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for SmtpTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            SmtpTransport::Plain(s) => Pin::new(s).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            SmtpTransport::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            SmtpTransport::Plain(s) => Pin::new(s).poll_flush(cx),
            #[cfg(feature = "tls")]
            SmtpTransport::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut *self {
            SmtpTransport::Plain(s) => Pin::new(s).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            SmtpTransport::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl Unpin for SmtpTransport {}

// ===========================================================================
// SMTP Server
// ===========================================================================

pub struct SmtpServer {
    listener: Arc<edgerun_rt::AsyncTcpListener>,
    handler: Arc<dyn MailHandler>,
    config: SmtpServerConfig,
}

impl SmtpServer {
    pub fn new(config: SmtpServerConfig, handler: Arc<dyn MailHandler>) -> io::Result<Self> {
        let listener = Arc::new(edgerun_rt::AsyncTcpListener::bind(&config.bind_addr)?);
        edgerun_log::info!("edgerun-smtp: listening on {}", config.bind_addr);
        Ok(Self {
            listener,
            handler,
            config,
        })
    }

    pub fn with_memory_store(config: SmtpServerConfig) -> io::Result<Self> {
        let store = Arc::new(crate::server::handler::MemoryMailStore::new());
        Self::new(config, store)
    }

    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match self.listener.accept().await {
                Ok((stream, peer)) => {
                    let handler = Arc::clone(&self.handler);
                    let config = self.config.clone();
                    let shutdown = shutdown.clone();

                    edgerun_log::info!("edgerun-smtp: connection from {}", peer);
                    edgerun_rt::spawn(async move {
                        if let Err(e) = handle_connection(stream, peer, handler, config, shutdown)
                            .await
                        {
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

enum ControlFlow {
    Continue,
    Quit,
    StartTls,
}

/// State machine for SASL AUTH challenge/response exchanges.
enum AuthExchangeState {
    /// Not in an AUTH exchange.
    Idle,
    /// PLAIN: waiting for the initial base64 response (or empty challenge sent).
    Plain,
    /// LOGIN: challenge "Username:" sent, expecting base64 username next.
    LoginUsername,
    /// LOGIN: username received, "Password:" challenge sent, expecting password next.
    LoginPassword { username: String },
}

async fn handle_connection(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    handler: Arc<dyn MailHandler>,
    config: SmtpServerConfig,
    _shutdown: CancellationToken,
) -> io::Result<()> {
    // Unwrap the Arc — we need the owned AsyncTcpStream for the transport.
    // The accept loop only has one reference here, so this succeeds.
    let stream = match Arc::try_unwrap(stream) {
        Ok(s) => s,
        Err(_) => panic!("handle_connection called with non-exclusive Arc reference"),
    };

    let mut transport = SmtpTransport::Plain(stream);

    let greeting = SmtpResponse::service_ready(&config.domain);
    send_response(&mut transport, &greeting).await?;

    let mut state = SmtpState::Connected;
    let mut envelope = MailEnvelope::new(String::new());
    let mut ehlo_domain: Option<String> = None;
    let mut command_count: usize = 0;
    let mut last_activity = std::time::Instant::now();

    // ── Auth state ────────────────────────────────────────────────────
    let mut authenticated: bool = false;
    let mut auth_identity: Option<String> = None;
    let mut auth_exchange: AuthExchangeState = AuthExchangeState::Idle;

    loop {
        // Idle timeout
        let elapsed = last_activity.elapsed().as_secs();
        if elapsed > config.limits.idle_timeout_secs {
            edgerun_log::info!(
                "edgerun-smtp: {} idle timeout ({}s > {}s)",
                peer, elapsed, config.limits.idle_timeout_secs
            );
            send_response(
                &mut transport,
                &SmtpResponse::new(SmtpResponseCode::CLOSING, "Idle timeout"),
            )
            .await?;
            break;
        }

        // Command count limit
        if command_count >= config.limits.max_commands {
            edgerun_log::info!("edgerun-smtp: {} exceeded max commands", peer);
            send_response(&mut transport, &SmtpResponse::bad_sequence("Too many commands")).await?;
            break;
        }

        let line = match read_smtp_line(&mut transport).await? {
            Some(l) => l,
            None => {
                edgerun_log::info!("edgerun-smtp: {} disconnected", peer);
                break;
            }
        };

        last_activity = std::time::Instant::now();

        // Line length enforcement
        if line.len() > config.limits.max_line_length {
            send_response(
                &mut transport,
                &SmtpResponse::line_too_long(line.len(), config.limits.max_line_length),
            )
            .await?;
            continue;
        }

        // ── AUTH challenge/response exchange ────────────────────────
        if !matches!(auth_exchange, AuthExchangeState::Idle) {
            auth_exchange = handle_auth_response(
                &line,
                &handler,
                &mut transport,
                &mut authenticated,
                &mut auth_identity,
                auth_exchange,
            )
            .await?;
            continue;
        }

        // ── DATA phase ─────────────────────────────────────────────
        if state == SmtpState::Data {
            if line == "." {
                state = SmtpState::Ready;
                command_count += 1;

                match handler.accept_mail(&envelope) {
                    Ok(()) => {
                        edgerun_log::info!(
                            "edgerun-smtp: mail accepted from {} to {:?}",
                            envelope.from, envelope.recipients,
                        );
                        send_response(
                            &mut transport,
                            &SmtpResponse::ok("OK: queued")
                                .with_enhanced(EnhancedStatusCode::QUEUED),
                        )
                        .await?;
                    }
                    Err(e) => {
                        edgerun_log::error!("edgerun-smtp: delivery failed: {}", e);
                        send_response(
                            &mut transport,
                            &SmtpResponse::transient_failure("Delivery failed"),
                        )
                        .await?;
                    }
                }
                envelope.reset();
                envelope.authenticated_identity = auth_identity.clone();
            } else {
                let data_line = if line.starts_with("..") {
                    line[1..].to_string()
                } else {
                    line.clone()
                };
                envelope.data.extend_from_slice(data_line.as_bytes());
                envelope.data.extend_from_slice(b"\r\n");

                if config.limits.max_message_size > 0
                    && envelope.data.len() > config.limits.max_message_size
                {
                    send_response(&mut transport, &SmtpResponse::message_too_large()).await?;
                    state = SmtpState::Ready;
                    envelope.reset();
                    envelope.authenticated_identity = auth_identity.clone();
                }
            }
            continue;
        }

        // ── Parse + dispatch ───────────────────────────────────────
        command_count += 1;

        let cmd = match SmtpCommand::parse(&line) {
            Ok(c) => c,
            Err(e) => {
                send_response(&mut transport, &SmtpResponse::syntax_error(&e.to_string())).await?;
                continue;
            }
        };

        match handle_command(
            cmd,
            &mut state,
            &mut envelope,
            &mut ehlo_domain,
            &mut auth_exchange,
            &mut authenticated,
            &mut auth_identity,
            &handler,
            &config,
            &mut transport,
        )
        .await
        {
            Ok(ControlFlow::Quit) => break,
            Ok(ControlFlow::Continue) => {}
            Ok(ControlFlow::StartTls) => {
                #[cfg(feature = "tls")]
                {
                    if let Some(cert) = &config.tls_cert {
                        match transport.upgrade_tls(cert).await {
                            Ok(new_transport) => {
                                transport = new_transport;
                                edgerun_log::info!("edgerun-smtp: STARTTLS handshake complete");
                            }
                            Err(e) => {
                                edgerun_log::error!("edgerun-smtp: STARTTLS handshake failed: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                send_response(&mut transport, &SmtpResponse::syntax_error(&e.to_string())).await?;
            }
        }
    }

    Ok(())
}

// ===========================================================================
// AUTH Exchange
// ===========================================================================

async fn handle_auth_response(
    line: &str,
    handler: &Arc<dyn MailHandler>,
    transport: &mut SmtpTransport,
    authenticated: &mut bool,
    auth_identity: &mut Option<String>,
    state: AuthExchangeState,
) -> io::Result<AuthExchangeState> {
    match state {
        AuthExchangeState::Plain => {
            match base64_decode(line) {
                Ok(credentials) => {
                    match handler.authenticate("PLAIN", &credentials) {
                        AuthResult::Authenticated(identity) => {
                            *authenticated = true;
                            *auth_identity = Some(identity.clone());
                            edgerun_log::info!("edgerun-smtp: authenticated as {}", identity);
                            send_response(transport, &SmtpResponse::auth_success(&identity)).await?;
                            Ok(AuthExchangeState::Idle)
                        }
                        AuthResult::Failed => {
                            send_response(
                                transport,
                                &SmtpResponse::new(
                                    SmtpResponseCode::AUTHENTICATION_FAILED,
                                    "Authentication failed",
                                ),
                            )
                            .await?;
                            Ok(AuthExchangeState::Idle)
                        }
                        AuthResult::Unsupported => {
                            send_response(transport, &SmtpResponse::auth_required()).await?;
                            Ok(AuthExchangeState::Idle)
                        }
                    }
                }
                Err(e) => {
                    send_response(
                        transport,
                        &SmtpResponse::syntax_error(&format!("Invalid AUTH response: {}", e)),
                    )
                    .await?;
                    Ok(AuthExchangeState::Idle)
                }
            }
        }
        AuthExchangeState::LoginUsername => {
            let username = base64_decode_raw(line)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();
            send_response(
                transport,
                &SmtpResponse::auth_continue("UGFzc3dvcmQ6"),
            )
            .await?;
            Ok(AuthExchangeState::LoginPassword { username })
        }
        AuthExchangeState::LoginPassword { username } => {
            let password = base64_decode_raw(line)
                .ok()
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            let cred_bytes: Vec<u8> = [b"\0".as_slice(), username.as_bytes(), b"\0", password.as_bytes()]
                .concat();
            let creds = AuthCredentials::from_plain(&cred_bytes)?;
            match handler.authenticate("PLAIN", &creds) {
                AuthResult::Authenticated(identity) => {
                    *authenticated = true;
                    *auth_identity = Some(identity.clone());
                    edgerun_log::info!("edgerun-smtp: authenticated as {}", identity);
                    send_response(transport, &SmtpResponse::auth_success(&identity)).await?;
                    Ok(AuthExchangeState::Idle)
                }
                AuthResult::Failed => {
                    send_response(
                        transport,
                        &SmtpResponse::new(
                            SmtpResponseCode::AUTHENTICATION_FAILED,
                            "Authentication failed",
                        ),
                    )
                    .await?;
                    Ok(AuthExchangeState::Idle)
                }
                AuthResult::Unsupported => {
                    send_response(transport, &SmtpResponse::auth_required()).await?;
                    Ok(AuthExchangeState::Idle)
                }
            }
        }
        AuthExchangeState::Idle => Ok(AuthExchangeState::Idle),
    }
}

fn base64_decode(encoded: &str) -> io::Result<AuthCredentials> {
    let decoded = decode_base64(encoded)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid base64"))?;
    AuthCredentials::from_plain(&decoded)
}

fn base64_decode_raw(encoded: &str) -> io::Result<Vec<u8>> {
    decode_base64(encoded)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid base64"))
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input = input.trim();
    if input.is_empty() {
        return Some(Vec::new());
    }

    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits_collected: u32 = 0;

    for &byte in input.as_bytes() {
        if byte == b'=' {
            break;
        }
        let val = TABLE.iter().position(|&b| b == byte)? as u32;
        buf = (buf << 6) | val;
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            result.push(((buf >> bits_collected) & 0xFF) as u8);
        }
    }

    Some(result)
}

// ===========================================================================
// Command Dispatcher
// ===========================================================================

async fn handle_command(
    cmd: SmtpCommand,
    state: &mut SmtpState,
    envelope: &mut MailEnvelope,
    ehlo_domain: &mut Option<String>,
    auth_exchange: &mut AuthExchangeState,
    authenticated: &mut bool,
    auth_identity: &mut Option<String>,
    handler: &Arc<dyn MailHandler>,
    config: &SmtpServerConfig,
    transport: &mut SmtpTransport,
) -> io::Result<ControlFlow> {
    match cmd {
        SmtpCommand::Ehlo(domain) => {
            *ehlo_domain = Some(domain.clone());
            *state = SmtpState::Ready;

            let mut lines = vec![format!("Hello {}", domain)];
            for ext in crate::protocol::ESMTP_EXTENSIONS {
                lines.push(ext.to_string());
            }

            // AUTH extension (only if not already authenticated)
            if !*authenticated && !config.auth_mechanisms.is_empty() {
                let mech = config.auth_mechanisms.join(" ");
                lines.push(format!("AUTH {}", mech));
            }

            #[cfg(feature = "tls")]
            if !transport.is_tls() && config.tls_cert.is_some() {
                lines.push("STARTTLS".to_string());
            }

            send_multiline_response(transport, SmtpResponseCode::OK, lines).await?;
        }

        SmtpCommand::Helo(domain) => {
            *ehlo_domain = Some(domain.clone());
            *state = SmtpState::Ready;
            send_response(transport, &SmtpResponse::ok(&format!("Hello {}", domain))).await?;
        }

        SmtpCommand::MailFrom { address, parameters } => {
            if *state != SmtpState::Ready && *state != SmtpState::MailSet {
                send_response(transport, &SmtpResponse::bad_sequence("MAIL FROM not allowed in current state")).await?;
                return Ok(ControlFlow::Continue);
            }

            if handler.auth_required() && !*authenticated {
                send_response(transport, &SmtpResponse::auth_required()).await?;
                return Ok(ControlFlow::Continue);
            }

            if let Err(_e) = handler.validate_sender(&address) {
                send_response(transport, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            if config.limits.max_message_size > 0 {
                for (key, value) in &parameters {
                    if key == "SIZE" {
                        if let Some(s) = value {
                            if let Ok(size) = s.parse::<usize>() {
                                if size > config.limits.max_message_size {
                                    send_response(transport, &SmtpResponse::message_too_large()).await?;
                                    return Ok(ControlFlow::Continue);
                                }
                            }
                        }
                    }
                }
            }

            *envelope = MailEnvelope::new(address.clone());
            envelope.from_parameters = parameters;
            envelope.dsn_ret = extract_dsn_ret(&envelope.from_parameters).unwrap_or_default();
            envelope.dsn_envid = extract_dsn_envid(&envelope.from_parameters);
            envelope.authenticated_identity = auth_identity.clone();
            *state = SmtpState::MailSet;
            send_response(
                transport,
                &SmtpResponse::ok("Sender OK").with_enhanced(EnhancedStatusCode::MAIL_FROM_OK),
            )
            .await?;
        }

        SmtpCommand::RcptTo { address, parameters } => {
            if *state != SmtpState::MailSet && *state != SmtpState::RcptSet {
                send_response(transport, &SmtpResponse::bad_sequence("RCPT TO not allowed in current state")).await?;
                return Ok(ControlFlow::Continue);
            }

            if envelope.recipient_count() >= config.limits.max_recipients {
                send_response(
                    transport,
                    &SmtpResponse::too_many_recipients(envelope.recipient_count() + 1),
                )
                .await?;
                return Ok(ControlFlow::Continue);
            }

            if let Err(_e) = handler.validate_recipient(&address) {
                send_response(transport, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            let notify = extract_dsn_notify(&parameters).unwrap_or_default();
            let orcpt = extract_dsn_orcpt(&parameters);

            envelope.add_recipient(address, parameters, notify, orcpt);
            *state = SmtpState::RcptSet;
            send_response(
                transport,
                &SmtpResponse::ok("Recipient OK").with_enhanced(EnhancedStatusCode::RCPT_TO_OK),
            )
            .await?;
        }

        SmtpCommand::Data => {
            if *state != SmtpState::RcptSet {
                send_response(transport, &SmtpResponse::bad_sequence("No valid recipients")).await?;
                return Ok(ControlFlow::Continue);
            }
            *state = SmtpState::Data;
            send_response(transport, &SmtpResponse::start_mail_input()).await?;
        }

        SmtpCommand::Rset => {
            envelope.reset();
            envelope.authenticated_identity = auth_identity.clone();
            *state = SmtpState::Ready;
            send_response(transport, &SmtpResponse::ok("OK")).await?;
        }

        SmtpCommand::Noop => {
            send_response(transport, &SmtpResponse::ok("OK")).await?;
        }

        SmtpCommand::Quit => {
            *state = SmtpState::Quit;
            send_response(transport, &SmtpResponse::closing()).await?;
            return Ok(ControlFlow::Quit);
        }

        SmtpCommand::Starttls => {
            #[cfg(feature = "tls")]
            {
                if transport.is_tls() {
                    send_response(transport, &SmtpResponse::bad_sequence("TLS already active")).await?;
                    return Ok(ControlFlow::Continue);
                }
                if config.tls_cert.is_none() {
                    send_response(transport, &SmtpResponse::command_not_implemented("STARTTLS")).await?;
                    return Ok(ControlFlow::Continue);
                }
                send_response(transport, &SmtpResponse::ok("Ready to start TLS")).await?;
                edgerun_log::info!("edgerun-smtp: STARTTLS acknowledged, upgrading...");
                return Ok(ControlFlow::StartTls);
            }
            #[cfg(not(feature = "tls"))]
            {
                send_response(transport, &SmtpResponse::command_not_implemented("STARTTLS")).await?;
            }
        }

        SmtpCommand::Vrfy(_) => {
            send_response(transport, &SmtpResponse::vrfy_disabled()).await?;
        }

        SmtpCommand::Expn(_) => {
            send_response(transport, &SmtpResponse::expn_disabled()).await?;
        }

        SmtpCommand::Help(_topic) => {
            send_response(transport, &SmtpResponse::help_text(&config.domain)).await?;
        }

        SmtpCommand::Auth { mechanism, initial_response } => {
            if *authenticated {
                send_response(transport, &SmtpResponse::bad_sequence("Already authenticated")).await?;
                return Ok(ControlFlow::Continue);
            }

            let supported = config
                .auth_mechanisms
                .iter()
                .any(|m| m.eq_ignore_ascii_case(&mechanism));
            if !supported {
                send_response(transport, &SmtpResponse::auth_mechanism_unknown(&mechanism)).await?;
                return Ok(ControlFlow::Continue);
            }

            match mechanism.to_uppercase().as_str() {
                "PLAIN" => {
                    if let Some(response_b64) = initial_response {
                        match base64_decode(&response_b64) {
                            Ok(credentials) => {
                                match handler.authenticate("PLAIN", &credentials) {
                                    AuthResult::Authenticated(identity) => {
                                        *authenticated = true;
                                        *auth_identity = Some(identity.clone());
                                        edgerun_log::info!("edgerun-smtp: authenticated as {}", identity);
                                        send_response(transport, &SmtpResponse::auth_success(&identity)).await?;
                                    }
                                    AuthResult::Failed => {
                                        send_response(
                                            transport,
                                            &SmtpResponse::new(
                                                SmtpResponseCode::AUTHENTICATION_FAILED,
                                                "Authentication failed",
                                            ),
                                        )
                                        .await?;
                                    }
                                    AuthResult::Unsupported => {
                                        send_response(transport, &SmtpResponse::auth_required()).await?;
                                    }
                                }
                            }
                            Err(e) => {
                                send_response(
                                    transport,
                                    &SmtpResponse::syntax_error(&format!("Invalid base64: {}", e)),
                                )
                                .await?;
                            }
                        }
                    } else {
                        // Multi-step: send empty challenge, wait for PLAIN response
                        send_response(transport, &SmtpResponse::auth_continue("")).await?;
                        *auth_exchange = AuthExchangeState::Plain;
                    }
                }
                "LOGIN" => {
                    // Two-step challenge: "Username:" then "Password:"
                    send_response(transport, &SmtpResponse::auth_continue("VXNlcm5hbWU6")).await?;
                    *auth_exchange = AuthExchangeState::LoginUsername;
                }
                _ => {
                    send_response(transport, &SmtpResponse::auth_mechanism_unknown(&mechanism)).await?;
                }
            }
        }

        // AuthResponse should only arrive during an AUTH exchange,
        // which is now handled by the state machine in handle_auth_response.
        // If we get here outside an exchange, it's a syntax error.
        SmtpCommand::AuthResponse(_) => {
            send_response(
                transport,
                &SmtpResponse::syntax_error("Unexpected AUTH response outside AUTH exchange"),
            )
            .await?;
        }
    }

    Ok(ControlFlow::Continue)
}

// ===========================================================================
// Response Helpers
// ===========================================================================

async fn send_response(transport: &mut SmtpTransport, response: &SmtpResponse) -> io::Result<()> {
    let formatted = response.format();
    transport.write_all(formatted.as_bytes()).await?;
    transport.flush().await?;
    Ok(())
}

async fn send_multiline_response(
    transport: &mut SmtpTransport,
    code: SmtpResponseCode,
    lines: Vec<String>,
) -> io::Result<()> {
    let response = SmtpResponse::multiline(code, lines);
    send_response(transport, &response).await
}
