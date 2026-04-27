//! LMTP server implementation (RFC 2033).
//!
//! LMTP differs from SMTP in two key ways:
//! 1. Greeting: "220 <domain> LMTP ready"
//! 2. Per-recipient delivery: after DATA, each recipient gets an individual
//!    response instead of a single batch response.
//!
//! LMTP does NOT support: AUTH, STARTTLS, VRFY, EXPN, RSET, TURN, ETRN, BDAT.
//! It is a local delivery protocol — no queueing, no relaying.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::rt::{AsyncReadExt, AsyncTcpStream, AsyncWriteExt, CancellationToken};

use crate::command_middleware::{
    CommandMiddleware, ControlFlow as MwControlFlow, NextCommand, SessionExtensions,
};
use crate::server::read_line;
use crate::server::ConnectionInterceptor;
use crate::smtp::server::{MailHandler, MemoryMailStore};
use crate::smtp::types::{
    DsnNotify, EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse,
    SmtpResponseCode, SmtpState,
};
use edgerun_email_auth::EmailAuthEvaluator;

// ===========================================================================
// Server Configuration
// ===========================================================================

#[derive(Clone)]
pub struct LmtpServerConfig {
    pub bind_addr: String,
    pub domain: String,
    pub limits: ServerLimits,
}

impl Default for LmtpServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:24".to_string(), // LMTP default port
            domain: "edgerun.mail".to_string(),
            limits: ServerLimits::default(),
        }
    }
}

// ===========================================================================
// LMTP Server
// ===========================================================================

pub struct LmtpServer {
    listener: Arc<crate::rt::AsyncTcpListener>,
    handler: Arc<dyn MailHandler>,
    config: LmtpServerConfig,
    /// Connection interceptor (IP filter, rate limit, etc.).
    connection_interceptor: Option<Arc<dyn ConnectionInterceptor>>,
    /// Command middleware layers. Composed with the handler at runtime
    /// so middleware can capture mutable session state.
    command_middleware: Vec<Arc<dyn CommandMiddleware<SmtpCommand, SmtpResponse>>>,
}

impl LmtpServer {
    pub fn new(config: LmtpServerConfig, handler: Arc<dyn MailHandler>) -> io::Result<Self> {
        let listener = Arc::new(crate::rt::AsyncTcpListener::bind(&config.bind_addr)?);
        edgerun_log::info!("edgerun-lmtp: listening on {}", config.bind_addr);
        Ok(Self {
            listener,
            handler,
            config,
            connection_interceptor: None,
            command_middleware: Vec::new(),
        })
    }

    /// Add a command middleware layer.
    ///
    /// Middleware runs in order: first added = outermost (sees command first).
    pub fn with_command_middleware<M: CommandMiddleware<SmtpCommand, SmtpResponse>>(
        mut self,
        mw: M,
    ) -> Self {
        self.command_middleware.push(Arc::new(mw));
        self
    }

    /// Set the connection interceptor.
    ///
    /// The interceptor runs on every new TCP connection before protocol
    /// parsing. It can reject connections (IP filter, rate limit) or
    /// pass them through to the LMTP handler.
    pub fn with_connection_interceptor(
        mut self,
        interceptor: Arc<dyn ConnectionInterceptor>,
    ) -> Self {
        self.connection_interceptor = Some(interceptor);
        self
    }

