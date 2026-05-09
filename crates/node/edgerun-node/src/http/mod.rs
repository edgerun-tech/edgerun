//! A dependency-free HTTP/1.1, HTTP/2, and HTTP/3 implementation built with a shared
//! host/bare-metal compatibility layer (`std` on host, `core`/`alloc` on `target_os = "none"`).
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
//! # #[cfg(feature = "server")]
//! # {
//! use edgerun_node::http::{HttpServer, Handler, Request, Response, StatusCode};
//!
//! struct HelloHandler;
//!
//! impl Handler for HelloHandler {
//!     fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
//!         Box::pin(async move {
//!             Response::text(
//!                 StatusCode::new(200).unwrap(),
//!                 &format!("Hello from {}!", req.uri().request_target()),
//!             )
//!         })
//!     }
//! }
//!
//! # edgerun_node::rt::Runtime::new_multi_thread().enable_all().build().unwrap().block_on(async {
//! HttpServer::new(HelloHandler)
//!     .bind("127.0.0.1:0")
//!     .await
//!     .unwrap()
//!     .serve()
//!     .await
//!     .unwrap();
//! # });
//! # }
//! ```
//!
//! # Client Example
//! ```no_run
//! # #[cfg(feature = "client")]
//! # {
//! use edgerun_node::http::HttpClient;
//!
//! # edgerun_node::rt::Runtime::new_multi_thread().enable_all().build().unwrap().block_on(async {
//! let client = HttpClient::new();
//! let response = client.get("http://example.com/").await.unwrap();
//! println!("Status: {}", response.status().as_u16());
//! # });
//! # }
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

#![allow(non_camel_case_types)]

#[cfg(not(feature = "std"))]
#[path = "std.rs"]
mod std_compat;

#[cfg(feature = "runtime")]
pub mod runtime;
#[cfg(feature = "runtime")]
pub use runtime::{collections, fs, io, net, path, sync, time};
#[cfg(all(not(feature = "runtime"), feature = "std"))]
pub use std::{collections, fs, io, net, path, sync, time};
#[cfg(all(not(feature = "runtime"), not(feature = "std")))]
pub use std_compat::{collections, fs, io, net, path, sync, time};

// ---------------------------------------------------------------------------
// Shared HTTP types
// ---------------------------------------------------------------------------
mod chunked {
    pub use edgerun_protocols::http::chunked::*;
}
pub mod error;
pub mod header {
    pub use edgerun_protocols::http::header::*;
}
mod lock;
pub mod method {
    pub use edgerun_protocols::http::method::*;
}
pub mod status {
    pub use edgerun_protocols::http::status::*;
}
pub mod uri {
    pub use edgerun_protocols::http::uri::*;
}

pub use edgerun_protocols::http::{
    HeaderMap, HeaderName, HeaderValue, Method, Scheme, StatusCode, Uri, is_tchar,
};
pub use error::{Error, HttpError, Result};

// ---------------------------------------------------------------------------
// Unified API — cross-protocol server, client, handler, request, response
// ---------------------------------------------------------------------------
#[cfg(feature = "client")]
pub mod client;
pub mod handler;
pub mod request;
pub mod response;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "static-files")]
pub mod static_handler;

#[cfg(feature = "client")]
pub use client::{HttpClient, HttpVersion};
pub use handler::{AsyncHandler, Handler, SyncHandler, into_handler, into_handler_async};
pub use request::{Request, RequestBuilder};
pub use response::Response;
#[cfg(all(feature = "server", feature = "tls"))]
pub use server::TlsCertificate;
#[cfg(feature = "server")]
pub use server::{BoundHttpServer, HttpServer};
#[cfg(feature = "static-files")]
pub use static_handler::{StaticHandler, serve_static};

// ---------------------------------------------------------------------------
// Middleware system
// ---------------------------------------------------------------------------
pub mod middleware;
pub use middleware::{Chain, Extensions, FnMiddleware, Middleware, Next, middleware_fn};

// ---------------------------------------------------------------------------
// Connection middleware system (gated by feature)
// ---------------------------------------------------------------------------
#[cfg(feature = "connection-middleware")]
pub mod connection_middleware;
#[cfg(feature = "connection-middleware")]
pub use connection_middleware::{
    ConnectionChain, ConnectionHandler, ConnectionMiddleware, MiddlewareAdapter, PassThroughHandler,
};

// ---------------------------------------------------------------------------
// Client middleware system
// ---------------------------------------------------------------------------
#[cfg(feature = "client")]
pub mod client_middleware;
#[cfg(feature = "client")]
pub use client_middleware::{
    Chain as ClientChain, Client, ClientExtensions, ClientMiddleware, ClientNext, ClientRequest,
    ClientTransport, FnClientMiddleware, client_middleware_fn,
};

// ---------------------------------------------------------------------------
// Protocol-specific modules
// ---------------------------------------------------------------------------
#[cfg(feature = "http1")]
pub mod http1;
#[cfg(feature = "http2")]
pub mod http2;
#[cfg(feature = "http3")]
pub mod http3;

#[cfg(test)]
mod semantics_conformance;
