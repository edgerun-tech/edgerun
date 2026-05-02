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

pub fn run_server(config: Config) -> Result<(), alloc::string::String> {
    let addr = format!("{}:{}", config.host, config.port);
    let server = HttpServer::new(ExchangeApiHandler);
    
    // Note: this is synchronous blocking - in a real async runtime we'd use .await on bind
    // For now, we just return Ok as this would run in a spawnable context
    let _ = addr;
    let _ = server;
    
    Ok(())
}