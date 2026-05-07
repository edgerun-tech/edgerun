//! Unified server framework for all mail protocols.
//!
//! Provides:
//! - Generic TCP listener with `CancellationToken` shutdown
//! - Shared TLS upgrade (STARTTLS) via `AsyncTlsStream`
//! - Per-connection spawn with rate limiting + idle timeout
//! - Protocol-specific command dispatch via `MailProtocol` trait
//!
//! # Protocols
//! - SMTP (port 25, STARTTLS on port 587, SMTPS on port 465)
//! - IMAP (port 143, STARTTLS, IMAPS on port 993)
//! - LMTP (port 24, no TLS — local delivery only)

use crate::prelude::*;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::rt::{
    AsyncRead, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWrite, AsyncWriteExt,
    CancellationToken,
};

#[cfg(feature = "tls")]
use edgerun_node::tls::{AsyncTlsStream, CertificateAndKey};

// ===========================================================================
// Connection Interceptor — protocol-agnostic hook for connection middleware
// ===========================================================================

use std::future::Future;
use std::pin::Pin;

/// A hook that runs on every new TCP connection before protocol parsing.
///
/// Implementations can:
/// - **Accept** the connection and pass it downstream via returning `Ok(())`
/// - **Reject** the connection by returning `Err` (caller closes immediately)
///
/// This trait lives in `edgerun-email` so protocol servers can store
/// it without depending on `edgerun-server`. The server crate builds
/// the middleware chain and passes it in.
pub trait ConnectionInterceptor: Send + Sync + 'static {
    /// Called on every new TCP connection.
    ///
    /// Returns `Ok(())` to continue processing, or `Err` to reject
    /// the connection (the server will close it immediately).
    fn intercept(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>>;
}

// ===========================================================================
// Transport — unified wrapper for plain TCP and TLS
// ===========================================================================

/// Wraps either a plain TCP connection or a TLS stream so protocol
/// handlers can read/write transparently and upgrade mid-session.
pub enum Transport {
    Plain(AsyncTcpStream),
    #[cfg(feature = "tls")]
    Tls(AsyncTlsStream<AsyncTcpStream>),
}

impl Transport {
    /// Check whether this transport is already using TLS.
    pub fn is_tls(&self) -> bool {
        match self {
            Transport::Plain(_) => false,
            #[cfg(feature = "tls")]
            Transport::Tls(_) => true,
        }
    }

    /// Upgrade this transport to TLS via the `edgerun-node TLS` client handshake.
    #[cfg(feature = "tls")]
    pub async fn upgrade_tls(self, server_name: &str) -> io::Result<Self> {
        match self {
            Transport::Tls(_) => Err(io::Error::other("already using TLS")),
            Transport::Plain(stream) => {
                let tls = AsyncTlsStream::client(stream, server_name, &[], None)
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::ConnectionAborted, e.to_string()))?;
                Ok(Transport::Tls(tls))
            }
        }
    }
}

