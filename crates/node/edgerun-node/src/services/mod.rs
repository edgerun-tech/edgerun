//! Node-owned runtime services.
//!
//! Protocol modules and apps can request HTTP, DNS, DHCP, SMTP, IMAP, LMTP,
//! TFTP, and proxy service bindings. The node owns the actual resources and
//! decides whether those requests become native sockets, browser message
//! routes, mesh routes, or no binding on the current host.
//!
//! HTTP handlers live inside apps. The node owns the listener and routes HTTP
//! requests to the target app over node IPC.
//!
//! # Example
//! ```no_run
//! use edgerun_node::services::NodeRuntime;
//! # async fn example() -> std::io::Result<()> {
//! let mut runtime = NodeRuntime::new()
//!     .with_http_app("127.0.0.1:8080", [7; 32])
//!     .build()
//!     .await?;
//!
//! let shutdown = crate::rt::CancellationToken::new();
//! runtime.run(shutdown).await
//! # }
//! ```
//!
use crate::rt::CancellationToken;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::module_path;
use core::time::Duration;

#[cfg(target_os = "none")]
use crate::rt::io;
#[cfg(not(target_os = "none"))]
use std::io;

#[cfg(feature = "http")]
use self::http_runtime::HttpNodeBinding;
#[cfg(feature = "http")]
use self::middleware::{
    ConnectionChain, ConnectionHandler, ConnectionMiddleware, MiddlewareAdapter, PassThroughHandler,
};
#[cfg(all(feature = "http", feature = "tls"))]
use edgerun_protocols::tls::CertificateAndKey as TlsCertificate;

#[cfg(any(feature = "http", feature = "imap", feature = "smtp", feature = "lmtp"))]
use crate::network::{HostSocketTransport, TransportAddress};

#[cfg(feature = "acme")]
pub mod acme_runtime;
pub mod config;
#[cfg(feature = "dhcp")]
pub mod dhcp_runtime;
#[cfg(feature = "dns")]
pub mod dns_runtime;
#[cfg(feature = "http")]
pub mod http_runtime;
#[cfg(any(feature = "imap", feature = "smtp", feature = "lmtp"))]
pub mod mail_runtime;
#[cfg(feature = "http")]
pub mod middleware;
#[cfg(feature = "proxy")]
mod proxy_runtime;
#[cfg(feature = "tftp")]
pub mod tftp_runtime;
#[cfg(all(feature = "virtual-disk", not(target_os = "none")))]
pub mod virtual_disk_runtime;
pub use crate::network::{
    binding_intents, decide_binding, decide_bindings, NodeTransportSurface, ServiceBindingDecision,
    ServiceBindingIntent,
};
pub use config::*;
#[cfg(any(
    feature = "http",
    feature = "dns",
    feature = "dhcp",
    feature = "tftp",
    feature = "proxy"
))]
fn other_io_error(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("{error}"))
}

#[cfg(any(feature = "http", feature = "imap", feature = "smtp", feature = "lmtp"))]
fn bind_node_tcp_listener(addr: &str) -> io::Result<crate::rt::AsyncTcpListener> {
    HostSocketTransport
        .bind_stream_now(&TransportAddress::host_stream(addr.as_bytes().to_vec()))
        .map_err(other_io_error)
}

// ---------------------------------------------------------------------------
// NodeRuntime
// ---------------------------------------------------------------------------

mod bound;
mod builder;

pub use bound::BoundNodeRuntime;
pub use builder::NodeRuntime;
