// Re-export modules for use by other crates
pub mod analyzer;
pub mod codealyzer;
#[allow(dead_code)]
pub mod diagnostics;
#[allow(dead_code)]
pub mod edit;
#[allow(dead_code)]
pub mod filesystem;
#[allow(dead_code)]
pub mod git;
#[allow(dead_code)]
pub mod mcp_permission;
#[allow(dead_code)]
pub mod mcp_rust_ast;
#[allow(dead_code)]
pub mod parser;
pub mod tools;
#[allow(dead_code)]
pub mod uir;

/// Returns a current timestamp string (Unix seconds) without chrono.
pub fn timestamp_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

// rkyv-normalized wire protocol only.
pub const WIRE_PROTOCOL: &str = "rkyv";

pub mod generated {
    #[allow(dead_code)]
    pub mod codeanalyzer;
}