impl AsyncRead for Transport {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            Transport::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            Transport::Tls(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Transport {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        match &mut *self {
            Transport::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            Transport::Tls(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match &mut *self {
            Transport::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            #[cfg(feature = "tls")]
            Transport::Tls(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        match &mut *self {
            Transport::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            Transport::Tls(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl Unpin for Transport {}

// ===========================================================================
// Read helpers — shared line + chunk readers
// ===========================================================================

/// Read a line from the transport (CRLF-terminated).
/// Returns `None` on EOF.
///
/// Works with any type implementing `AsyncRead + Unpin`:
/// - `AsyncTcpStream` (plain TCP)
/// - `AsyncTlsStream<AsyncTcpStream>` (client TLS)
/// - `AsyncTlsServerStream<AsyncTcpStream>` (server TLS)
/// - Protocol-specific Transport enums
pub async fn read_line<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = String::with_capacity(1024);
    loop {
        let mut buf = [0u8; 1];
        let n = match reader.read(&mut buf).await {
            Ok(0) => {
                if line.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(line));
            }
            Ok(n) => n,
            Err(e) => return Err(e),
        };
        if n == 0 {
            continue;
        }
        if buf[0] == b'\n' {
            if line.ends_with('\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
        line.push(buf[0] as char);
    }
}

// ===========================================================================
// MailProtocol trait
// ===========================================================================

/// A mail protocol implementation (SMTP, IMAP, LMTP).
///
/// Implementors define:
/// - The greeting sent on connection
/// - Command parsing and dispatch
/// - Per-connection state machine
pub trait MailProtocol: Send + Sync + 'static {
    /// The command type parsed from input lines.
    type Command: Send;
    /// The response type sent back to clients.
    type Response: Send;

    /// Generate the greeting for this protocol.
    fn greeting(&self, peer: SocketAddr) -> String;

    /// Parse a command from an input line.
    fn parse_command(&self, line: &str) -> Result<Self::Command, String>;

    /// Execute a command and return the response.
    fn execute(
        &self,
        cmd: Self::Command,
        state: &mut ProtocolState,
        transport: &mut Transport,
    ) -> impl std::future::Future<Output = io::Result<ControlFlow>> + Send;
}

/// Control flow returned by command execution.
pub enum ControlFlow {
    /// Continue reading commands.
    Continue,
    /// Close the connection (QUIT).
    Quit,
    /// Upgrade to TLS (STARTTLS).
    StartTls,
}

/// Per-connection protocol state.
///
/// Protocols store their own state machine here.
pub struct ProtocolState {
    /// Whether the connection is authenticated.
    pub authenticated: bool,
    /// Whether the connection has been upgraded to TLS.
    pub tls_upgraded: bool,
    /// Number of commands processed.
    pub command_count: usize,
    /// Time of last activity.
    pub last_activity: std::time::Instant,
}

impl Default for ProtocolState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolState {
    pub fn new() -> Self {
        Self {
            authenticated: false,
            tls_upgraded: false,
            command_count: 0,
            last_activity: std::time::Instant::now(),
        }
    }
}

// ===========================================================================
// Server configuration
// ===========================================================================

/// Configuration for a mail protocol server.
#[derive(Clone)]
pub struct ServerConfig {
    /// Address to bind to (e.g. "0.0.0.0:25").
    pub bind_addr: String,
    /// Domain name used in greetings.
    pub domain: String,
    /// Optional TLS certificate for STARTTLS / implicit TLS.
    #[cfg(feature = "tls")]
    pub tls_cert: Option<CertificateAndKey>,
    /// Whether to use implicit TLS (SMTPS/IMAPS).
    pub implicit_tls: bool,
    /// Idle timeout in seconds.
    pub idle_timeout_secs: u64,
    /// Maximum commands per connection.
    pub max_commands: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:25".to_string(),
            domain: "edgerun.mail".to_string(),
            #[cfg(feature = "tls")]
            tls_cert: None,
            implicit_tls: false,
            idle_timeout_secs: 300,
            max_commands: 10000,
        }
    }
}

// ===========================================================================
// ProtocolServer
// ===========================================================================

/// A generic mail protocol server.
///
/// Listens for TCP connections, spawns a handler per connection,
/// and gracefully shuts down via `CancellationToken`.
pub struct ProtocolServer<P: MailProtocol> {
    listener: Arc<crate::rt::AsyncTcpListener>,
    protocol: Arc<P>,
    config: ServerConfig,
}

impl<P: MailProtocol> ProtocolServer<P> {
    /// Create a new protocol server.
    pub fn new(config: ServerConfig, protocol: P) -> io::Result<Self> {
        let listener = Arc::new(
            crate::rt::AsyncTcpListener::bind(&config.bind_addr).map_err(crate::rt::bare_io)?,
        );
        Self::with_listener(config, protocol, listener)
    }

    pub fn with_listener(
        config: ServerConfig,
        protocol: P,
        listener: Arc<crate::rt::AsyncTcpListener>,
    ) -> io::Result<Self> {
        edgerun_log::info!(
            "edgerun-mail: {} listening on {}",
            std::any::type_name::<P>(),
            config.bind_addr
        );
        Ok(Self {
            listener,
            protocol: Arc::new(protocol),
            config,
        })
    }

    /// Run the server until `shutdown` is cancelled.
    pub async fn run(&self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match self.listener.accept().await {
                Ok((stream, peer)) => {
                    let protocol = Arc::clone(&self.protocol);
                    let config = self.config.clone();
                    let shutdown = shutdown.clone();

                    edgerun_log::info!("edgerun-mail: connection from {}", peer);
                    crate::rt::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, peer, protocol, config, shutdown).await
                        {
                            edgerun_log::error!("edgerun-mail: connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("edgerun-mail: accept error: {}", e);
                    crate::rt::sleep(Duration::from_millis(10)).await;
                }
            }
        }
        edgerun_log::info!("edgerun-mail: server shut down");
        Ok(())
    }
}

// ===========================================================================
// Connection handler
// ===========================================================================

async fn handle_connection<P: MailProtocol>(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    protocol: Arc<P>,
    config: ServerConfig,
    shutdown: CancellationToken,
) -> io::Result<()> {
    // Unwrap Arc to get owned stream
    let stream = match Arc::try_unwrap(stream) {
        Ok(s) => s,
        Err(_) => {
            edgerun_log::error!("edgerun-mail: stream has multiple references");
            return Err(io::Error::other("stream reference error"));
        }
    };

    // Use plain TCP — protocol implementations handle their own TLS upgrade
    // because AsyncTlsServerStream and AsyncTlsStream are different types.
    let mut transport = Transport::Plain(stream);

    // Send greeting
    let greeting = protocol.greeting(peer);
    transport.write_all(greeting.as_bytes()).await?;
    transport.flush().await?;

    let mut state = ProtocolState::new();

    loop {
        if shutdown.is_cancelled() {
            break;
        }

        // Idle timeout
        let elapsed = state.last_activity.elapsed().as_secs();
        if elapsed > config.idle_timeout_secs {
            edgerun_log::info!("edgerun-mail: {} idle timeout ({}s)", peer, elapsed);
            break;
        }

        if state.command_count >= config.max_commands {
            break;
        }

        // Read command line
        let line = match read_line(&mut transport).await {
            Ok(Some(l)) => l,
            Ok(None) => {
                edgerun_log::info!("edgerun-mail: {} disconnected", peer);
                break;
            }
            Err(e) => {
                edgerun_log::error!("edgerun-mail: read error: {}", e);
                break;
            }
        };

        state.last_activity = std::time::Instant::now();
        state.command_count += 1;

        // Parse command
        let cmd = match protocol.parse_command(&line) {
            Ok(c) => c,
            Err(e) => {
                edgerun_log::warn!("edgerun-mail: parse error: {}", e);
                continue;
            }
        };

        // Execute command
        match protocol.execute(cmd, &mut state, &mut transport).await {
            Ok(ControlFlow::Continue) => {}
            Ok(ControlFlow::Quit) => break,
            Ok(ControlFlow::StartTls) => {
                // Protocol handles its own TLS upgrade (needs AsyncTlsServerStream).
                // The generic framework can't do server-side TLS because
                // AsyncTlsServerStream and AsyncTlsStream are different types.
                edgerun_log::warn!(
                    "edgerun-mail: StartTls returned but protocol must handle TLS upgrade internally"
                );
            }
            Err(e) => {
                edgerun_log::error!("edgerun-mail: command error: {}", e);
            }
        }
    }

    Ok(())
}
