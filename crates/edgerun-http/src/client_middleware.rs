//! Client-side middleware system.
//!
//! Mirrors the server-side [`crate::middleware`] API but wraps
//! [`crate::client::HttpClient::execute`] instead of [`crate::Handler`].
//!
//! # Example
//!
//! ```
//! use edgerun_http::client_middleware::{Chain, ClientMiddleware, ClientNext};
//! use edgerun_http::{HttpClient, Request, Response, Result};
//! use std::future::Future;
//! use std::pin::Pin;
//! use std::time::Duration;
//!
//! // Retry with exponential backoff
//! struct Retry { max_attempts: u32 }
//! impl ClientMiddleware for Retry {
//!     fn call(
//!         &self,
//!         req: Request,
//!         next: ClientNext,
//!     ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>> {
//!         let max = self.max_attempts;
//!         Box::pin(async move {
//!             let mut last_err = None;
//!             for attempt in 0..max {
//!                 match next.run(req.clone()).await {
//!                     Ok(resp) if resp.is_success() => return Ok(resp),
//!                     Ok(resp) if attempt + 1 < max => {
//!                         edgerun_bare_rt::sleep(Duration::from_millis(100 * 2u64.pow(attempt as u32))).await;
//!                         continue;
//!                     }
//!                     Err(e) => {
//!                         last_err = Some(e);
//!                         if attempt + 1 < max {
//!                             edgerun_bare_rt::sleep(Duration::from_millis(100 * 2u64.pow(attempt as u32))).await;
//!                             continue;
//!                         }
//!                     }
//!                     Ok(resp) => return Ok(resp),
//!                 }
//!             }
//!             Err(last_err.unwrap_or_else(|| crate::Error::ProtocolError("max retries exhausted".into())))
//!         })
//!     }
//! }
//!
//! // Function-based: add auth header
//! let auth = edgerun_http::client_middleware::client_middleware_fn(|req, next| {
//!     Box::pin(async move {
//!         let mut req = req;
//!         req.headers_mut().insert("Authorization", "Bearer my-token");
//!         next.run(req).await
//!     })
//! });
//!
//! // Stack them
//! let client = Chain::new(HttpClient::new())
//!     .with(auth)
//!     .with(Retry { max_attempts: 3 })
//!     .build();
//!
//! let resp = client.get("https://api.example.com/data").await?;
//! ```

use crate::client::HttpClient;
use crate::header::HeaderMap;
use crate::method::Method;
use crate::{Error, Request, Response, Result};
use edgerun_bare_rt::sync::Mutex;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ===========================================================================
// ClientExtensions — type-erased data attached to client requests/responses
// ===========================================================================

/// Type-erased data store for client middleware to share state.
///
/// Unlike server [`crate::middleware::Extensions`], these live on the
/// `ClientRequest` wrapper rather than the raw [`Request`].
#[derive(Clone)]
pub struct ClientExtensions {
    inner: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send>>>>,
}

impl ClientExtensions {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert<T: Send + 'static>(&self, value: T) {
        self.inner.lock().insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Clone + 'static>(&self) -> Option<T> {
        self.inner
            .lock()
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
            .cloned()
    }

    pub fn replace<T: Send + 'static>(&self, value: T) -> Option<T> {
        self.inner
            .lock()
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|b| *b)
    }

    pub fn remove<T: 'static>(&self) -> Option<T> {
        self.inner
            .lock()
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|b| *b)
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.inner.lock().contains_key(&TypeId::of::<T>())
    }

    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

impl Default for ClientExtensions {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ClientExtensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.inner.lock();
        f.debug_struct("ClientExtensions")
            .field("count", &guard.len())
            .finish()
    }
}

// ===========================================================================
// ClientRequest — wraps Request with client-side metadata
// ===========================================================================

/// A request with client-side metadata (extensions) attached.
///
/// Middleware can attach tracing IDs, auth tokens, retry config, etc.
#[derive(Clone)]
pub struct ClientRequest {
    request: Request,
    extensions: ClientExtensions,
}

impl ClientRequest {
    pub fn new(request: Request) -> Self {
        Self {
            request,
            extensions: ClientExtensions::new(),
        }
    }

    pub fn from_builder(builder: crate::request::RequestBuilder) -> Result<Self> {
        Ok(Self::new(builder.build()?))
    }

    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn request_mut(&mut self) -> &mut Request {
        &mut self.request
    }
    pub fn into_request(self) -> Request {
        self.request
    }

    pub fn method(&self) -> &Method {
        self.request.method()
    }
    pub fn uri(&self) -> &crate::uri::Uri {
        self.request.uri()
    }
    pub fn headers(&self) -> &HeaderMap {
        self.request.headers()
    }
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.request.headers_mut()
    }
    pub fn body(&self) -> Option<&[u8]> {
        self.request.body()
    }

    pub fn extensions(&self) -> &ClientExtensions {
        &self.extensions
    }
    pub fn extensions_mut(&mut self) -> &mut ClientExtensions {
        &mut self.extensions
    }
}

// ===========================================================================
// ClientMiddleware trait
// ===========================================================================

