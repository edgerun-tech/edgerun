//! Connection-level middleware for `BoundServer`.
//!
//! Middleware runs on every TCP connection **before** protocol parsing,
//! regardless of which protocol (HTTP, SMTP, IMAP, LMTP, DNS, etc.)
//! the connection is for.
//!
//! # Use Cases
//! - IP allowlist/blocklist
//! - Per-IP connection rate limiting
//! - PROXY protocol parsing (extract real client IP from load balancer)
//! - Connection logging/telemetry
//! - GeoIP enrichment
//! - TLS interception (wrap all connections with TLS)
//!
//! # Example
//! ```ignore
//! use edgerun_server::middleware::{ConnectionMiddleware, NextConnection};
//!
//! struct IpFilter { allowed: Vec<IpNet> }
//!
//! impl ConnectionMiddleware for IpFilter {
//!     async fn on_connect(
//!         &self,
//!         peer: SocketAddr,
//!         stream: AsyncTcpStream,
//!         next: NextConnection,
//!     ) -> io::Result<()> {
//!         if !self.allowed.iter().any(|n| n.contains(&peer.ip())) {
//!             return Err(io::Error::new(io::ErrorKind::PermissionDenied, "IP not allowed"));
//!         }
//!         next.run(peer, stream).await
//!     }
//! }
//!
//! let server = Server::new()
//!     .with_connection_middleware(IpFilter { allowed: vec!["10.0.0.0/8".parse().unwrap()] })
//!     .with_http(handler, "0.0.0.0:8080")
//!     .build()
//!     .await?;
//! ```

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use edgerun_rt::AsyncTcpStream;

// ===========================================================================
// ConnectionMiddleware Trait
// ===========================================================================

/// Connection-level middleware.
///
/// Implementations run on every new TCP connection before protocol parsing.
/// They can:
/// - **Accept** the connection and pass it downstream via `next.run()`
/// - **Reject** the connection by returning `Err` (caller closes immediately)
/// - **Wrap** the stream (e.g., PROXY protocol, TLS interception)
pub trait ConnectionMiddleware: Send + Sync + 'static {
    /// Called on every new TCP connection.
    ///
    /// - `peer`: the remote socket address
    /// - `stream`: the raw TCP stream
    /// - `next`: handle to the next middleware / protocol handler
    ///
    /// Returns `Ok(())` to continue processing, or `Err` to reject
    /// the connection (the server will close it immediately).
    fn on_connect(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
        next: NextConnection,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>>;
}

// ===========================================================================
// NextConnection Handle
// ===========================================================================

/// Handle to the next middleware or protocol handler in the chain.
///
/// Calling `.run()` passes the connection downstream exactly once.
/// The handle is consumed by `run()`, preventing double-handling.
pub struct NextConnection {
    inner: Box<dyn FnOnce(SocketAddr, AsyncTcpStream) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> + Send>,
}

impl NextConnection {
    pub fn new<F>(inner: F) -> Self
    where
        F: FnOnce(SocketAddr, AsyncTcpStream) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> + Send + 'static,
    {
        Self { inner: Box::new(inner) }
    }

    /// Pass the connection to the next middleware or protocol handler.
    pub fn run(self, peer: SocketAddr, stream: AsyncTcpStream) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> {
        (self.inner)(peer, stream)
    }
}

// ===========================================================================
// MiddlewareLayer
// ===========================================================================

struct MiddlewareLayer {
    middleware: Arc<dyn ConnectionMiddleware>,
    inner: Arc<dyn ConnectionHandler>,
}

impl ConnectionHandler for MiddlewareLayer {
    fn handle(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> {
        let mw = Arc::clone(&self.middleware);
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let next = NextConnection::new(move |peer, stream| {
                inner.handle(peer, stream)
            });
            mw.on_connect(peer, stream, next).await
        })
    }
}

// ===========================================================================
// ConnectionHandler Trait
// ===========================================================================