    pub fn with_memory_store(config: LmtpServerConfig) -> io::Result<Self> {
        let store = Arc::new(MemoryMailStore::new());
        Self::new(config, store)
    }

    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match self.listener.accept().await {
                Ok((stream, peer)) => {
                    // Connection interceptor (IP filter, rate limit, etc.)
                    if let Some(ref interceptor) = self.connection_interceptor {
                        let interceptor = Arc::clone(interceptor);
                        let peer_addr = peer;
                        let stream_ref = Arc::clone(&stream);
                        match interceptor.intercept(peer_addr, stream_ref).await {
                            Ok(()) => {}
                            Err(e) => {
                                edgerun_log::info!(
                                    "edgerun-lmtp: connection from {} rejected: {}",
                                    peer,
                                    e
                                );
                                continue;
                            }
                        }
                    }

                    let handler = Arc::clone(&self.handler);
                    let config = self.config.clone();
                    let shutdown = shutdown.clone();
                    let command_middleware = self.command_middleware.clone();

                    edgerun_log::info!("edgerun-lmtp: connection from {}", peer);
                    crate::rt::spawn(async move {
                        if let Err(e) = handle_connection(
                            stream,
                            peer,
                            handler,
                            config,
                            shutdown,
                            command_middleware,
                        )
                        .await
                        {
                            edgerun_log::error!("edgerun-lmtp: connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("edgerun-lmtp: accept error: {}", e);
                    crate::rt::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        }
        edgerun_log::info!("edgerun-lmtp: server shut down");
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

/// Evaluate SPF/DKIM/DMARC and notify the handler.
async fn evaluate_and_notify_auth(
    handler: &Arc<dyn MailHandler>,
    envelope: &MailEnvelope,
    domain: &str,
    peer_ip: &str,
) {
    // Extract header From address
    let data_str = String::from_utf8_lossy(&envelope.data);
    let headers_str = if let Some(pos) = data_str.find("\r\n\r\n") {
        data_str[..pos].as_bytes()
    } else {
        data_str.as_bytes()
    };

    // Parse From: header
    let header_from =
        crate::smtp::types::headers::get_from_address(headers_str).unwrap_or_default();

    // Get a DNS client for evaluation
    let mut dns_client = match edgerun_dns::client::DnsClient::new("8.8.8.8:53") {
        Ok(c) => c,
        Err(e) => {
            edgerun_log::warn!("edgerun-email-auth: failed to create DNS client: {}", e);
            return;
        }
    };

    let mut evaluator = EmailAuthEvaluator::new(&mut dns_client);
    match evaluator
        .evaluate(
            peer_ip,
            &envelope.from,
            &header_from,
            headers_str,
            &envelope.data,
        )
        .await
    {
        Ok(auth_results) => {
            // Log the results
            let header_value = auth_results.to_header_value(domain);
            edgerun_log::info!("edgerun-lmtp: Authentication-Results: {}", header_value);
            handler.on_mail_received(envelope, &auth_results);
        }
        Err(e) => {
            edgerun_log::warn!("edgerun-email-auth: evaluation failed: {}", e);
        }
    }
}

async fn handle_connection(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    handler: Arc<dyn MailHandler>,
    config: LmtpServerConfig,
    _shutdown: CancellationToken,
    command_middleware: Vec<Arc<dyn CommandMiddleware<SmtpCommand, SmtpResponse>>>,
) -> io::Result<()> {
    let mut stream = match Arc::try_unwrap(stream) {
        Ok(s) => s,
        Err(_) => {
            edgerun_log::error!("edgerun-lmtp: stream has multiple references");
            return Err(io::Error::other("stream reference error"));
        }
    };

    // LMTP greeting
    let greeting = format!("220 {} LMTP ready\r\n", config.domain);
    stream.write_all(greeting.as_bytes()).await?;
    stream.flush().await?;

    let mut state = SmtpState::Connected;
    let mut envelope = MailEnvelope::new(String::new());
    let mut lhlo_domain: Option<String> = None;
    let mut command_count: usize = 0;
    let mut last_activity = std::time::Instant::now();
    let mut in_data_phase = false;

    loop {
        // Idle timeout
        let elapsed = last_activity.elapsed().as_secs();
        if elapsed > config.limits.idle_timeout_secs {
            edgerun_log::info!("edgerun-lmtp: {} idle timeout ({}s)", peer, elapsed);
            send_response(
                &mut stream,
                &SmtpResponse::new(SmtpResponseCode::CLOSING, "Idle timeout"),
            )
            .await?;
            break;
        }

        if command_count >= config.limits.max_commands {
            send_response(
                &mut stream,
                &SmtpResponse::bad_sequence("Too many commands"),
            )
            .await?;
            break;
        }

        // ── DATA phase: read body lines until "." ─────────────────
        if in_data_phase {
            let line = match read_line(&mut stream).await {
                Ok(Some(l)) => l,
                Ok(None) => break,
                Err(e) => {
                    edgerun_log::error!("edgerun-lmtp: read error during DATA: {}", e);
                    break;
                }
            };

            if line == "." {
                // End of data — deliver per-recipient (RFC 2033 §3.3)
                command_count += 1;
                in_data_phase = false;
                let peer_ip_str = peer.ip().to_string();
                let domain = config.domain.clone();

                for recipient in &envelope.recipients {
                    // Create a single-recipient envelope for delivery
                    let mut single_envelope = envelope.clone();
                    single_envelope.recipients = vec![recipient.clone()];

                    match handler.validate_recipient(recipient) {
                        Ok(()) => {
                            match handler.accept_mail(&single_envelope) {
                                Ok(()) => {
                                    send_response(
                                        &mut stream,
                                        &SmtpResponse::ok(&format!("OK: queued for {}", recipient))
                                            .with_enhanced(EnhancedStatusCode::QUEUED),
                                    )
                                    .await?;
                                    edgerun_log::info!("edgerun-lmtp: delivered to {}", recipient,);
                                    // Evaluate SPF/DKIM/DMARC in background (first recipient only)
                                    if recipient == &envelope.recipients[0] {
                                        let handler_clone = Arc::clone(&handler);
                                        let envelope_clone = envelope.clone();
                                        let domain_clone = domain.clone();
                                        let peer_ip_clone = peer_ip_str.clone();
                                        crate::rt::spawn(async move {
                                            evaluate_and_notify_auth(
                                                &handler_clone,
                                                &envelope_clone,
                                                &domain_clone,
                                                &peer_ip_clone,
                                            )
                                            .await;
                                        });
                                    }
                                }
                                Err(e) => {
                                    send_response(
                                        &mut stream,
                                        &SmtpResponse::transient_failure(&format!(
                                            "Delivery failed for {}: {}",
                                            recipient, e
                                        )),
                                    )
                                    .await?;
                                    edgerun_log::error!(
                                        "edgerun-lmtp: delivery failed to {}: {}",
                                        recipient,
                                        e,
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            send_response(&mut stream, &SmtpResponse::mailbox_not_found(recipient))
                                .await?;
                        }
                    }
                }

                state = SmtpState::Ready;
                envelope.reset();
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
                    send_response(&mut stream, &SmtpResponse::message_too_large()).await?;
                    state = SmtpState::Ready;
                    in_data_phase = false;
                    envelope.reset();
                }
            }
            continue;
        }

        // ── Read and parse command ────────────────────────────────
        let line = match read_line(&mut stream).await {
            Ok(Some(l)) => l,
            Ok(None) => {
                edgerun_log::info!("edgerun-lmtp: {} disconnected", peer);
                break;
            }
            Err(e) => {
                edgerun_log::error!("edgerun-lmtp: read error: {}", e);
                break;
            }
        };

        last_activity = std::time::Instant::now();
        command_count += 1;

        let cmd = match SmtpCommand::parse(&line) {
            Ok(c) => c,
            Err(e) => {
                send_response(&mut stream, &SmtpResponse::syntax_error(&e.to_string())).await?;
                continue;
            }
        };

        // ── Middleware pre-filter (if configured) ──────────────────
        if !command_middleware.is_empty() {
            let session = SessionExtensions::new();

            let mut blocked = false;
            for mw in &command_middleware {
                let mw = Arc::clone(mw);
                let cmd_for_mw = cmd.clone();
                let session_for_mw = session.clone();
                let next = NextCommand::new(|_cmd, _session| {
                    Box::pin(async move { Ok(MwControlFlow::Continue) })
                });

                match mw.handle(cmd_for_mw, session_for_mw, next).await {
                    Ok(MwControlFlow::Respond(resp)) => {
                        send_response(&mut stream, &resp).await?;
                        blocked = true;
                        break;
                    }
                    Ok(MwControlFlow::Continue) => {}
                    Err(e) => {
                        send_response(&mut stream, &SmtpResponse::syntax_error(&e.to_string()))
                            .await?;
                        blocked = true;
                        break;
                    }
                }
            }
            if blocked {
                continue;
            }
        }

        match handle_command(
            cmd,
            &mut state,
            &mut envelope,
            &mut lhlo_domain,
            &handler,
            &config,
            &mut stream,
        )
        .await
        {
            Ok(ControlFlow::Quit) => break,
            Ok(ControlFlow::Continue) => {}
            Err(e) => {
                send_response(&mut stream, &SmtpResponse::syntax_error(&e.to_string())).await?;
            }
        }

        if state == SmtpState::Data {
            in_data_phase = true;
        }
    }

    Ok(())
}

// ===========================================================================
// Command Dispatcher
// ===========================================================================

async fn handle_command(
    cmd: SmtpCommand,
    state: &mut SmtpState,
    envelope: &mut MailEnvelope,
    lhlo_domain: &mut Option<String>,
    handler: &Arc<dyn MailHandler>,
    config: &LmtpServerConfig,
    stream: &mut (impl AsyncReadExt + AsyncWriteExt),
) -> io::Result<ControlFlow> {
    match cmd {
        SmtpCommand::Ehlo(domain) | SmtpCommand::Helo(domain) => {
            // LMTP uses LHLO but accepts EHLO/HELO for compatibility
            *lhlo_domain = Some(domain.clone());
            *state = SmtpState::Ready;

            let lines = vec![
                format!("Hello {}", domain),
                format!("SIZE {}", config.limits.max_message_size),
                "8BITMIME".to_string(),
                "ENHANCEDSTATUSCODES".to_string(),
                "SMTPUTF8".to_string(),
            ];

            send_multiline_response(stream, SmtpResponseCode::OK, lines).await?;
        }

        SmtpCommand::MailFrom {
            address,
            parameters,
        } => {
            if *state != SmtpState::Ready && *state != SmtpState::MailSet {
                send_response(
                    stream,
                    &SmtpResponse::bad_sequence("MAIL FROM not allowed in current state"),
                )
                .await?;
                return Ok(ControlFlow::Continue);
            }

            // Check SIZE parameter
            if config.limits.max_message_size > 0 {
                for (key, value) in &parameters {
                    if key == "SIZE" {
                        if let Some(s) = value {
                            if let Ok(size) = s.parse::<usize>() {
                                if size > config.limits.max_message_size {
                                    send_response(stream, &SmtpResponse::message_too_large())
                                        .await?;
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
            send_response(
                stream,
                &SmtpResponse::ok("Sender OK").with_enhanced(EnhancedStatusCode::MAIL_FROM_OK),
            )
            .await?;
        }

        SmtpCommand::RcptTo {
            address,
            parameters,
        } => {
            if *state != SmtpState::MailSet && *state != SmtpState::RcptSet {
                send_response(
                    stream,
                    &SmtpResponse::bad_sequence("RCPT TO not allowed in current state"),
                )
                .await?;
                return Ok(ControlFlow::Continue);
            }

            if envelope.recipient_count() >= config.limits.max_recipients {
                send_response(
                    stream,
                    &SmtpResponse::too_many_recipients(envelope.recipient_count() + 1),
                )
                .await?;
                return Ok(ControlFlow::Continue);
            }

            if let Err(_e) = handler.validate_recipient(&address) {
                send_response(stream, &SmtpResponse::mailbox_not_found(&address)).await?;
                return Ok(ControlFlow::Continue);
            }

            envelope.add_recipient(address, parameters, DsnNotify::default(), None);
            *state = SmtpState::RcptSet;
            send_response(
                stream,
                &SmtpResponse::ok("Recipient OK").with_enhanced(EnhancedStatusCode::RCPT_TO_OK),
            )
            .await?;
        }

        SmtpCommand::Data => {
            if *state != SmtpState::RcptSet {
                send_response(stream, &SmtpResponse::bad_sequence("No valid recipients")).await?;
                return Ok(ControlFlow::Continue);
            }
            *state = SmtpState::Data;
            send_response(stream, &SmtpResponse::start_mail_input()).await?;
        }

        SmtpCommand::Noop => {
            send_response(stream, &SmtpResponse::ok("OK")).await?;
        }

        SmtpCommand::Quit => {
            *state = SmtpState::Quit;
            send_response(stream, &SmtpResponse::closing()).await?;
            return Ok(ControlFlow::Quit);
        }

        // LMTP does not support these — reject with 502
        SmtpCommand::Starttls
        | SmtpCommand::Auth { .. }
        | SmtpCommand::AuthResponse(_)
        | SmtpCommand::Vrfy(_)
        | SmtpCommand::Expn(_)
        | SmtpCommand::Help(_)
        | SmtpCommand::Rset
        | SmtpCommand::Turn
        | SmtpCommand::Etrn(_)
        | SmtpCommand::Bdat { .. } => {
            send_response(stream, &SmtpResponse::command_not_implemented("LMTP")).await?;
        }
    }

    Ok(ControlFlow::Continue)
}

// ===========================================================================
// Response Helpers
// ===========================================================================

async fn send_response(
    stream: &mut (impl AsyncWriteExt),
    response: &SmtpResponse,
) -> io::Result<()> {
    let formatted = response.format();
    stream.write_all(formatted.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_multiline_response(
    stream: &mut (impl AsyncWriteExt),
    code: SmtpResponseCode,
    lines: Vec<String>,
) -> io::Result<()> {
    let response = SmtpResponse::multiline(code, lines);
    send_response(stream, &response).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmtp_config_default() {
        let config = LmtpServerConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:24");
    }

    #[test]
    fn test_enhanced_status_codes_available() {
        // Verify we can construct enhanced status codes from edgerun-smtp
        let code = EnhancedStatusCode::new(2, 6, 0);
        assert_eq!(code.to_string(), "2.6.0");
    }
}
