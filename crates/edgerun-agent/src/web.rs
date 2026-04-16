//! Web server for the agent API.

use crate::agent::Agent;
use crate::client::{ChatRequest, ChatResponse, TabbyClient};
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
        }
    }

    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.agent = Arc::new(
            Arc::try_unwrap(self.agent)
                .unwrap_or_else(|arc| (*arc).clone())
                .with_allowed_commands(commands),
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

fn json_response(status: StatusCode, body: &str) -> Response {
    Response::new(status)
        .with_header("Content-Type", "application/json")
        .with_header("Access-Control-Allow-Origin", "*")
        .with_body(body.as_bytes().to_vec())
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

            if method == "POST" && path == "/chat" {
                let body = req.body().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
                if let Ok(payload) = edgerun_json::from_str::<Value>(body) {
                    let message = payload
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    let use_tools = payload
                        .get("use_tools")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);

                    let result = if use_tools {
                        agent.chat_with_tools(&message).await
                    } else {
                        agent.chat(&message).await
                    };

                    match result {
                        Ok(resp) => {
                            let json = edgerun_json::json!({
                                "reply": resp.reply,
                                "error": resp.error.unwrap_or_default(),
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
            } else if method == "POST" && path == "/chat" {
                unreachable!()
            } else {
                Self::serve_static(&static_dir, &path).await
            }
        })
    }
}

impl AgentHandler {
    async fn serve_static(static_dir: &PathBuf, path: &str) -> Response {
        let path = if path == "/" { "/index.html" } else { path };

        let mut file_path = static_dir.clone();
        for component in path.trim_start_matches('/').split('/') {
            if component == ".." {
                return Response::not_found();
            }
            file_path = file_path.join(component);
        }

        if file_path.exists() && file_path.is_file() && file_path.starts_with(static_dir) {
            match std::fs::read(&file_path) {
                Ok(data) => {
                    let content_type = match file_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                    {
                        "html" => "text/html",
                        "js" => "application/javascript",
                        "css" => "text/css",
                        "json" => "application/json",
                        "png" => "image/png",
                        "svg" => "image/svg+xml",
                        "ico" => "image/x-icon",
                        _ => "text/plain",
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