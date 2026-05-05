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

// Custom binary protocol - no protobuf, no external dependencies
pub mod generated {
    #[allow(dead_code)]
    pub mod codeanalyzer;
}
