//! Node-owned runtime surface.
//!
//! This is currently a compatibility bridge over `edgerun-rt`. New node service
//! code should import runtime primitives through this module so the runtime
//! implementation can move under `edgerun-node` without preserving a separate
//! public runtime crate boundary.

pub use edgerun_rt::*;