/// A handler for raw TCP connections.
///
/// Protocol servers (HTTP, SMTP, IMAP, etc.) implement this trait.
/// Middleware wraps handlers via `MiddlewareLayer`.
pub trait ConnectionHandler: Send + Sync + 'static {
    fn handle(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>>;
}

// ===========================================================================
// ConnectionChain Builder
// ===========================================================================

/// Builder for composing connection middleware into a single handler.
///
/// # Example
/// ```ignore
/// let handler = ConnectionChain::new(http_handler)
///     .with(IpFilter { allowed: vec!["10.0.0.0/8".parse().unwrap()] })
///     .with(ConnectionLogger)
///     .build();
/// ```
pub struct ConnectionChain {
    middlewares: Vec<Arc<dyn ConnectionMiddleware>>,
    handler: Arc<dyn ConnectionHandler>,
}

impl ConnectionChain {
    /// Create a new chain with the given protocol handler.
    pub fn new<H: ConnectionHandler>(handler: H) -> Self {
        Self {
            middlewares: Vec::new(),
            handler: Arc::new(handler),
        }
    }

    /// Add middleware to the chain.
    ///
    /// First `.with()` = outermost (runs first on connect, last on disconnect).
    /// Last `.with()` = innermost (closest to protocol handler).
    pub fn with<M: ConnectionMiddleware>(mut self, mw: M) -> Self {
        self.middlewares.push(Arc::new(mw));
        self
    }

    /// Build the chain into a single `impl ConnectionHandler`.
    pub fn build(self) -> impl ConnectionHandler {
        let mut h = self.handler;
        // Reverse so first `.with()` is outermost
        for mw in self.middlewares.into_iter().rev() {
            h = Arc::new(MiddlewareLayer { middleware: mw, inner: h });
        }
        h
    }
}

// ===========================================================================
// Function-Based Middleware
// ===========================================================================

/// Function-based connection middleware.
pub struct FnConnectionMiddleware<F> {
    f: F,
}

/// Create connection middleware from a closure.
///
/// # Example
/// ```ignore
/// let mw = connection_fn(|peer, _stream, next| {
///     println!("Connection from {}", peer);
///     next.run(peer, stream)
/// });
/// ```
pub fn connection_fn<F>(f: F) -> FnConnectionMiddleware<F>
where
    F: Fn(SocketAddr, AsyncTcpStream, NextConnection) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>>
        + Send + Sync + 'static,
{
    FnConnectionMiddleware { f }
}

impl<F> ConnectionMiddleware for FnConnectionMiddleware<F>
where
    F: Fn(SocketAddr, AsyncTcpStream, NextConnection) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>>
        + Send + Sync + 'static,
{
    fn on_connect(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
        next: NextConnection,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        Box::pin((self.f)(peer, stream, next))
    }
}

// ===========================================================================
// Pass-Through Handler (innermost layer)
// ===========================================================================

/// A handler that does nothing — used as the innermost layer when
/// no protocol server is configured, or as a placeholder.
pub struct PassThroughHandler;

impl ConnectionHandler for PassThroughHandler {
    fn handle(
        &self,
        _peer: SocketAddr,
        _stream: AsyncTcpStream,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}

impl<T: ConnectionHandler + ?Sized> ConnectionHandler for Arc<T> {
    fn handle(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> {
        T::handle(self, peer, stream)
    }
}

// ===========================================================================
// Built-in Connection Middleware Implementations
// ===========================================================================

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

/// IP filter — allow or deny connections by IP address prefix.
///
/// Uses simple string prefix matching (no subnet math).
/// For CIDR matching, add the `ipnet` crate.
pub struct IpFilter {
    allowed: Vec<String>,
    denied: Vec<String>,
}

impl IpFilter {
    pub fn new() -> Self {
        Self {
            allowed: Vec::new(),
            denied: Vec::new(),
        }
    }

    pub fn allow_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.allowed.push(prefix.into());
        self
    }

    pub fn deny_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.denied.push(prefix.into());
        self
    }

