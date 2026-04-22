#![allow(missing_docs)]
//! HTTP proxy server with CONNECT tunnel support (RFC 9110 / RFC 9111).
//!
//! # Architecture
//! - **HTTP CONNECT** — establish TCP tunnels to arbitrary hosts
//! - **HTTP proxy** — forward HTTP/HTTPS requests through upstream proxies
//! - **Upstream chaining** — configurable upstream proxy URL
//!
//! # Flow
//! ```text
//! Client                      Proxy                   Upstream
//!   |                          |                        |
//!   |-- CONNECT target:443 -->|                        |
//!   |<-- 200 Connection OK ---|                        |
//!   |                        |-- CONNECT target:443 -->|
//!   |                        |<-- 200 Connection OK -|
//!   |                        |<-- 200 OK ------------|
//!   |<-- 200 OK -------------|                        |
//!   |=========================| (tunnel established) |
//!   |<~~~~ encrypted data ~~~~>|~~~~ encrypted data ~~~~>|
//! ```

pub mod server;

pub use server::{ProxyConfig, ProxyServer};