//! HTTP/1.1 implementation (RFC 9112)
//!
//! This module provides HTTP/1.1 client and types built on top of the shared types.

pub mod client;
pub mod request;
pub mod response;

pub use client::Client;
pub use request::{Request, RequestBuilder};
pub use response::Response;
