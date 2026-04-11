//! Async HTTP/1.1 implementation (RFC 9112)
//!
//! This module provides HTTP/1.1 client and server built on top of
//! [`edgerun_rt`] async primitives.

pub mod client;
pub mod server;
pub mod handler;
pub mod body;
pub mod buf_reader;
pub mod request;
pub mod response;

pub use client::Client;
pub use server::{Server, BoundServer};
#[cfg(feature = "tls")]
pub use server::{TlsServer, TlsBoundServer};
pub use handler::{Handler, into_handler, into_handler_async};
pub use body::{Body, BodyReader, BodySender, AsyncBodyReader};
pub use buf_reader::BufReader;
pub use request::{Request, RequestBuilder};
pub use response::Response;
