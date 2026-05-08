// Re-export modules for use by other crates
#[cfg(not(target_arch = "wasm32"))]
pub mod analyzer;
#[cfg(not(target_arch = "wasm32"))]
pub mod codealyzer;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub mod diagnostics;
#[allow(dead_code)]
pub mod edit;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub mod filesystem;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub mod git;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub mod mcp_permission;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub mod mcp_rust_ast;
#[allow(dead_code)]
pub mod parser;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub mod rust_edit;
#[cfg(not(target_arch = "wasm32"))]
pub mod tools;
#[allow(dead_code)]
pub mod uir;

pub mod source_analysis;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

/// Returns a current timestamp string (Unix seconds) without chrono.
pub fn timestamp_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

// rkyv-normalized wire protocol only.
pub const WIRE_PROTOCOL: &str = "rkyv";

pub mod xray_wire;

pub mod generated {
    #[allow(dead_code)]
    pub mod codeanalyzer;
}
