//! AI-powered coding agent with shell tool access, optimized for 32K context models.

pub mod agent;
pub mod analyzer;
pub mod client;
pub mod context;
pub mod prompts;
pub mod tools;
pub mod web;

pub use agent::Agent;
pub use client::{ChatRequest, ChatResponse, TabbyClient};
pub use context::truncate_str;
pub use tools::ToolExecutor;
