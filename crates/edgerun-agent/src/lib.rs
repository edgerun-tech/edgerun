//! AI-powered coding agent with hardware-optimized acceleration.
//!
//! This crate provides AI-powered code generation, debugging, and testing
//! capabilities by integrating with:
//! - TabbyAPI server (24B model on dGPU)
//! - Local embedding models (NPU/iGPU acceleration)
//! - AST-level code editing (syn + tree-sitter)
//! - Semantic search and RAG
//!
//! # Hardware Utilization
//!
//! - **dGPU**: 24B dense model for complex reasoning
//! - **iGPU**: Small model (1-3B) for fast inference
//! - **NPU**: Embedding generation and semantic search
//! - **CPU**: Tool execution, AST parsing, orchestration
//!
//! # Features
//!
//! - Dual-model architecture (small + large LM)
//! - AST-safe code editing
//! - Multi-language code analysis (tree-sitter)
//! - Semantic search with NPU acceleration
//! - Context caching and RAG
//! - Intelligent tool routing
//!
//! # Example
//!
//! ```no_run
//! use edgerun_agent::{Agent, TabbyClient};
//!
//! fn main() {
//!     let rt = edgerun_rt::Runtime::new_multi_thread().enable_all().build().unwrap();
//!     rt.block_on(async {
//!         let agent = Agent::new("http://localhost:5001", "devstral-small-2:24b", ".");
//!         let response = agent.chat("Write a hello world function").await.unwrap();
//!         println!("{}", response.reply);
//!     });
//! }
//! ```

pub mod agent;
pub mod analyzer;
pub mod cache;
pub mod client;
pub mod context;
pub mod embeddings;
pub mod prompts;
pub mod tools;
pub mod vfs;
pub mod web;

pub use agent::Agent;
pub use analyzer::{analyze_directory, graph_to_json, ProgramGraph};
pub use cache::{CacheStats, SemanticCache};
pub use client::{ChatRequest, ChatResponse, TabbyClient};
pub use embeddings::{EmbeddingModel, SemanticSearch};
pub use vfs::{SharedVFS, VirtualFileSystem};
// Benchmark comment