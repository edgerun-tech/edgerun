//! Web server for the agent UI.

use crate::agent::Agent;
use crate::client::{ChatRequest, StreamChunk, TabbyClient};
use crate::vfs::SharedVFS;
use edgerun_http::{Handler, Request, Response, StatusCode};
use edgerun_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

pub struct WebServer {
    agent: Arc<Agent>,
    static_dir: PathBuf,
    tabby_url: String,
    model: String,
}

impl WebServer {
    pub fn new(static_dir: &str, project_root: &str, tabby_url: &str, model: &str) -> Self {
        Self {
            agent: Arc::new(Agent::new(tabby_url, model, project_root)),
            static_dir: PathBuf::from(static_dir),
            tabby_url: tabby_url.to_string(),
            model: model.to_string(),
        }
    }

    pub fn with_vfs(mut self, vfs: SharedVFS) -> Self {
        self.agent = Arc::new(
            Arc::try_unwrap(self.agent)
                .unwrap_or_else(|arc| (*arc).clone())
                .with_vfs(vfs)
        );
        self
    }

    pub fn into_handler(self) -> impl Handler {
        AgentHandler {
            agent: self.agent,
            static_dir: self.static_dir,
            tabby_url: self.tabby_url,
            model: self.model,
        }
    }
}

struct AgentHandler {
    agent: Arc<Agent>,
    static_dir: PathBuf,
    tabby_url: String,
    model: String,
}

impl Clone for AgentHandler {
    fn clone(&self) -> Self {
        Self {
            agent: self.agent.clone(),
            static_dir: self.static_dir.clone(),
            tabby_url: self.tabby_url.clone(),
            model: self.model.clone(),
        }
    }
}

impl Handler for AgentHandler {
    fn handle(&self, req: Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send + '_>> {
        let agent = self.agent.clone();
        let static_dir = self.static_dir.clone();
        let tabby_url = self.tabby_url.clone();
        let model = self.model.clone();

        Box::pin(async move {
            let path = req.uri().request_target().to_string();
            let method = req.method().as_str();

            if method == "GET" && path == "/health" {
                let json = edgerun_json::json!({
                    "status": "ok",
                    "model": model,
                    "tabby_url": tabby_url
                });
                return Response::new(StatusCode::new(200).unwrap())
                    .with_header("Content-Type", "application/json")
                    .with_body(json.to_string().as_bytes().to_vec());
            }

            if method == "POST" && path == "/chat" {
                let body = req.body().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
                if let Ok(payload) = edgerun_json::from_str::<Value>(body) {
                    let message = payload.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();

                    match agent.chat_with_tools(&message).await {
                        Ok(resp) => {
                            let json = edgerun_json::json!({ "reply": resp.reply });
                            return Response::new(StatusCode::new(200).unwrap())
                                .with_header("Content-Type", "application/json")
                                .with_body(json.to_string().as_bytes().to_vec());
                        }
                        Err(e) => {
                            return Response::new(StatusCode::new(500).unwrap())
                                .with_body(e.as_bytes().to_vec());
                        }
                    }
                }
            }

            if method == "POST" && path == "/chat/stream" {
                let body = req.body().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
                if let Ok(payload) = edgerun_json::from_str::<Value>(body) {
                    let message = payload.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default();
                    let context = payload.get("context").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let system_prompt = payload.get("system_prompt").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let max_tokens = payload.get("max_tokens").and_then(|v| v.as_u64()).map(|v| v as u32);
                    let temperature = payload.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32);

                    let client = TabbyClient::new(&self.tabby_url, &self.model);
                    let request = ChatRequest {
                        message,
                        context,
                        system_prompt,
                        max_tokens,
                        temperature,
                    };

                    // Use non-streaming for now until streaming is fixed
                    match client.chat(request).await {
                        Ok(resp) => {
                            let json = edgerun_json::json!({ "reply": resp.reply });
                            return Response::new(StatusCode::new(200).unwrap())
                                .with_header("Content-Type", "application/json")
                                .with_body(json.to_string().as_bytes().to_vec());
                        }
                        Err(e) => {
                            return Response::new(StatusCode::new(500).unwrap())
                                .with_body(e.as_bytes().to_vec());
                        }
                    }
                }
            }

            Self::serve_static(&static_dir, &path).await
        })
    }
}

impl AgentHandler {
    async fn serve_static(static_dir: &PathBuf, path: &str) -> Response {
        let path = if path == "/" { "/index.html" } else { path };

        let mut file_path = static_dir.clone();
        for component in path.trim_start_matches('/').split('/') {
            file_path = file_path.join(component);
        }

        if file_path.exists() && file_path.is_file() {
            match std::fs::read(&file_path) {
                Ok(data) => {
                    let content_type = if path.ends_with(".html") {
                        "text/html"
                    } else if path.ends_with(".js") {
                        "application/javascript"
                    } else if path.ends_with(".css") {
                        "text/css"
                    } else if path.ends_with(".json") {
                        "application/json"
                    } else {
                        "text/plain"
                    };
                    Response::new(StatusCode::new(200).unwrap())
                        .with_header("Content-Type", content_type)
                        .with_body(data)
                }
                Err(_) => Response::not_found(),
            }
        } else {
            Response::not_found()
        }
    }
}
// Benchmark comment