    fn is_allowed(&self, ip: IpAddr) -> bool {
        let ip_str = ip.to_string();
        if self.denied.iter().any(|d| ip_str.starts_with(d)) {
            return false;
        }
        if !self.allowed.is_empty() {
            return self.allowed.iter().any(|a| ip_str.starts_with(a));
        }
        true
    }
}

impl ConnectionMiddleware for IpFilter {
    fn on_connect(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
        next: NextConnection,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        if self.is_allowed(peer.ip()) {
            next.run(peer, stream)
        } else {
            Box::pin(async move {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("IP {} not allowed", peer.ip()),
                ))
            })
        }
    }
}

/// Connection logger — logs every connect/disconnect.
pub struct ConnectionLogger {
    prefix: String,
}

impl ConnectionLogger {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self { prefix: prefix.into() }
    }
}

impl ConnectionMiddleware for ConnectionLogger {
    fn on_connect(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
        next: NextConnection,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        edgerun_log::info!("{}: connection from {}", self.prefix, peer);
        next.run(peer, stream)
    }
}

/// Per-IP connection rate limiter.
pub struct ConnectionRateLimit {
    state: Arc<edgerun_rt::Mutex<HashMap<IpAddr, RateLimitEntry>>>,
    max_connections: usize,
    window_secs: u64,
}

struct RateLimitEntry {
    count: usize,
    window_start: Instant,
}

impl ConnectionRateLimit {
    pub fn new(max_connections: usize, window_secs: u64) -> Self {
        Self {
            state: Arc::new(edgerun_rt::Mutex::new(HashMap::new())),
            max_connections,
            window_secs,
        }
    }
}

impl ConnectionMiddleware for ConnectionRateLimit {
    fn on_connect(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
        next: NextConnection,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let state = Arc::clone(&self.state);
        let max = self.max_connections;
        let window = self.window_secs;

        Box::pin(async move {
            let mut map = state.lock().await;
            let now = Instant::now();

            let entry = map.entry(peer.ip()).or_insert(RateLimitEntry {
                count: 0,
                window_start: now,
            });

            if now.duration_since(entry.window_start).as_secs() >= window {
                entry.count = 0;
                entry.window_start = now;
            }

            entry.count += 1;
            if entry.count > max {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Connection rate limit exceeded ({} per {}s)", max, window),
                ));
            }

            drop(map);
            next.run(peer, stream).await
        })
    }
}

// ===========================================================================
// Adapter: wrap Arc<dyn ConnectionMiddleware> as ConnectionMiddleware
// ===========================================================================

/// Adapter that wraps an `Arc<dyn ConnectionMiddleware>` so it can be
/// passed to `ConnectionChain::with()`.
pub struct MiddlewareAdapter(pub Arc<dyn ConnectionMiddleware>);

impl ConnectionMiddleware for MiddlewareAdapter {
    fn on_connect(
        &self,
        peer: SocketAddr,
        stream: AsyncTcpStream,
        next: NextConnection,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        self.0.on_connect(peer, stream, next)
    }
}

// ===========================================================================
// ConnectionInterceptor Adapter — wraps edgerun-server middleware for
// edgerun-email protocol servers
// ===========================================================================

/// Adapter that implements `edgerun_email::server::ConnectionInterceptor`
/// by delegating to a compiled `ConnectionHandler`.
///
/// This allows edgerun-server's connection middleware chain to be passed
/// into SMTP/IMAP/LMTP servers without circular dependencies.
pub struct ConnectionInterceptorAdapter {
    handler: Arc<dyn ConnectionHandler>,
}

impl ConnectionInterceptorAdapter {
    pub fn new(handler: Arc<dyn ConnectionHandler>) -> Self {
        Self { handler }
    }
}

impl edgerun_email::server::ConnectionInterceptor for ConnectionInterceptorAdapter {
    fn intercept(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let handler = Arc::clone(&self.handler);
        let stream = Arc::clone(&stream);
        Box::pin(async move {
            let stream = match Arc::try_unwrap(stream) {
                Ok(s) => s,
                Err(arc) => Arc::clone(&arc),
            };
            handler.handle(peer, stream).await
        })
    }
}
