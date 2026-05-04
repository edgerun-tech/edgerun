// Re-export modules for use by other crates
pub mod analyzer;
pub mod diagnostics;
#[allow(dead_code)]
pub mod edit;
pub mod filesystem;
pub mod git;
pub mod parser;
pub mod repo_registry;
pub mod tools;
pub mod uir;

// Custom binary protocol - no protobuf, no external dependencies
#[allow(dead_code)]
pub mod generated {
    pub mod codeanalyzer;
}
