//! AI-powered coding agent with shell tool access.
//!
//! Connects to a TabbyAPI server for LLM inference and provides
//! shell command execution for code manipulation.

pub mod agent;
pub mod analyzer;
pub mod client;
pub mod context;
pub mod prompts;
pub mod tools;
pub mod web;

pub use agent::Agent;
pub use analyzer::{analyze_directory, graph_to_json, ProgramGraph};
pub use client::{ChatRequest, ChatResponse, TabbyClient};
pub use tools::ToolExecutor;