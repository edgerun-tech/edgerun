//! Built-in command middleware implementations for mail protocol servers.
//!
//! # Command Middleware (edgerun-email)
//! - `SmtpRequireAuth` — reject MAIL FROM without AUTH
//! - `SmtpCommandLogger` — log every SMTP command and response
//! - `SmtpRateLimit` — per-IP command rate limiting
//! - `ImapCommandLogger` — log every IMAP command and response

use crate::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use crate::rt::Mutex;

use crate::command_middleware::{CommandMiddleware, ControlFlow, NextCommand, SessionExtensions};

/// Peer address stored in SessionExtensions.
#[derive(Clone)]
pub struct PeerAddr(pub SocketAddr);

struct RateLimitEntry {
    count: usize,
    window_start: Instant,
}

// ---- SMTP ----

/// Require AUTH before MAIL FROM.
///
/// If a client sends MAIL FROM without authenticating first,
/// returns `530 Authentication required`.
pub struct SmtpRequireAuth;

impl CommandMiddleware<crate::smtp::SmtpCommand, crate::smtp::SmtpResponse> for SmtpRequireAuth {
    fn handle(
        &self,
        cmd: crate::smtp::SmtpCommand,
        session: SessionExtensions,
        next: NextCommand<crate::smtp::SmtpCommand, crate::smtp::SmtpResponse>,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<crate::smtp::SmtpResponse>>> + Send + '_>>
    {
        use crate::smtp::SmtpCommand;
        use crate::smtp::SmtpResponse;
        use crate::smtp::server::session::SmtpAuth;

        Box::pin(async move {
            let needs_auth = matches!(&cmd, SmtpCommand::MailFrom { .. });

            if needs_auth {
                if let Some(auth) = session.get::<SmtpAuth>().await {
                    if auth.authenticated {
                        return next.run(cmd, session).await;
                    }
                }
                return Ok(ControlFlow::Respond(SmtpResponse::auth_required()));
            }

            next.run(cmd, session).await
        })
    }
}

/// Log every SMTP command and its response.
pub struct SmtpCommandLogger {
    prefix: String,
}

impl SmtpCommandLogger {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl CommandMiddleware<crate::smtp::SmtpCommand, crate::smtp::SmtpResponse> for SmtpCommandLogger {
    fn handle(
        &self,
        cmd: crate::smtp::SmtpCommand,
        session: SessionExtensions,
        next: NextCommand<crate::smtp::SmtpCommand, crate::smtp::SmtpResponse>,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<crate::smtp::SmtpResponse>>> + Send + '_>>
    {
        let prefix = self.prefix.clone();
        Box::pin(async move {
            edgerun_log::debug!("{}: CMD {:?}", prefix, cmd);
            let result = next.run(cmd, session).await;
            if let Ok(ControlFlow::Respond(ref resp)) = result {
                edgerun_log::debug!("{}: RESP {:?}", prefix, resp);
            }
            result
        })
    }
}

/// Per-IP command rate limiter for SMTP.
///
/// Returns `451 Rate limit exceeded` when a client exceeds
/// `max_commands` within the sliding `window_secs` window.
pub struct SmtpRateLimit {
    state: Arc<Mutex<HashMap<IpAddr, RateLimitEntry>>>,
    max_commands: usize,
    window_secs: u64,
}

impl SmtpRateLimit {
    pub fn new(max_commands: usize, window_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_commands,
            window_secs,
        }
    }
}

impl CommandMiddleware<crate::smtp::SmtpCommand, crate::smtp::SmtpResponse> for SmtpRateLimit {
    fn handle(
        &self,
        cmd: crate::smtp::SmtpCommand,
        session: SessionExtensions,
        next: NextCommand<crate::smtp::SmtpCommand, crate::smtp::SmtpResponse>,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<crate::smtp::SmtpResponse>>> + Send + '_>>
    {
        use crate::smtp::SmtpResponse;

        let state = Arc::clone(&self.state);
        let max = self.max_commands;
        let window = self.window_secs;

        Box::pin(async move {
            let peer = match session.get::<PeerAddr>().await {
                Some(p) => p.0.ip(),
                None => return next.run(cmd, session).await,
            };

            let mut map = state.lock().await;
            let now = Instant::now();

            let entry = map.entry(peer).or_insert(RateLimitEntry {
                count: 0,
                window_start: now,
            });

            if now.duration_since(entry.window_start).as_secs() >= window {
                entry.count = 0;
                entry.window_start = now;
            }

            entry.count += 1;
            if entry.count > max {
                return Ok(ControlFlow::Respond(SmtpResponse::transient_failure(
                    "Rate limit exceeded",
                )));
            }

            drop(map);
            next.run(cmd, session).await
        })
    }
}

// ---- IMAP ----

/// Log every IMAP command and its response.
pub struct ImapCommandLogger {
    prefix: String,
}

impl ImapCommandLogger {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl CommandMiddleware<crate::imap::ImapCommand, crate::imap::ImapResponse> for ImapCommandLogger {
    fn handle(
        &self,
        cmd: crate::imap::ImapCommand,
        session: SessionExtensions,
        next: NextCommand<crate::imap::ImapCommand, crate::imap::ImapResponse>,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<crate::imap::ImapResponse>>> + Send + '_>>
    {
        let prefix = self.prefix.clone();
        Box::pin(async move {
            edgerun_log::debug!("{}: CMD {:?}", prefix, cmd);
            let result = next.run(cmd, session).await;
            if let Ok(ControlFlow::Respond(ref resp)) = result {
                edgerun_log::debug!("{}: RESP {:?}", prefix, resp);
            }
            result
        })
    }
}
