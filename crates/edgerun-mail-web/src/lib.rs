//! Webmail frontend for edgerun email server.
//!
//! Provides a simple web interface for accessing email via IMAP.
//! Serves static HTML/CSS/JS and handles API requests.

use edgerun_http::{Handler, Request, Response, StatusCode};
use std::sync::Arc;

pub struct MailWebHandler {
    static_handler: edgerun_http::StaticHandler,
    api_handler: Arc<MailApiHandler>,
}

struct MailApiHandler {
    _imap_host: String,
    _imap_port: u16,
}

impl MailWebHandler {
    pub fn new(static_root: std::path::PathBuf, imap_host: String, imap_port: u16) -> Self {
        Self {
            static_handler: edgerun_http::StaticHandler::new(static_root),
            api_handler: Arc::new(MailApiHandler { _imap_host: imap_host, _imap_port: imap_port }),
        }
    }
}

impl Handler for MailWebHandler {
    fn handle(
        &self,
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        let uri = request.uri().request_target().to_string();

        if uri.starts_with("/api/") {
            let handler = Arc::clone(&self.api_handler);
            Box::pin(async move {
                handle_api(&handler, &uri[5..]).await
            })
        } else {
            self.static_handler.handle(request)
        }
    }
}

async fn handle_api(_handler: &MailApiHandler, path: &str) -> Response {
    match path {
        "login" => {
            Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/json")
                .with_body(r#"{"status": "ok", "message": "Use IMAP authentication"}"#)
        }
        "folders" => {
            Response::new(StatusCode::OK)
                .with_header("Content-Type", "application/json")
                .with_body(r#"{"folders": ["INBOX", "Sent", "Drafts", "Trash"]}"#)
        }
        _ => {
            Response::new(StatusCode::NOT_FOUND)
                .with_header("Content-Type", "application/json")
                .with_body(r#"{"error": "not found"}"#)
        }
    }
}

pub fn start_webmail(
    static_root: std::path::PathBuf,
    bind_addr: &str,
    imap_host: String,
    imap_port: u16,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), std::io::Error>> + Send + '_>> {
    Box::pin(async move {
        let handler = MailWebHandler::new(static_root, imap_host, imap_port);
        let server = edgerun_http::HttpServer::new(handler)
            .bind(bind_addr)
            .await?;

        edgerun_log::info!("edgerun-mail-web: listening on {}", bind_addr);
        server.serve().await
    })
}