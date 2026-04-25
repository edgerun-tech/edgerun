//! Connection-level middleware for the HTTP server.
//!
//! This runs on every TCP connection before protocol parsing, enabling:
//! - Rate limiting by IP
//! - Connection-level logging
//! - TLS inspection
//! - Protocol detection and routing
//!
//! # Architecture
//!
//! ```text
//!   TCP Connection  ──► [ConnectionMiddleware 1] ──► ... ──► [Handler]
//! ```
//!
//! Each middleware receives the peer address and stream. It can:
//! - **Accept** the connection by calling `next.run()`
//! - **Reject** the connection by returning `Err` (server closes it immediately)
//! - **Inspect/modify** the stream before passing it downstream

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use edgerun_rt::AsyncTcpStream;

// ===========================================================================
// ConnectionHandler trait
// ===========================================================================

/// Handle a new TCP connection.
///
/// Receives the peer address and the stream. Returns `Ok(())` to accept
/// the connection, or `Err` to reject it (server will close the stream).
pub trait ConnectionHandler: Send + Sync + 'static {
    fn handle(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>>;
}

// ===========================================================================
// ConnectionMiddleware trait
// ===========================================================================

/// Connection-level middleware component.
///
/// Implement this trait to intercept connections before they reach the
/// protocol handler. Applied via [`ConnectionChain`].
pub trait ConnectionMiddleware: Send + Sync + 'static {
    fn process(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
        next: ConnectionNext,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>>;
}

/// Handle to the rest of the connection middleware chain.
pub struct ConnectionNext {
    inner: Arc<dyn ConnectionHandler>,
}

impl ConnectionNext {
    pub fn new(inner: Arc<dyn ConnectionHandler>) -> Self {
        Self { inner }
    }

    pub async fn run(self, peer: SocketAddr, stream: Arc<AsyncTcpStream>) -> io::Result<()> {
        self.inner.handle(peer, stream).await
    }
}

// ===========================================================================
// PassThroughHandler
// ===========================================================================

/// Default handler that accepts all connections.
///
/// Used when no middleware is configured or as the innermost handler
/// in a middleware chain.
pub struct PassThroughHandler;

impl ConnectionHandler for PassThroughHandler {
    fn handle(
        &self,
        _peer: SocketAddr,
        _stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

// ===========================================================================
// MiddlewareAdapter
// ===========================================================================

/// Adapt a [`ConnectionMiddleware`] to be used as the inner handler in a chain.
pub struct MiddlewareAdapter<M> {
    inner: M,
}

impl<M> MiddlewareAdapter<M> {
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl ConnectionHandler for MiddlewareAdapter<Arc<dyn ConnectionMiddleware>> {
    fn handle(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let mw = Arc::clone(&self.inner);
        let next = ConnectionNext::new(Arc::new(PassThroughHandler));
        Box::pin(async move { mw.process(peer, stream, next).await })
    }
}

impl ConnectionMiddleware for MiddlewareAdapter<Arc<dyn ConnectionMiddleware>> {
    fn process(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
        next: ConnectionNext,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let mw = Arc::clone(&self.inner);
        Box::pin(async move { mw.process(peer, stream, next).await })
    }
}

/// Enable `Arc<dyn ConnectionMiddleware>` to be used with `MiddlewareAdapter`.
impl ConnectionMiddleware for Arc<dyn ConnectionMiddleware> {
    fn process(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
        next: ConnectionNext,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let this = Arc::clone(self);
        Box::pin(async move { this.process(peer, stream, next).await })
    }
}

// ===========================================================================
// ConnectionChain builder
// ===========================================================================

/// Builder for stacking connection middleware on top of a handler.
///
/// Middlewares are applied **outside → in**: the first `.with()` is the
/// outermost layer, the last `.with()` is closest to the handler.
#[derive(Clone)]
pub struct ConnectionChain<H = PassThroughHandler> {
    inner: H,
    middlewares: Vec<Arc<dyn ConnectionMiddleware>>,
}

impl ConnectionChain<PassThroughHandler> {
    pub fn new(inner: PassThroughHandler) -> Self {
        Self {
            inner,
            middlewares: Vec::new(),
        }
    }
}

impl ConnectionChain {
    pub fn with<M: ConnectionMiddleware>(mut self, mw: M) -> Self {
        self.middlewares.push(Arc::new(mw));
        self
    }

    pub fn build(self) -> Arc<dyn ConnectionHandler> {
        let mut h: Arc<dyn ConnectionHandler> = Arc::new(self.inner);
        for mw in self.middlewares.into_iter().rev() {
            h = Arc::new(MiddlewareLayer {
                middleware: mw,
                inner: h,
            });
        }
        h
    }
}

/// A single layer: wraps one middleware around an inner handler.
struct MiddlewareLayer {
    middleware: Arc<dyn ConnectionMiddleware>,
    inner: Arc<dyn ConnectionHandler>,
}

impl ConnectionHandler for MiddlewareLayer {
    fn handle(
        &self,
        peer: SocketAddr,
        stream: Arc<AsyncTcpStream>,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        let mw = Arc::clone(&self.middleware);
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let next = ConnectionNext::new(inner);
            mw.process(peer, stream, next).await
        })
    }
}
