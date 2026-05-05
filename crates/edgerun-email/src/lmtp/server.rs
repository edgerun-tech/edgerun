//! LMTP server implementation (RFC 2033).
//!
//! LMTP differs from SMTP in two key ways:
//! 1. Greeting: "220 <domain> LMTP ready"
//! 2. Per-recipient delivery: after DATA, each recipient gets an individual
//!    response instead of a single batch response.
//!
//! LMTP does NOT support: AUTH, STARTTLS, VRFY, EXPN, RSET, TURN, ETRN, BDAT.
//! It is a local delivery protocol — no queueing, no relaying.

use crate::prelude::*;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::rt::{AsyncReadExt, AsyncTcpStream, AsyncWriteExt, CancellationToken};

use crate::command_middleware::{
    CommandMiddleware, ControlFlow as MwControlFlow, NextCommand, SessionExtensions,
};
use crate::lmtp::session_core::{
    LmtpCommand, LmtpSessionAction, LmtpSessionConfig, LmtpSessionCore, LmtpSessionPolicy,
};
use crate::server::read_line;
use crate::server::ConnectionInterceptor;
use crate::smtp::server::{MailHandler, MemoryMailStore};
use crate::smtp::types::{
    EnhancedStatusCode, MailEnvelope, ServerLimits, SmtpCommand, SmtpResponse, SmtpResponseCode,
    SmtpState,
};
#[cfg(feature = "dkim")]
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

struct LmtpHandlerPolicy<'a> {
    handler: &'a Arc<dyn MailHandler>,
}

impl LmtpSessionPolicy for LmtpHandlerPolicy<'_> {
    fn validate_recipient(&self, address: &str) -> bool {
        self.handler.validate_recipient(address).is_ok()
    }
}

/// Evaluate SPF/DKIM/DMARC and notify the handler.
#[cfg(feature = "dkim")]
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

    let mut core = LmtpSessionCore::new(LmtpSessionConfig {
        domain: config.domain.clone(),
        limits: config.limits.clone(),
    });
    send_response(&mut stream, &core.greeting()).await?;

    let mut command_count: usize = 0;
    let mut last_activity = std::time::Instant::now();
    let mut in_data_phase = false;
    let policy = LmtpHandlerPolicy { handler: &handler };

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

            let step = core.handle_line(&line, &policy);
            for response in &step.responses {
                send_response(&mut stream, response).await?;
            }

            if step.action == LmtpSessionAction::Deliver {
                command_count += 1;
                in_data_phase = false;
                let peer_ip_str = peer.ip().to_string();
                let domain = config.domain.clone();

                for recipient in &core.envelope.recipients {
                    // Create a single-recipient envelope for delivery
                    let mut single_envelope = core.envelope.clone();
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
                                    #[cfg(feature = "dkim")]
                                    if recipient == &core.envelope.recipients[0] {
                                        let handler_clone = Arc::clone(&handler);
                                        let envelope_clone = core.envelope.clone();
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

                core.reset_transaction();
            } else if !matches!(step.action, LmtpSessionAction::Continue) {
                in_data_phase = false;
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

        let lmtp_cmd = match LmtpCommand::parse(&line) {
            Ok(command) => command,
            Err(e) => {
                send_response(&mut stream, &SmtpResponse::syntax_error(&e)).await?;
                continue;
            }
        };
        let middleware_cmd = match &lmtp_cmd {
            LmtpCommand::Lhlo(domain) => SmtpCommand::Ehlo(domain.clone()),
            LmtpCommand::Smtp(command) => command.clone(),
        };

        // ── Middleware pre-filter (if configured) ──────────────────
        if !command_middleware.is_empty() {
            let session = SessionExtensions::new();

            let mut blocked = false;
            for mw in &command_middleware {
                let mw = Arc::clone(mw);
                let cmd_for_mw = middleware_cmd.clone();
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

        let step = core.handle_command(lmtp_cmd, &policy);
        for response in &step.responses {
            send_response(&mut stream, response).await?;
        }

        match step.action {
            LmtpSessionAction::Quit => break,
            LmtpSessionAction::Deliver => {
                in_data_phase = false;
            }
            LmtpSessionAction::Continue => {
                in_data_phase = core.state == SmtpState::Data;
            }
        }
    }

    Ok(())
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
