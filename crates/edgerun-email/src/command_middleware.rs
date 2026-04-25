//! Command-level middleware for mail protocol servers (SMTP, IMAP, LMTP).
//!
//! Adapted from the HTTP middleware pattern but designed for **stateful,
//! multi-command sessions** where `SessionExtensions` persists across
//! all commands on a single connection.
//!
//! # Architecture
//! ```text
//! Command (parsed from wire)
//!     ↓
//! ┌─ Command Middleware Chain ───────────────────────────────────┐
//! │  RateLimit → AuthCheck → TlsEnforce → QuotaCheck → Handler   │
//! └──────────────────────────────────────────────────────────────┘
//!     ↓
//! Response → Wire
//! ```
//!
//! # Example — SMTP rate limiter
//! ```ignore
//! use edgerun_email::command_middleware::{CommandMiddleware, NextCommand, SessionExtensions};
//! use edgerun_email::smtp::types::SmtpCommand;
//! use edgerun_email::smtp::server::SmtpResponse;
//!
//! struct SmtpRateLimit { limiter: Arc<RateLimiter> }
//!
//! impl CommandMiddleware<SmtpCommand, SmtpResponse> for SmtpRateLimit {
//!     async fn handle(
//!         &self,
//!         cmd: SmtpCommand,
//!         session: &mut SessionExtensions,
//!         next: NextCommand<SmtpCommand, SmtpResponse>,
//!     ) -> io::Result<ControlFlow<SmtpResponse>> {
//!         let peer = session.get::<PeerAddr>().unwrap();
//!         if !self.limiter.is_allowed(&peer.0) {
//!             return Ok(ControlFlow::Respond(SmtpResponse::rate_limited()));
//!         }
//!         next.run(cmd, session).await
//!     }
//! }
//! ```
//!
//! # Middleware Ordering
//! First `.with()` = outermost (sees command first, response last).
//! Last `.with()` = innermost (closest to protocol handler).
//!
//! ```ignore
//! let chain = CommandChain::new(smtp_handler)
//!     .with(rate_limit(limiter))    // outermost
//!     .with(require_tls())
//!     .with(require_auth())
//!     .with(log_commands())         // innermost
//!     .build();
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::sync::Arc;

use edgerun_rt::Mutex;

// ===========================================================================
// SessionExtensions — type-erased, session-scoped data store
// ===========================================================================

/// Type-erased data store that persists across all commands on a single
/// connection. Unlike HTTP's per-request Extensions, this lives for the
/// entire session lifetime.
///
/// Keyed by `TypeId` — only one value of each type can exist at a time.
///
/// # Example
/// ```ignore
/// // Auth middleware stores authenticated identity
/// session.insert(AuthenticatedUser { username: "ken".into() }).await;
///
/// // Later middleware or handler retrieves it
/// let user: Option<AuthenticatedUser> = session.get().await;
/// ```
#[derive(Clone)]
pub struct SessionExtensions {
    inner: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send>>>>,
}

impl SessionExtensions {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Insert a value, replacing any existing value of the same type.
    pub async fn insert<T: Send + 'static>(&self, value: T) {
        self.inner
            .lock()
            .await
            .insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a cloned value if present.
    pub async fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.inner
            .lock()
            .await
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>().cloned())
    }

    /// Remove and return a value if present.
    pub async fn remove<T: 'static>(&self) -> Option<T> {
        self.inner
            .lock()
            .await
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast::<T>().ok())
            .map(|b| *b)
    }

    /// Check if a value of the given type is present.
    pub async fn contains<T: 'static>(&self) -> bool {
        self.inner.lock().await.contains_key(&TypeId::of::<T>())
    }

    /// Clear all stored values.
    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }
}

