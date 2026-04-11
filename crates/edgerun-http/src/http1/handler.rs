//! HTTP handler trait and adapters for sync and async handlers.
//!
//! The [`Handler`] trait is the core abstraction — it accepts an HTTP request
//! and returns a response asynchronously.
//!
//! # Creating handlers
//!
//! ## From a sync function
//! ```no_run
//! use edgerun_http::http1::{Handler, into_handler, Request, Response, StatusCode};
//!
//! fn my_handler(req: Request) -> Response {
//!     Response::new(StatusCode::OK)
//! }
//!
//! let handler = into_handler(my_handler);
//! ```
//!
//! ## From an async function
//! ```no_run
//! use edgerun_http::http1::{Handler, into_handler_async, Request, Response, StatusCode};
//!
//! async fn my_async_handler(req: Request) -> Response {
//!     Response::new(StatusCode::OK)
//! }
//!
//! let handler = into_handler_async(my_async_handler);
//! ```
//!
//! ## From a closure
//! ```no_run
//! use edgerun_http::http1::{into_handler, Request, Response, StatusCode};
//!
//! let handler = into_handler(|req: Request| {
//!     Response::new(StatusCode::OK)
//! });
//! ```

use std::future::Future;
use std::pin::Pin;

use crate::http1::request::Request;
use crate::http1::response::Response;

/// An HTTP request handler.
///
/// Implement this trait to handle HTTP requests. The handler receives a
/// fully parsed [`Request`] and returns a [`Response`].
pub trait Handler: Send + Sync + 'static {
    /// Handle an incoming request.
    fn handle(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>>;
}

/// Create a handler from a sync function or closure.
///
/// The function is executed on the runtime's blocking thread pool
/// (via `spawn_blocking`) so it may perform blocking I/O without
/// stalling the reactor.
pub fn into_handler<F>(f: F) -> SyncHandler<F>
where
    F: Fn(Request) -> Response + Send + Sync + 'static,
{
    SyncHandler { f }
}

/// Create a handler from an async function or closure.
///
/// The function is executed directly on the async runtime, so it
/// should NOT perform blocking I/O. Use `into_handler` for handlers
/// that need blocking I/O.
pub fn into_handler_async<F, Fut>(f: F) -> AsyncHandler<F>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    AsyncHandler { f }
}

// ---------------------------------------------------------------------------
// SyncHandler — wraps a sync function
// ---------------------------------------------------------------------------

/// Handler adapter for sync functions.
/// Created via [`into_handler`].
pub struct SyncHandler<F> {
    f: F,
}

impl<F> Handler for SyncHandler<F>
where
    F: Fn(Request) -> Response + Send + Sync + 'static,
{
    fn handle(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let resp = (self.f)(request);
        Box::pin(async move { resp })
    }
}

impl<F> Clone for SyncHandler<F>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self { f: self.f.clone() }
    }
}

// ---------------------------------------------------------------------------
// AsyncHandler — wraps an async function
// ---------------------------------------------------------------------------

/// Handler adapter for async functions.
/// Created via [`into_handler_async`].
pub struct AsyncHandler<F> {
    f: F,
}

impl<F, Fut> Handler for AsyncHandler<F>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn handle(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let f = &self.f;
        Box::pin(async move { f(request).await })
    }
}

impl<F> Clone for AsyncHandler<F>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self { f: self.f.clone() }
    }
}

// ---------------------------------------------------------------------------
// Handler for Arc<dyn Handler> (for shared ownership)
// ---------------------------------------------------------------------------

impl Handler for std::sync::Arc<dyn Handler> {
    fn handle(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let this = self.clone();
        Box::pin(async move { this.handle(request).await })
    }
}

// ---------------------------------------------------------------------------
// Response builder helpers for use in handlers
// ---------------------------------------------------------------------------

/// Convenience type for a handler that returns text responses.
pub fn text_response(status: crate::StatusCode, body: &str) -> Response {
    let mut resp = Response::new(status);
    let _ = resp.headers_mut().insert("Content-Type", "text/plain; charset=utf-8");
    let _ = resp.headers_mut().insert(
        "Content-Length",
        &body.len().to_string(),
    );
    resp.set_body(body.as_bytes().to_vec());
    resp
}

/// Convenience type for a handler that returns JSON responses.
pub fn json_response(status: crate::StatusCode, body: &str) -> Response {
    let mut resp = Response::new(status);
    let _ = resp.headers_mut().insert("Content-Type", "application/json");
    let _ = resp.headers_mut().insert(
        "Content-Length",
        &body.len().to_string(),
    );
    resp.set_body(body.as_bytes().to_vec());
    resp
}

/// Convenience type for a handler that returns HTML responses.
pub fn html_response(status: crate::StatusCode, body: &str) -> Response {
    let mut resp = Response::new(status);
    let _ = resp.headers_mut().insert("Content-Type", "text/html; charset=utf-8");
    let _ = resp.headers_mut().insert(
        "Content-Length",
        &body.len().to_string(),
    );
    resp.set_body(body.as_bytes().to_vec());
    resp
}

/// 404 Not Found response.
pub fn not_found() -> Response {
    text_response(crate::StatusCode::new(404).unwrap(), "404 Not Found")
}

/// 500 Internal Server Error response.
pub fn internal_error() -> Response {
    text_response(crate::StatusCode::new(500).unwrap(), "500 Internal Server Error")
}

/// 405 Method Not Allowed response.
pub fn method_not_allowed() -> Response {
    text_response(crate::StatusCode::new(405).unwrap(), "405 Method Not Allowed")
}

/// 101 Switching Protocols response with upgrade headers.
///
/// Use this to accept an HTTP upgrade. Add the appropriate upgrade-specific
/// headers (e.g., WebSocket accept headers) to the returned response.
pub fn switching_protocols_response() -> Response {
    Response::switching_protocols()
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_rt::Runtime;
    use crate::{Method, StatusCode};

    #[test]
    fn test_sync_handler() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://localhost/")
            .build()
            .unwrap();

        let response = rt.block_on(async {
            let handler = into_handler(|_req: Request| {
                Response::new(StatusCode::new(200).unwrap())
            });
            handler.handle(request).await
        });
        assert!(response.is_success());
    }

    #[test]
    fn test_async_handler() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://localhost/")
            .build()
            .unwrap();

        let response = rt.block_on(async {
            let handler = into_handler_async(|_req: Request| async {
                Response::new(StatusCode::new(200).unwrap())
            });
            handler.handle(request).await
        });
        assert!(response.is_success());
    }

    #[test]
    fn test_text_response() {
        let resp = text_response(StatusCode::new(200).unwrap(), "hello");
        assert!(resp.is_success());
        assert_eq!(resp.body_as_string().unwrap(), "hello");
    }

    #[test]
    fn test_json_response() {
        let resp = json_response(StatusCode::new(200).unwrap(), r#"{"key":"value"}"#);
        assert!(resp.is_success());
        assert_eq!(resp.body_as_string().unwrap(), r#"{"key":"value"}"#);
    }
}
