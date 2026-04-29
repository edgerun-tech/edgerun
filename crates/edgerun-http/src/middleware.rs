//! Middleware system for the unified HTTP server.
//!
//! # Architecture
//!
//! ```text
//!   Request  ──► [Middleware 1] ──► [Middleware 2] ──► [Handler]
//!   Response ◄── [Middleware 1] ◄── [Middleware 2] ◄── [Handler]
//! ```
//!
//! Each middleware receives the request and a [`Next`] handle. It can:
//! - **Short-circuit** — return a `Response` immediately (auth failure, CORS)
//! - **Mutate the request** — add extensions, modify headers, etc.
//! - **Mutate the response** — add headers, transform body, etc.
//!
//! # Example
//!
//! ```
//! use edgerun_http::middleware::{Chain, Middleware, Next};
//! use edgerun_http::{into_handler, Handler, Request, Response, StatusCode};
//! use std::future::Future;
//! use std::pin::Pin;
//!
//! // Struct-based middleware
//! struct Log;
//! impl Middleware for Log {
//!     fn call(&self, req: Request, next: Next)
//!         -> Pin<Box<dyn Future<Output = Response> + Send + '_>>
//!     {
//!         Box::pin(async move {
//!             eprintln!("[IN]  {} {}", req.method().as_str(), req.uri().request_target());
//!             let resp = next.run(req).await;
//!             eprintln!("[OUT] {}", resp.status().as_u16());
//!             resp
//!         })
//!     }
//! }
//!
//! // Function-based middleware
//! let cors = edgerun_http::middleware::middleware_fn(|req, next| {
//!     Box::pin(async move {
//!         if req.method().as_str() == "OPTIONS" {
//!             return Response::text(StatusCode::new(204).unwrap(), "")
//!                 .with_header("Access-Control-Allow-Origin", "*");
//!         }
//!         let mut resp = next.run(req).await;
//!         let _ = resp.headers_mut().insert("Access-Control-Allow-Origin", "*");
//!         resp
//!     })
//! });
//!
//! let my_handler = into_handler(|_| Response::text(StatusCode::new(200).unwrap(), "ok"));
//!
//! // Stack them (applied outside → in)
//! let handler = Chain::new(my_handler)
//!     .with(cors)
//!     .with(Log)
//!     .build();
//! ```

use crate::lock::SpinMutex;
use crate::{Handler, Request, Response};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::fmt;
use core::future::Future;
use core::pin::Pin;

// ===========================================================================
// Extensions
// ===========================================================================

/// A type-erased, shareable data store attached to a [`Request`].
///
/// Middleware uses this to pass data downstream: authenticated user info,
/// request IDs, rate-limit tokens, parsed sessions, etc.
///
/// Cloning an `Extensions` is cheap — it shares the underlying map via
/// `Arc`. Writes use interior mutability.
///
/// # Example
///
/// ```
/// use edgerun_http::Request;
///
/// fn auth_middleware(req: &mut Request) {
///     // After verifying a token:
///     req.extensions_mut().insert("user_id".to_string());
/// }
///
/// fn handler(req: Request) {
///     let user_id: Option<String> = req.extensions().get();
/// }
/// ```
#[derive(Clone)]
pub struct Extensions {
    inner: Arc<SpinMutex<BTreeMap<TypeId, Box<dyn Any + Send>>>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SpinMutex::new(BTreeMap::new())),
        }
    }

    /// Insert a value, replacing any existing value of the same type.
    pub fn insert<T: Send + 'static>(&self, value: T) {
        self.inner.lock().insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a reference to a value by type.
    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.inner
            .lock()
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .cloned()
    }

    /// Insert and return the old value of the same type, if any.
    pub fn replace<T: Send + 'static>(&self, value: T) -> Option<T> {
        self.inner
            .lock()
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|b| *b)
    }

    /// Remove and return a value by type.
    pub fn remove<T: 'static>(&self) -> Option<T> {
        self.inner
            .lock()
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|b| *b)
    }

    /// Returns true if a value of this type exists.
    pub fn contains<T: 'static>(&self) -> bool {
        self.inner.lock().contains_key(&TypeId::of::<T>())
    }

    /// Clear all extensions.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guard = self.inner.lock();
        f.debug_struct("Extensions")
            .field("count", &guard.len())
            .finish()
    }
}

// ===========================================================================
// Middleware trait
// ===========================================================================

/// Handle to the rest of the middleware chain.
///
/// Calling `.run(req)` consumes the handle and passes the request to the
/// next middleware or final handler. The `self`-by-value prevents calling
/// `.run()` twice — you must explicitly short-circuit or pass through.
pub struct Next {
    inner: Arc<dyn Handler>,
}

impl Next {
    pub fn new(inner: Arc<dyn Handler>) -> Self {
        Self { inner }
    }

    /// Pass the request to the next middleware or final handler.
    pub async fn run(self, req: Request) -> Response {
        self.inner.handle(req).await
    }
}

/// A middleware component.
///
/// Implement this trait to intercept requests before they reach the handler.
/// Applied via [`Chain`].
pub trait Middleware: Send + Sync + 'static {
    fn call(&self, req: Request, next: Next)
        -> Pin<Box<dyn Future<Output = Response> + Send + '_>>;
}

/// Function-based middleware. Created via [`middleware_fn`].
pub struct FnMiddleware<F> {
    f: F,
}

/// Create a middleware from a closure.
///
/// ```
/// let cors = edgerun_http::middleware::middleware_fn(|req, next| {
///     Box::pin(async move {
///         let mut resp = next.run(req).await;
///         resp.headers_mut().insert("X-Powered-By", "edgerun");
///         resp
///     })
/// });
/// ```
pub fn middleware_fn<F>(f: F) -> FnMiddleware<F>
where
    F: Fn(Request, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static,
{
    FnMiddleware { f }
}

impl<F> Middleware for FnMiddleware<F>
where
    F: Fn(Request, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync + 'static,
{
    fn call(
        &self,
        req: Request,
        next: Next,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin((self.f)(req, next))
    }
}

// ===========================================================================
// Chain builder
// ===========================================================================

/// Builder for stacking middleware on top of a handler.
///
/// Middlewares are applied **outside → in**: the first `.with()` is the
/// outermost layer, the last `.with()` is closest to the handler.
///
/// ```text
/// let handler = edgerun_http::middleware::Chain::new(my_handler)
///     .with(cors())      // outermost
///     .with(logging())
///     .with(auth())      // innermost (closest to handler)
///     .build();
/// ```
pub struct Chain {
    middlewares: Vec<Arc<dyn Middleware>>,
    handler: Arc<dyn Handler>,
}

impl Chain {
    pub fn new<H: Handler>(handler: H) -> Self {
        Self {
            middlewares: Vec::new(),
            handler: Arc::new(handler),
        }
    }

    /// Push a middleware onto the stack.
    pub fn with<M: Middleware>(mut self, mw: M) -> Self {
        self.middlewares.push(Arc::new(mw));
        self
    }

    /// Compile the middleware chain into a single [`Handler`].
    ///
    /// Wraps from the inside out so the first `.with()` becomes the
    /// outermost layer.
    pub fn build(self) -> impl Handler {
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

/// A single layer: wraps one middleware around an inner handler.
struct MiddlewareLayer {
    middleware: Arc<dyn Middleware>,
    inner: Arc<dyn Handler>,
}

impl Handler for MiddlewareLayer {
    fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let mw = Arc::clone(&self.middleware);
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let next = Next::new(inner);
            mw.call(req, next).await
        })
    }
}