impl Default for SessionExtensions {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// CommandMiddleware Trait
// ===========================================================================

/// Command-level middleware for a specific protocol.
///
/// `Cmd` is the parsed command type (e.g., `SmtpCommand`).
/// `Resp` is the response type (e.g., `SmtpResponse`).
pub trait CommandMiddleware<Cmd, Resp>: Send + Sync + 'static
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
{
    /// Intercept a command before it reaches the handler.
    ///
    /// Return `ControlFlow::Respond(resp)` to short-circuit and send
    /// a response directly.
    /// Return `ControlFlow::Continue` to pass the command downstream.
    fn handle(
        &self,
        cmd: Cmd,
        session: SessionExtensions,
        next: NextCommand<Cmd, Resp>,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send + '_>>;
}

// ===========================================================================
// ControlFlow
// ===========================================================================

/// Result of command middleware execution.
pub enum ControlFlow<Resp> {
    /// Short-circuit: send this response and do not call downstream.
    Respond(Resp),
    /// Continue: pass the command to the next middleware/handler.
    Continue,
}

// ===========================================================================
// NextCommand Handle
// ===========================================================================

/// Handle to the next middleware or handler in the chain.
///
/// Calling `.run()` passes the command downstream exactly once.
/// The handle is consumed by `run()`, preventing double-handling.
pub struct NextCommand<Cmd, Resp> {
    inner: Box<
        dyn FnOnce(
                Cmd,
                SessionExtensions,
            )
                -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send>>
            + Send,
    >,
}

use std::pin::Pin;

impl<Cmd, Resp> NextCommand<Cmd, Resp> {
    pub fn new<F>(inner: F) -> Self
    where
        F: FnOnce(
                Cmd,
                SessionExtensions,
            )
                -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send>>
            + Send
            + 'static,
    {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Pass the command to the next middleware or handler.
    pub fn run(
        self,
        cmd: Cmd,
        session: SessionExtensions,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send>> {
        (self.inner)(cmd, session)
    }
}

// ===========================================================================
// CommandHandler Trait
// ===========================================================================

/// A handler for protocol commands.
///
/// Protocol implementations (SMTP, IMAP, LMTP) implement this trait.
/// Middleware wraps handlers via `CommandChain`.
pub trait CommandHandler<Cmd, Resp>: Send + Sync + 'static
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
{
    fn handle(
        &self,
        cmd: Cmd,
        session: SessionExtensions,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send + '_>>;
}

// ===========================================================================
// MiddlewareLayer
// ===========================================================================

struct MiddlewareLayer<Cmd, Resp> {
    middleware: Arc<dyn CommandMiddleware<Cmd, Resp>>,
    inner: Arc<dyn CommandHandler<Cmd, Resp>>,
}

impl<Cmd, Resp> CommandHandler<Cmd, Resp> for MiddlewareLayer<Cmd, Resp>
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
{
    fn handle(
        &self,
        cmd: Cmd,
        session: SessionExtensions,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send + '_>> {
        let mw = Arc::clone(&self.middleware);
        let inner = Arc::clone(&self.inner);
        let session_clone = session.clone();

        let next = NextCommand::new(move |cmd, sess| {
            let inner = Arc::clone(&inner);
            Box::pin(async move { inner.handle(cmd, sess).await })
        });

        // Clone the future to extend its lifetime past this function
        Box::pin(async move { mw.handle(cmd, session_clone, next).await })
    }
}

// ===========================================================================
// CommandChain Builder
// ===========================================================================

/// Builder for composing command middleware into a single handler.
///
/// # Example
/// ```ignore
/// let handler = CommandChain::new(smtp_handler)
///     .with(rate_limit)
///     .with(require_auth)
///     .build();
/// ```
pub struct CommandChain<Cmd, Resp>
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
{
    middlewares: Vec<Arc<dyn CommandMiddleware<Cmd, Resp>>>,
    handler: Arc<dyn CommandHandler<Cmd, Resp>>,
}

impl<Cmd, Resp> CommandChain<Cmd, Resp>
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
{
    /// Create a new chain with the given protocol handler.
    pub fn new<H: CommandHandler<Cmd, Resp>>(handler: H) -> Self {
        Self {
            middlewares: Vec::new(),
            handler: Arc::new(handler),
        }
    }

    /// Add middleware to the chain.
    ///
    /// First `.with()` = outermost (sees command first, response last).
    /// Last `.with()` = innermost (closest to handler).
    pub fn with<M: CommandMiddleware<Cmd, Resp>>(mut self, mw: M) -> Self {
        self.middlewares.push(Arc::new(mw));
        self
    }

    /// Build the chain into a single `impl CommandHandler`.
    pub fn build(self) -> impl CommandHandler<Cmd, Resp> {
        let mut h = self.handler;
        // Reverse so first `.with()` is outermost
        for mw in self.middlewares.into_iter().rev() {
            h = Arc::new(MiddlewareLayer {
                middleware: mw,
                inner: h,
            });
        }
        h
    }
}

// ===========================================================================
// Function-Based Middleware
// =========================================================================//

/// Function-based command middleware.
pub struct FnCommandMiddleware<Cmd, Resp, F> {
    f: F,
    _cmd: std::marker::PhantomData<Cmd>,
    _resp: std::marker::PhantomData<Resp>,
}

/// Create command middleware from a closure.
pub fn command_fn<Cmd, Resp, F>(f: F) -> FnCommandMiddleware<Cmd, Resp, F>
where
    F: Fn(
            Cmd,
            SessionExtensions,
            NextCommand<Cmd, Resp>,
        ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send>>
        + Send
        + Sync
        + 'static,
{
    FnCommandMiddleware {
        f,
        _cmd: std::marker::PhantomData,
        _resp: std::marker::PhantomData,
    }
}

impl<Cmd, Resp, F> CommandMiddleware<Cmd, Resp> for FnCommandMiddleware<Cmd, Resp, F>
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
    F: Fn(
            Cmd,
            SessionExtensions,
            NextCommand<Cmd, Resp>,
        ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send>>
        + Send
        + Sync
        + 'static,
{
    fn handle(
        &self,
        cmd: Cmd,
        session: SessionExtensions,
        next: NextCommand<Cmd, Resp>,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send + '_>> {
        Box::pin((self.f)(cmd, session, next))
    }
}

// =========================================================================//
// Blanket impl for Arc<T: CommandHandler>
// ===========================================================================

impl<Cmd, Resp, T> CommandHandler<Cmd, Resp> for Arc<T>
where
    Cmd: Send + Sync + 'static,
    Resp: Send + Sync + 'static,
    T: CommandHandler<Cmd, Resp> + ?Sized,
{
    fn handle(
        &self,
        cmd: Cmd,
        session: SessionExtensions,
    ) -> Pin<Box<dyn Future<Output = io::Result<ControlFlow<Resp>>> + Send + '_>> {
        T::handle(self, cmd, session)
    }
}
