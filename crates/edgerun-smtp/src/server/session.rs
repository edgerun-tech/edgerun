//! Per-connection session handling — command loop, state machine, SASL AUTH, enforcement.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use edgerun_rt::{AsyncTcpStream, AsyncWriteExt, CancellationToken, Mutex};

use crate::protocol::SmtpReader;
use crate::server::handler::{AuthCredentials, AuthResult, MailHandler};
use crate::types::command::{extract_dsn_envid, extract_dsn_notify, extract_dsn_orcpt, extract_dsn_ret};
use crate::types::{
    EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse, SmtpResponseCode,
    SmtpState,
};

#[cfg(feature = "tls")]
use edgerun_tls::CertificateAndKey;

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
}

async fn handle_connection(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    handler: Arc<dyn MailHandler>,
    config: SmtpServerConfig,
    _shutdown: CancellationToken,
) -> io::Result<()> {
    let (reader_raw, writer_raw) = stream.split();
    let writer = Arc::new(Mutex::new(writer_raw));

    let greeting = SmtpResponse::service_ready(&config.domain);
    send_response(&writer, &greeting).await?;

    let mut buf_reader = SmtpReader::new(reader_raw);

    let mut state = SmtpState::Connected;
    let mut envelope = MailEnvelope::new(String::new());
    let mut ehlo_domain: Option<String> = None;
    let mut tls_active = false;
    let mut command_count: usize = 0;
    let mut last_activity = std::time::Instant::now();

    // ── Auth state ────────────────────────────────────────────────────
    let mut authenticated: bool = false;
    let mut auth_identity: Option<String> = None;
    let mut in_auth_exchange: bool = false;

    loop {
        // Idle timeout
        let elapsed = last_activity.elapsed().as_secs();
        if elapsed > config.limits.idle_timeout_secs {
            edgerun_log::info!(
                "edgerun-smtp: {} idle timeout ({}s > {}s)",
                peer, elapsed, config.limits.idle_timeout_secs
            );
            send_response(
                &writer,
                &SmtpResponse::new(SmtpResponseCode::CLOSING, "Idle timeout"),
            )
            .await?;
            break;
        }

        // Command count limit
        if command_count >= config.limits.max_commands {
            edgerun_log::info!("edgerun-smtp: {} exceeded max commands", peer);
            send_response(&writer, &SmtpResponse::bad_sequence("Too many commands")).await?;
            break;
        }

        let line = match buf_reader.read_line().await? {
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
                &writer,
                &SmtpResponse::line_too_long(line.len(), config.limits.max_line_length),
            )
            .await?;
            continue;
        }

        // ── AUTH challenge/response exchange ────────────────────────
        if in_auth_exchange {
            in_auth_exchange = handle_auth_response(
                &line,
                &handler,
                &writer,
                &mut authenticated,
                &mut auth_identity,
            )
            .await?;
            if in_auth_exchange {
                // Still in exchange — don't parse as command
                continue;
            }
            // Auth done — fall through to normal command processing
            // but skip this line (it was the final response)
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
                            &writer,
                            &SmtpResponse::ok("OK: queued")
                                .with_enhanced(EnhancedStatusCode::QUEUED),
                        )
                        .await?;
                    }
                    Err(e) => {
                        edgerun_log::error!("edgerun-smtp: delivery failed: {}", e);
                        send_response(
                            &writer,
                            &SmtpResponse::transient_failure("Delivery failed"),
                        )
                        .await?;
                    }
                }
                envelope.reset();
                // Preserve auth identity across reset
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
                    send_response(&writer, &SmtpResponse::message_too_large()).await?;
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
                send_response(&writer, &SmtpResponse::syntax_error(&e.to_string())).await?;
                continue;
            }
        };

        match handle_command(
            cmd,
            &mut state,
            &mut envelope,
            &mut ehlo_domain,
            &mut tls_active,
            &mut in_auth_exchange,
            &mut authenticated,
            &mut auth_identity,
            &handler,
            &config,
            &writer,
        )
        .await
        {
            Ok(ControlFlow::Quit) => break,
            Ok(ControlFlow::Continue) => {}
            Err(e) => {
                send_response(&writer, &SmtpResponse::syntax_error(&e.to_string())).await?;
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
    writer: &Arc<Mutex<edgerun_rt::AsyncWriteHalf>>,
    authenticated: &mut bool,
    auth_identity: &mut Option<String>,
) -> io::Result<bool> {
    // line is the base64 response from the client
    // Try to decode and authenticate
    match base64_decode(line) {
        Ok(credentials) => {
            match handler.authenticate("PLAIN", &credentials) {
                AuthResult::Authenticated(identity) => {
                    *authenticated = true;
                    *auth_identity = Some(identity.clone());
                    edgerun_log::info!("edgerun-smtp: authenticated as {}", identity);
                    send_response(
                        writer,
                        &SmtpResponse::auth_success(&identity),
                    )
                    .await?;
                    Ok(false) // exchange complete
                }
                AuthResult::Failed => {
                    send_response(
                        writer,
                        &SmtpResponse::new(
                            SmtpResponseCode::AUTHENTICATION_FAILED,
                            "Authentication failed",
                        ),
                    )
                    .await?;
                    Ok(false)
                }
                AuthResult::Unsupported => {
                    send_response(writer, &SmtpResponse::auth_required()).await?;
                    Ok(false)
                }
            }
        }
        Err(e) => {
            send_response(
                writer,
                &SmtpResponse::syntax_error(&format!("Invalid AUTH response: {}", e)),
            )
            .await?;
            Ok(false)
        }
    }
}

/// Decode base64 credentials from an AUTH exchange.
fn base64_decode(encoded: &str) -> io::Result<AuthCredentials> {
    // Use the standard library's base64-like approach or a simple decoder.
    // Since we can't add external deps, implement a minimal base64 decoder.
    let decoded = decode_base64(encoded)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid base64"))?;
    AuthCredentials::from_plain(&decoded)
}

/// Minimal base64 decoder (RFC 4648, standard alphabet).
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
            break; // padding
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

#[allow(clippy::too_many_arguments)]
async fn handle_command(
    cmd: SmtpCommand,
    state: &mut SmtpState,
    envelope: &mut MailEnvelope,
    ehlo_domain: &mut Option<String>,
    tls_active: &mut bool,
    in_auth_exchange: &mut bool,
    authenticated: &mut bool,
    auth_identity: &mut Option<String>,
    handler: &Arc<dyn MailHandler>,
    config: &SmtpServerConfig,
    writer: &Arc<Mutex<edgerun_rt::AsyncWriteHalf>>,
) -> io::Result<ControlFlow> {
    match cmd {
        SmtpCommand::Ehlo(domain) => {
            *ehlo_domain = Some(domain.clone());
            *state = SmtpState::Ready;

            let mut lines = vec![format!("Hello {}", domain)];
            for ext in crate::protocol::ESMTP_EXTENSIONS {
                lines.push(ext.to_string());
            }

            // AUTH extension (only if not already authenticated and mechanisms available)
            if !*authenticated && !config.auth_mechanisms.is_empty() {
                let mech = config.auth_mechanisms.join(" ");
                lines.push(format!("AUTH {}", mech));
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
                send_response(writer, &SmtpResponse::bad_sequence("MAIL FROM not allowed in current state")).await?;
                return Ok(ControlFlow::Continue);
            }

            // Require AUTH if configured
            if handler.auth_required() && !*authenticated {
                send_response(writer, &SmtpResponse::auth_required()).await?;
                return Ok(ControlFlow::Continue);
            }

            if let Err(_e) = handler.validate_sender(&address) {
                send_response(writer, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            if config.limits.max_message_size > 0 {
                for (key, value) in &parameters {
                    if key == "SIZE" {
                        if let Some(s) = value {
                            if let Ok(size) = s.parse::<usize>() {
                                if size > config.limits.max_message_size {
                                    send_response(writer, &SmtpResponse::message_too_large()).await?;
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
                writer,
                &SmtpResponse::ok("Sender OK").with_enhanced(EnhancedStatusCode::MAIL_FROM_OK),
            )
            .await?;
        }

        SmtpCommand::RcptTo { address, parameters } => {
            if *state != SmtpState::MailSet && *state != SmtpState::RcptSet {
                send_response(writer, &SmtpResponse::bad_sequence("RCPT TO not allowed in current state")).await?;
                return Ok(ControlFlow::Continue);
            }

            if envelope.recipient_count() >= config.limits.max_recipients {
                send_response(
                    writer,
                    &SmtpResponse::too_many_recipients(envelope.recipient_count() + 1),
                )
                .await?;
                return Ok(ControlFlow::Continue);
            }

            if let Err(_e) = handler.validate_recipient(&address) {
                send_response(writer, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            let notify = extract_dsn_notify(&parameters).unwrap_or_default();
            let orcpt = extract_dsn_orcpt(&parameters);

            envelope.add_recipient(address, parameters, notify, orcpt);
            *state = SmtpState::RcptSet;
            send_response(
                writer,
                &SmtpResponse::ok("Recipient OK").with_enhanced(EnhancedStatusCode::RCPT_TO_OK),
            )
            .await?;
        }

        SmtpCommand::Data => {
            if *state != SmtpState::RcptSet {
                send_response(writer, &SmtpResponse::bad_sequence("No valid recipients")).await?;
                return Ok(ControlFlow::Continue);
            }
            *state = SmtpState::Data;
            send_response(writer, &SmtpResponse::start_mail_input()).await?;
        }

        SmtpCommand::Rset => {
            envelope.reset();
            envelope.authenticated_identity = auth_identity.clone();
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
                *tls_active = true;
                edgerun_log::info!("edgerun-smtp: STARTTLS acknowledged");
            }
            #[cfg(not(feature = "tls"))]
            {
                send_response(writer, &SmtpResponse::command_not_implemented("STARTTLS")).await?;
            }
        }

        SmtpCommand::Vrfy(_) => {
            send_response(writer, &SmtpResponse::vrfy_disabled()).await?;
        }

        SmtpCommand::Expn(_) => {
            send_response(writer, &SmtpResponse::expn_disabled()).await?;
        }

        SmtpCommand::Help(_topic) => {
            send_response(writer, &SmtpResponse::help_text(&config.domain)).await?;
        }

        SmtpCommand::Auth { mechanism, initial_response } => {
            if *authenticated {
                send_response(writer, &SmtpResponse::bad_sequence("Already authenticated")).await?;
                return Ok(ControlFlow::Continue);
            }

            // Check if mechanism is supported
            let supported = config
                .auth_mechanisms
                .iter()
                .any(|m| m.eq_ignore_ascii_case(&mechanism));
            if !supported {
                send_response(writer, &SmtpResponse::auth_mechanism_unknown(&mechanism)).await?;
                return Ok(ControlFlow::Continue);
            }

            match mechanism.to_uppercase().as_str() {
                "PLAIN" => {
                    if let Some(response_b64) = initial_response {
                        // Inline authentication — response provided on same line
                        match base64_decode(&response_b64) {
                            Ok(credentials) => {
                                match handler.authenticate("PLAIN", &credentials) {
                                    AuthResult::Authenticated(identity) => {
                                        *authenticated = true;
                                        *auth_identity = Some(identity.clone());
                                        edgerun_log::info!("edgerun-smtp: authenticated as {}", identity);
                                        send_response(writer, &SmtpResponse::auth_success(&identity)).await?;
                                    }
                                    AuthResult::Failed => {
                                        send_response(
                                            writer,
                                            &SmtpResponse::new(
                                                SmtpResponseCode::AUTHENTICATION_FAILED,
                                                "Authentication failed",
                                            ),
                                        )
                                        .await?;
                                    }
                                    AuthResult::Unsupported => {
                                        send_response(writer, &SmtpResponse::auth_required()).await?;
                                    }
                                }
                            }
                            Err(e) => {
                                send_response(
                                    writer,
                                    &SmtpResponse::syntax_error(&format!("Invalid base64: {}", e)),
                                )
                                .await?;
                            }
                        }
                    } else {
                        // Multi-step — send challenge
                        send_response(writer, &SmtpResponse::auth_continue("")).await?;
                        *in_auth_exchange = true;
                    }
                }
                "LOGIN" => {
                    if initial_response.is_some() {
                        // Inline: LOGIN <base64_username> — expect password next
                        send_response(writer, &SmtpResponse::auth_continue("VXNlcm5hbWU6")).await?; // "Username:"
                        *in_auth_exchange = true;
                    } else {
                        send_response(writer, &SmtpResponse::auth_continue("VXNlcm5hbWU6")).await?; // "Username:"
                        *in_auth_exchange = true;
                    }
                }
                _ => {
                    send_response(writer, &SmtpResponse::auth_mechanism_unknown(&mechanism)).await?;
                }
            }
        }

        SmtpCommand::AuthResponse(response_b64) => {
            // Handle during LOGIN username challenge
            // The handler will process this as a PLAIN credential since we don't
            // distinguish LOGIN state here
            match base64_decode(&response_b64) {
                Ok(credentials) => {
                    match handler.authenticate("PLAIN", &credentials) {
                        AuthResult::Authenticated(identity) => {
                            *authenticated = true;
                            *auth_identity = Some(identity.clone());
                            send_response(writer, &SmtpResponse::auth_success(&identity)).await?;
                        }
                        AuthResult::Failed => {
                            send_response(
                                writer,
                                &SmtpResponse::new(
                                    SmtpResponseCode::AUTHENTICATION_FAILED,
                                    "Authentication failed",
                                ),
                            )
                            .await?;
                        }
                        AuthResult::Unsupported => {
                            send_response(writer, &SmtpResponse::auth_required()).await?;
                        }
                    }
                }
                Err(_) => {
                    send_response(
                        writer,
                        &SmtpResponse::syntax_error("Invalid base64 in AUTH response"),
                    )
                    .await?;
                }
            }
        }
    }

    Ok(ControlFlow::Continue)
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
