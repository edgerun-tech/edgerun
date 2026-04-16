//! Web server for the agent API.

use crate::agent::Agent;
use edgerun_http::{Handler, Request, Response, StatusCode};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct WebServer {
    agent: Arc<Agent>,
    static_dir: PathBuf,
    tabby_url: String,
    model: String,
    project_root: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl WebServer {
    pub fn new(
        static_dir: &str,
        project_root: &str,
        tabby_url: &str,
        model: &str,
    ) -> Self {
        Self {
            agent: Arc::new(Agent::new(tabby_url, model, project_root)),
            static_dir: PathBuf::from(static_dir),
            tabby_url: tabby_url.to_string(),
            model: model.to_string(),
            project_root: project_root.to_string(),
            temperature: None,
            max_tokens: None,
        }
    }

    pub fn with_allowed_commands(self, commands: Vec<String>) -> Self {
        let agent = (*self.agent).clone()
            .with_allowed_commands(commands)
            .with_temperature(self.temperature.unwrap_or(0.7))
            .with_max_tokens(self.max_tokens.unwrap_or(2048));
        Self {
            agent: Arc::new(agent),
            static_dir: self.static_dir,
            tabby_url: self.tabby_url,
            model: self.model,
            project_root: self.project_root,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }

    pub fn with_temperature(self, temperature: Option<f32>) -> Self {
        Self {
            temperature,
            ..self
        }
    }

    pub fn with_max_tokens(self, max_tokens: Option<u32>) -> Self {
        Self {
            max_tokens,
            ..self
        }
    }

    pub fn into_handler(self) -> impl Handler {
        AgentHandler {
            agent: self.agent,
            static_dir: self.static_dir,
            tabby_url: self.tabby_url,
            model: self.model,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

struct AgentHandler {
    agent: Arc<Agent>,
    static_dir: PathBuf,
    tabby_url: String,
    model: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

impl Clone for AgentHandler {
    fn clone(&self) -> Self {
        Self {
            agent: self.agent.clone(),
            static_dir: self.static_dir.clone(),
            tabby_url: self.tabby_url.clone(),
            model: self.model.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

fn json_response(status: StatusCode, body: &str) -> Response {
    Response::new(status)
        .with_header("Content-Type", "application/json")
        .with_header("Access-Control-Allow-Origin", "*")
        .with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .with_header("Access-Control-Allow-Headers", "Content-Type")
        .with_body(body.as_bytes().to_vec())
}

fn cors_preflight() -> Response {
    Response::new(StatusCode::new(204).unwrap())
        .with_header("Access-Control-Allow-Origin", "*")
        .with_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .with_header("Access-Control-Allow-Headers", "Content-Type")
        .with_header("Access-Control-Max-Age", "86400")
}

impl Handler for AgentHandler {
    fn handle(
        &self,
        req: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        let agent = self.agent.clone();
        let static_dir = self.static_dir.clone();
        let tabby_url = self.tabby_url.clone();
        let model = self.model.clone();

        Box::pin(async move {
            let path = req.uri().request_target().to_string();
            let method = req.method().as_str();

            if method == "OPTIONS" {
                return cors_preflight();
            }

            if method == "GET" && path == "/health" {
                let json = edgerun_json::json!({
                    "status": "ok",
                    "model": model,
                    "tabby_url": tabby_url,
                    "allowed_commands": agent.get_allowed_commands(),
                });
                return json_response(StatusCode::new(200).unwrap(), &json.to_string());
            }

            if method == "GET" && path == "/commands" {
                let json = edgerun_json::json!({
                    "commands": agent.get_allowed_commands(),
                });
                return json_response(StatusCode::new(200).unwrap(), &json.to_string());
            }

            if method == "POST" && path == "/clear" {
                agent.clear_history();
                let json = edgerun_json::json!({"status": "cleared"});
                return json_response(StatusCode::new(200).unwrap(), &json.to_string());
            }

            if method == "POST" && path == "/chat" {
                let body = req.body().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
                if let Ok(payload) = edgerun_json::from_str::<edgerun_json::Value>(body) {
                    let message = payload
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    if message.is_empty() {
                        let json = edgerun_json::json!({"error": "message is required"});
                        return json_response(StatusCode::new(400).unwrap(), &json.to_string());
                    }

                    let result = agent.chat_with_tools(&message).await;

                    match result {
                        Ok(resp) => {
                            let error_str = resp.error.unwrap_or_default();
                            let json = edgerun_json::json!({
                                "reply": resp.reply,
                                "error": error_str,
                            });
                            json_response(StatusCode::new(200).unwrap(), &json.to_string())
                        }
                        Err(e) => {
                            let json = edgerun_json::json!({"error": e});
                            json_response(StatusCode::new(500).unwrap(), &json.to_string())
                        }
                    }
                } else {
                    let json = edgerun_json::json!({"error": "Invalid JSON body"});
                    json_response(StatusCode::new(400).unwrap(), &json.to_string())
                }
            } else {
                serve_static(&static_dir, &path).await
            }
        })
    }
}

async fn serve_static(static_dir: &Path, path: &str) -> Response {
    let path = if path == "/" { "/index.html" } else { path };

    let mut file_path = static_dir.to_path_buf();
    let decoded = match url_decode(path) {
        Ok(d) => d,
        Err(_) => return Response::not_found(),
    };

    for component in decoded.trim_start_matches('/').split('/') {
        if component == ".." || component == "." || component.is_empty() {
            if component == ".." {
                return Response::not_found();
            }
            continue;
        }
        file_path = file_path.join(component);
    }

    if !file_path.starts_with(static_dir) {
        return Response::not_found();
    }

    if file_path.exists() && file_path.is_file() {
        match std::fs::read(&file_path) {
            Ok(data) => {
                let content_type = match file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                {
                    "html" => "text/html; charset=utf-8",
                    "js" | "mjs" => "application/javascript",
                    "css" => "text/css; charset=utf-8",
                    "json" => "application/json",
                    "png" => "image/png",
                    "svg" => "image/svg+xml",
                    "ico" => "image/x-icon",
                    "woff" | "woff2" => "font/woff2",
                    "wasm" => "application/wasm",
                    _ => "application/octet-stream",
                };
                Response::new(StatusCode::new(200).unwrap())
                    .with_header("Content-Type", content_type)
                    .with_header("Access-Control-Allow-Origin", "*")
                    .with_body(data)
            }
            Err(_) => Response::not_found(),
        }
    } else {
        Response::not_found()
    }
}

fn url_decode(s: &str) -> Result<String, std::str::Utf8Error> {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).map_err(|e| e.utf8_error())
}
