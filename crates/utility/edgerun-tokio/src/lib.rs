//! Edgerun-owned compatibility surface for Tokio-shaped APIs.
//!
//! The `edgerun` module exposes Edgerun's native runtime primitives. The
//! crate-root compatibility API currently forwards Tokio's broad surface so
//! Codex can migrate imports first without losing behavior.

#![cfg_attr(not(feature = "compat"), no_std)]

#[cfg(feature = "edgerun-runtime")]
pub mod edgerun {
    pub use edgerun_node::rt::*;
}

#[cfg(feature = "compat")]
pub use tokio::*;

#[cfg(all(feature = "compat", feature = "edgerun-runtime"))]
pub mod runtime_bridge {
    pub use crate::edgerun;
    pub use tokio as compat;
}
