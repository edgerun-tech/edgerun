//! Edgerun-owned compatibility surface for Tokio utility APIs.
//!
//! Native Edgerun equivalents are exposed under `edgerun`; the crate-root
//! compatibility API forwards `tokio-util` while Codex call sites migrate.

#![cfg_attr(not(feature = "compat"), no_std)]

#[cfg(feature = "edgerun-runtime")]
pub mod edgerun {
    pub use edgerun_tokio::edgerun::CancellationToken;
    pub use edgerun_tokio::edgerun::Cancelled;
}

#[cfg(feature = "compat")]
pub use tokio_util::*;
