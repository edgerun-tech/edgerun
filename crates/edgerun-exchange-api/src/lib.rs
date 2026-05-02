//! EdgeRun Exchange API — HTTP service.
//!
//! Uses edgerun-http for the HTTP server (no axum/tokio).
//! Uses edgerun-json for JSON parsing (no serde_json).
//! Public API never exposes provider names.

extern crate alloc;

use core::pin::Pin;
use core::future::Future;

use edgerun_http::{Handler, HttpServer, Request, Response, StatusCode};

pub mod config;
pub mod types;
mod handlers;
mod routes;

pub use config::Config;
pub use types::*;

#[derive(Clone)]
pub struct ExchangeApiHandler;

impl Handler for ExchangeApiHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move {
            routes::route(&request)
        })
    }
}

/// Build a configured handler for the exchange API.
/// Does NOT start a server — the caller is responsible for binding
/// this handler to an actual HTTP transport.
pub fn build_handler() -> ExchangeApiHandler {
    ExchangeApiHandler
}