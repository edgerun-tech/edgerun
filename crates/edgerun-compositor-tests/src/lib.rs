//! Protocol conformance tests for edgerun-compositor.
//!
//! This crate provides both:
//! - A `#[test]` integration test suite (in `tests/conformance.rs`)
//! - A CLI binary `protocol-conformance` for visual/overlay testing

pub mod harness;
pub mod client;
pub mod spec;
pub mod protocols;
