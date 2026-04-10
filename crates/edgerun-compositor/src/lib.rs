//! Pure Rust Wayland compositor.
//!
//! No external C libraries — direct Linux kernel via ioctls and raw sockets.

#![warn(missing_docs)]

pub mod wire;
pub mod protocol;
pub mod server;
pub mod client;
pub mod resource;
pub mod drm;
pub mod input;
pub mod compositor;
pub mod render;
pub mod r#loop;
pub mod gpu;
