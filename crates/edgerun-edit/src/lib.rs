//! AST-level Rust code editor.
//!
//! This crate provides both a CLI binary and a library of AST-level
//! edit operations for Rust source files.

#![no_std]

#[cfg(all(feature = "std", not(target_os = "none")))]
extern crate std;

#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod edit_ops;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod git;
#[cfg(all(feature = "std", not(target_os = "none")))]
pub mod project;
#[cfg(all(feature = "std", not(target_os = "none")))]
mod server;

#[cfg(all(feature = "std", not(target_os = "none")))]
pub use server::start as start_server;
