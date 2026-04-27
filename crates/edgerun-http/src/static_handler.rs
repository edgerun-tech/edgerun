//! Static file serving for HTTP servers.

use crate::runtime::path::{Path, PathBuf};

use crate::{Handler, Request, Response, StatusCode};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use core::future::Future;
use core::pin::Pin;

pub struct StaticHandler {
    root: PathBuf,
    default_file: String,
}

impl StaticHandler {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            default_file: "index.html".to_string(),
        }
    }

    pub fn with_default_file(mut self, file: impl Into<String>) -> Self {
        self.default_file = file.into();
        self
    }

    fn map_path(&self, uri: &str) -> Option<PathBuf> {
        let path = if uri.is_empty() || uri == "/" {
            self.default_file.clone()
        } else {
            uri.trim_start_matches('/').to_string()
        };

        let full_path = self.root.join(&path);
        if full_path.is_file() {
            Some(full_path)
        } else if full_path.is_dir() {
            let index_path = full_path.join(&self.default_file);
            if index_path.is_file() {
                Some(index_path)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn guess_mime_type(path: &Path) -> &'static str {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "html" | "htm" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "xml" => "application/xml; charset=utf-8",
            "txt" | "text" => "text/plain; charset=utf-8",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "ico" => "image/x-icon",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "eot" => "application/vnd.ms-fontobject",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            _ => "application/octet-stream",
        }
    }
}

impl Handler for StaticHandler {
    fn handle(
        &self,
        request: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        let uri = request.uri().request_target().to_string();
        let root = self.root.clone();

        Box::pin(async move {
            let handler = Self {
                root,
                default_file: self.default_file.clone(),
            };
            let path = match handler.map_path(&uri) {
                Some(p) => p,
                None => {
                    return Response::new(StatusCode::NOT_FOUND).with_body("404 Not Found");
                }
            };

            match crate::runtime::fs::read(&path) {
                Ok(content) => {
                    let mime = StaticHandler::guess_mime_type(&path);
                    Response::new(StatusCode::OK)
                        .with_header("Content-Type", mime)
                        .with_body(content)
                }
                Err(e) => Response::new(StatusCode::INTERNAL_SERVER_ERROR)
                    .with_body(format!("500 Internal Server Error: {}", e)),
            }
        })
    }
}

pub fn serve_static(root: impl Into<PathBuf>) -> StaticHandler {
    StaticHandler::new(root)
}
