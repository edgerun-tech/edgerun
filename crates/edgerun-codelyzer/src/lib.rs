// Re-export modules for use by other crates
pub mod analyzer;
pub mod diagnostics;
#[allow(dead_code)]
pub mod edit;
pub mod filesystem;
pub mod git;
pub mod parser;
pub mod qwen;
pub mod repo_registry;
pub mod tools;
pub mod uir;

#[cfg(feature = "server")]
pub mod server_http;
