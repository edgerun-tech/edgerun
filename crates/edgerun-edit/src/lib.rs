//! AST-level Rust code editor.
//!
//! This crate provides both a CLI binary and a library of AST-level
//! edit operations for Rust source files.

pub mod edit_ops;
pub mod git;
pub mod project;
mod server;

pub use server::start as start_server;
