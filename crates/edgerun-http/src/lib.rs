//! A dependency-free HTTP/1.1, HTTP/2, and HTTP/3 implementation built exclusively with
//! Rust's standard library.
//!
//! # Unified API
//!
//! - [`HttpServer`] — Unified server handling HTTP/1.1, HTTP/2, and HTTP/3
//! - [`HttpClient`] — Unified client with automatic protocol negotiation
//! - [`Handler`] — Request handler trait that works across all protocols
//! - [`Request`] / [`Response`] — Protocol-agnostic request/response types
//!
//! # HTTP/1.1 Example
//! ```no_run
//! use edgerun_http::{HttpServer, Handler, Request, Response, StatusCode};
//!
//! struct HelloHandler;
//!
//! impl Handler for HelloHandler {
//!     fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
//!         Box::pin(async move {
//!             Response::text(
//!                 StatusCode::from_u16(200).unwrap(),
//!                 &format!("Hello from {}!", req.uri().request_target()),
//!             )
//!         })
//!     }
//! }
//!
//! # edgerun_rt::block_on(async {
//! HttpServer::new(HelloHandler)
//!     .bind("127.0.0.1:0")
//!     .await
//!     .unwrap()
//!     .serve()
//!     .await
//!     .unwrap();
//! # });
//! ```
//!
//! # Client Example
//! ```no_run
//! use edgerun_http::HttpClient;
//!
//! # edgerun_rt::block_on(async {
//! let client = HttpClient::new();
//! let response = client.get("http://example.com/").await.unwrap();
//! println!("Status: {}", response.status().as_u16());
//! # });
//! ```
//!
//! # Protocol Modules
//! - [`http1`] — HTTP/1.1 client and types (RFC 9112, RFC 9110)
//! - [`http2`] — HTTP/2 with HPACK, frames, streams, flow control (RFC 9113, RFC 7541)
//! - [`http3`] — HTTP/3 with QUIC transport and QPACK (RFC 9114, RFC 9000, RFC 9204)
//!
//! # Shared Types
//! - [`Method`] — HTTP methods (GET, POST, etc.)
//! - [`StatusCode`] — HTTP status codes with category helpers
//! - [`HeaderName`] / [`HeaderValue`] / [`HeaderMap`] — Header types
//! - [`Uri`] / [`Scheme`] — URI parsing
//! - [`Error`] / [`Result`] — Shared error types
//!
//! # Official Specifications
//! This crate implements the following RFCs:
//!
//! | Module | RFC | Title |
//! |--------|-----|-------|
//! | http1  | [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110) | HTTP Semantics |
//! | http1  | [RFC 9112](https://www.rfc-editor.org/rfc/rfc9112) | HTTP/1.1 Message Syntax |
//! | http2  | [RFC 9113](https://www.rfc-editor.org/rfc/rfc9113) | HTTP/2 |
//! | http2  | [RFC 7541](https://www.rfc-editor.org/rfc/rfc7541) | HPACK Header Compression |
//! | http3  | [RFC 9114](https://www.rfc-editor.org/rfc/rfc9114) | HTTP/3 |
//! | http3  | [RFC 9000](https://www.rfc-editor.org/rfc/rfc9000) | QUIC Transport |
//! | http3  | [RFC 9001](https://www.rfc-editor.org/rfc/rfc9001) | QUIC TLS Mapping |
//! | http3  | [RFC 9204](https://www.rfc-editor.org/rfc/rfc9204) | QPACK Header Compression |

#![allow(non_camel_case_types)] // HTTP/2 error code names follow RFC 9113

// ---------------------------------------------------------------------------
// Shared HTTP types
// ---------------------------------------------------------------------------
pub mod header;
pub mod method;
pub mod status;
pub mod uri;
pub mod error;

pub use error::{Error, HttpError, Result};
pub use header::{is_tchar, HeaderMap, HeaderName, HeaderValue};
pub use method::Method;
pub use status::StatusCode;
pub use uri::{Scheme, Uri};

// ---------------------------------------------------------------------------
// Unified API — cross-protocol server, client, handler, request, response
// ---------------------------------------------------------------------------
pub mod handler;
pub mod request;
pub mod response;
pub mod server;
pub mod client;

pub use handler::{Handler, into_handler, into_handler_async, SyncHandler, AsyncHandler};
pub use request::{Request, RequestBuilder};
pub use response::Response;
pub use server::{HttpServer, BoundHttpServer, TlsCertificate};
pub use client::{HttpClient, HttpVersion};

// ---------------------------------------------------------------------------
// Middleware system
// ---------------------------------------------------------------------------
pub mod middleware;
pub use middleware::{Extensions, Middleware, Next, Chain, middleware_fn, FnMiddleware};

// ---------------------------------------------------------------------------
// Client middleware system
// ---------------------------------------------------------------------------
pub mod client_middleware;
pub use client_middleware::{
    ClientExtensions, ClientRequest, ClientMiddleware, ClientNext, ClientTransport,
    client_middleware_fn, FnClientMiddleware, Chain as ClientChain, Client,
};

// ---------------------------------------------------------------------------
// Protocol-specific modules
// ---------------------------------------------------------------------------
pub mod http1;
pub mod http2;
pub mod http3;

#[cfg(test)]
mod semantics_conformance;
