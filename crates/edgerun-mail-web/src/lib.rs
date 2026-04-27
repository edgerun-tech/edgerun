//! Webmail frontend for edgerun email server.
//!
//! Provides a simple web interface for accessing email via IMAP.
//! Serves static HTML/CSS/JS and handles API requests.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::future::Future;
use core::marker::Send;
use core::module_path;
use core::pin::Pin;
use edgerun_http::{Handler, Request, Response, StatusCode};

#[cfg(target_os = "none")]
use edgerun_http::{io, path::PathBuf};

#[cfg(not(target_os = "none"))]
use std::{io, path::PathBuf};

pub struct MailWebHandler {
    #[cfg(not(target_os = "none"))]
    static_handler: edgerun_http::StaticHandler,
    #[cfg(target_os = "none")]
    _static_root: PathBuf,
    api_handler: Arc<MailApiHandler>,
}

struct MailApiHandler {
    _imap_host: String,
    _imap_port: u16,
}

impl MailWebHandler {
    pub fn new(static_root: PathBuf, imap_host: String, imap_port: u16) -> Self {
        Self {
            #[cfg(not(target_os = "none"))]
            static_handler: edgerun_http::StaticHandler::new(static_root),
            #[cfg(target_os = "none")]
            _static_root: static_root,
            api_handler: Arc::new(MailApiHandler {
                _imap_host: imap_host,
                _imap_port: imap_port,
            }),
        }
    }
}

impl Handler for MailWebHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        let uri = request.uri().request_target().to_string();

        if uri.starts_with("/api/") {
            let handler = Arc::clone(&self.api_handler);
            Box::pin(async move { handle_api(&handler, &uri[5..]).await })
        } else {
            #[cfg(not(target_os = "none"))]
            {
                self.static_handler.handle(request)
            }
            #[cfg(target_os = "none")]
            {
                Box::pin(async move {
                    let _ = request;
                    Response::new(StatusCode::NOT_FOUND)
                        .with_header("Content-Type", "text/plain")
                        .with_body("static files are unavailable on bare target")
                })
            }
        }
    }
}

async fn handle_api(_handler: &MailApiHandler, path: &str) -> Response {
    match path {
        "login" => Response::new(StatusCode::OK)
            .with_header("Content-Type", "application/json")
            .with_body(r#"{"status": "ok", "message": "Use IMAP authentication"}"#),
        "folders" => Response::new(StatusCode::OK)
            .with_header("Content-Type", "application/json")
            .with_body(r#"{"folders": ["INBOX", "Sent", "Drafts", "Trash"]}"#),
        _ => Response::new(StatusCode::NOT_FOUND)
            .with_header("Content-Type", "application/json")
            .with_body(r#"{"error": "not found"}"#),
    }
}

pub fn start_webmail(
    static_root: PathBuf,
    bind_addr: &str,
    imap_host: String,
    imap_port: u16,
) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
    Box::pin(async move {
        let handler = MailWebHandler::new(static_root, imap_host, imap_port);
        let server = edgerun_http::HttpServer::new(handler)
            .bind(bind_addr)
            .await?;

        edgerun_log::info!("edgerun-mail-web: listening on {}", bind_addr);
        server.serve().await
    })
}