/// Handle to the rest of the client middleware chain.
pub struct ClientNext {
    inner: Arc<dyn ClientTransport>,
}

impl ClientNext {
    pub fn new(inner: Arc<dyn ClientTransport>) -> Self {
        Self { inner }
    }

    /// Pass the request to the next middleware or final transport.
    pub async fn run(self, req: ClientRequest) -> Result<Response> {
        self.inner.execute(&req).await
    }
}

/// The innermost transport — actually sends the request over the network.
pub trait ClientTransport: Send + Sync + 'static {
    fn execute(
        &self,
        req: &ClientRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>>;
}

/// A client middleware that wraps [`ClientTransport`].
pub trait ClientMiddleware: Send + Sync + 'static {
    fn call(
        &self,
        req: ClientRequest,
        next: ClientNext,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>>;
}

/// Function-based client middleware. Created via [`client_middleware_fn`].
pub struct FnClientMiddleware<F> {
    f: F,
}

/// Create a client middleware from a closure.
pub fn client_middleware_fn<F>(f: F) -> FnClientMiddleware<F>
where
    F: Fn(ClientRequest, ClientNext) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>
        + Send
        + Sync
        + 'static,
{
    FnClientMiddleware { f }
}

impl<F> ClientMiddleware for FnClientMiddleware<F>
where
    F: Fn(ClientRequest, ClientNext) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>>
        + Send
        + Sync
        + 'static,
{
    fn call(
        &self,
        req: ClientRequest,
        next: ClientNext,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>> {
        Box::pin((self.f)(req, next))
    }
}

// ===========================================================================
// Chain builder
// ===========================================================================

/// Builder for stacking client middleware on top of an [`HttpClient`].
///
/// ```
/// let client = edgerun_http::client_middleware::Chain::new(edgerun_http::HttpClient::new())
///     .with(logging())
///     .with(retry(3))
///     .build();
/// ```
pub struct Chain {
    middlewares: Vec<Arc<dyn ClientMiddleware>>,
    transport: Arc<dyn ClientTransport>,
}

impl Chain {
    pub fn new(client: HttpClient) -> Self {
        Self {
            middlewares: Vec::new(),
            transport: Arc::new(ClientTransportAdapter(client)),
        }
    }

    pub fn with<M: ClientMiddleware>(mut self, mw: M) -> Self {
        self.middlewares.push(Arc::new(mw));
        self
    }

    /// Compile into a [`Client`] that can make requests.
    pub fn build(self) -> Client {
        let mut t = self.transport;
        for mw in self.middlewares.into_iter().rev() {
            t = Arc::new(ClientMiddlewareLayer {
                middleware: mw,
                inner: t,
            });
        }
        Client { transport: t }
    }
}

/// Wraps `HttpClient` as a `ClientTransport`.
struct ClientTransportAdapter(HttpClient);

impl ClientTransport for ClientTransportAdapter {
    fn execute(
        &self,
        req: &ClientRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>> {
        let client = &self.0;
        let req = req.request().clone();
        Box::pin(async move { client.execute(&req).await })
    }
}

/// A single middleware layer wrapping an inner transport.
struct ClientMiddlewareLayer {
    middleware: Arc<dyn ClientMiddleware>,
    inner: Arc<dyn ClientTransport>,
}

impl ClientTransport for ClientMiddlewareLayer {
    fn execute(
        &self,
        req: &ClientRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + '_>> {
        let mw = Arc::clone(&self.middleware);
        let inner = Arc::clone(&self.inner);
        let req = req.clone();
        Box::pin(async move {
            let next = ClientNext::new(inner);
            mw.call(req, next).await
        })
    }
}

// ===========================================================================
// Client — the composed client with middleware
// ===========================================================================

/// A client with middleware applied.
///
/// Created via [`Chain::build`]. Provides the same convenience methods as
/// [`HttpClient`] but routes through the middleware stack.
pub struct Client {
    transport: Arc<dyn ClientTransport>,
}

impl Client {
    pub async fn get(&self, uri: &str) -> Result<Response> {
        let req = Request::builder().method(Method::GET).uri(uri).build()?;
        self.execute(ClientRequest::new(req)).await
    }

    pub async fn post(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(ClientRequest::new(req)).await
    }

    pub async fn post_json(&self, uri: &str, json: &str) -> Result<Response> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .json_body(json)
            .build()?;
        self.execute(ClientRequest::new(req)).await
    }

    pub async fn put(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let req = Request::builder()
            .method(Method::PUT)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(ClientRequest::new(req)).await
    }

    pub async fn delete(&self, uri: &str) -> Result<Response> {
        let req = Request::builder().method(Method::DELETE).uri(uri).build()?;
        self.execute(ClientRequest::new(req)).await
    }

    pub async fn patch(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let req = Request::builder()
            .method(Method::PATCH)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(ClientRequest::new(req)).await
    }

    /// Execute a [`ClientRequest`] through the full middleware stack.
    pub async fn execute(&self, req: ClientRequest) -> Result<Response> {
        self.transport.execute(&req).await
    }
}
