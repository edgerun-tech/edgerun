//! Handler trait for HTTP servers.
//!
//! Works across HTTP/1.1, HTTP/2, and HTTP/3.

use crate::{Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A trait for processing HTTP requests.
///
/// Implement this trait to handle requests from HTTP/1.1, HTTP/2, and HTTP/3
/// servers with a single implementation.
///
/// # Example
///
/// ```
/// use edgerun_http::{Handler, Request, Response, StatusCode};
/// use std::future::Future;
/// use std::pin::Pin;
///
/// struct MyHandler;
///
/// impl Handler for MyHandler {
///     fn handle(&self, _req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
///         Box::pin(async move {
///             Response::text(StatusCode::new(200).unwrap(), "Hello!")
///         })
///     }
/// }
/// ```
pub trait Handler: Send + Sync + 'static {
    fn handle(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + '_>>;
}

/// Wrap a synchronous function as a [`Handler`].
///
/// ```
/// use edgerun_http::{into_handler, Response, StatusCode};
///
/// let handler = into_handler(|request| {
///     Response::text(
///         StatusCode::new(200).unwrap(),
///         &format!("Got: {} {}", request.method().as_str(), request.uri().request_target()),
///     )
/// });
/// ```
pub fn into_handler<F>(f: F) -> SyncHandler<F>
where
    F: Fn(Request) -> Response + Send + Sync + 'static,
{
    SyncHandler { f }
}

/// Wrap an async function as a [`Handler`].
///
/// ```
/// use edgerun_http::{into_handler_async, Response, StatusCode};
///
/// let handler = into_handler_async(|request| async move {
///     Response::text(
///         StatusCode::new(200).unwrap(),
///         &format!("Hello, {}!", request.uri().request_target()),
///     )
/// });
/// ```
pub fn into_handler_async<F, Fut>(f: F) -> AsyncHandler<F>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    AsyncHandler { f }
}

/// A handler that wraps a synchronous function.
pub struct SyncHandler<F> { f: F }

impl<F> Handler for SyncHandler<F>
where
    F: Fn(Request) -> Response + Send + Sync + 'static,
{
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { (self.f)(request) })
    }
}

/// A handler that wraps an async function.
pub struct AsyncHandler<F> { f: F }

impl<F, Fut> Handler for AsyncHandler<F>
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin((self.f)(request))
    }
}

impl Handler for Arc<dyn Handler> {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let handler = Arc::clone(self);
        Box::pin(async move { handler.handle(request).await })
    }
}
