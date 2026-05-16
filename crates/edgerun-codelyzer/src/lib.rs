#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;

// Re-export modules for use by other crates
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub mod analyzer;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub mod codealyzer;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub mod dead_code;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[allow(dead_code)]
pub mod diagnostics;
#[cfg(feature = "std")]
#[allow(dead_code)]
pub mod edit;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[allow(dead_code)]
pub mod filesystem;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[allow(dead_code)]
pub mod git;
pub mod glob;
#[cfg(feature = "std")]
pub mod graph_types;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[allow(dead_code)]
pub mod mcp_permission;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[allow(dead_code)]
pub mod mcp_rust_ast;
#[cfg(feature = "std")]
#[allow(dead_code)]
pub mod parser;
#[cfg(feature = "std")]
pub mod regex;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
#[allow(dead_code)]
pub mod rust_edit;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub mod tools;
#[cfg(feature = "std")]
#[allow(dead_code)]
pub mod uir;

#[cfg(feature = "std")]
pub mod source_analysis;
#[cfg(feature = "vfs")]
#[allow(dead_code, unused_imports, unused_variables)]
pub use edgerun_vfs as vfs;
#[cfg(all(feature = "std", target_arch = "wasm32", feature = "wire-rkyv"))]
pub mod wasm;

/// Returns a current timestamp string (Unix seconds) without chrono.
pub fn timestamp_now() -> String {
    #[cfg(feature = "std")]
    {
        use alloc::string::ToString;
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default()
    }

    #[cfg(not(feature = "std"))]
    {
        alloc::string::String::new()
    }
}

#[cfg(feature = "wire-rkyv")]
pub const WIRE_PROTOCOL: &str = "rkyv";

#[cfg(feature = "wire-rkyv")]
pub mod xray_wire;

#[cfg(feature = "wire-rkyv")]
pub mod generated {
    #[allow(dead_code)]
    pub mod codeanalyzer;
}
