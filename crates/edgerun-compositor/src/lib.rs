//! Pure Rust Wayland compositor.
//!
//! No external C libraries — direct Linux kernel via ioctls and raw sockets.

// Protocol and FFI modules generate many doc/name warnings — suppress them
#![allow(missing_docs)]
#![allow(non_snake_case)]
#![allow(unused_comparisons)]

pub mod client;
pub mod compositor;
pub mod drm;
pub mod font;
pub mod gpu;
pub mod input;
pub mod logind;
pub mod r#loop;
pub mod protocol;
pub mod render;
pub mod resource;
pub mod server;
pub mod vt;
pub mod wire;
