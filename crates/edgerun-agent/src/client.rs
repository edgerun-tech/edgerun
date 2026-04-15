//! TabbyAPI client for code generation, debugging, and testing.

use edgerun_http::{HttpClient, Request, StatusCode};
use edgerun_json::{from_str, to_string};
use edgerun_rt::{AsyncRead, AsyncReadExt, AsyncWriteExt, TcpSocket};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub struct TabbyClient {
    base_url: String,
    model: String,
    client: HttpClient,
    max_tokens: Option<u32>,
}

impl std::fmt::Debug for TabbyClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabbyClient")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}

impl Clone for TabbyClient {
    fn clone(&self) -> Self {
        Self {
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            client: HttpClient::new(),
            max_tokens: self.max_tokens,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CompletionRequest {
    prompt: String,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct CompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct Choice {
    text: String,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: Option<u32>,
    #[serde(default)]
    pub completion_tokens: Option<u32>,
    #[serde(default)]
    pub total_tokens: Option<u32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ChatResponse {
    pub reply: String,
    pub usage: Option<Usage>,
    pub error: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct StreamChunk {
    pub text: String,
    pub done: bool,
}

impl TabbyClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        let client = HttpClient::new()
            .with_read_timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS));

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client,
            max_tokens: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub async fn complete(&self, prompt: &str) -> Result<ChatResponse, String> {
        let request = CompletionRequest {
            prompt: prompt.to_string(),
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            temperature: None,
            stream: Some(false),
        };
        self.send_completion(request).await
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let req = self.build_completion_request(request, false);
        self.send_completion(req).await
    }

    fn build_completion_request(&self, request: ChatRequest, stream: bool) -> CompletionRequest {
        let system = request.system_prompt.unwrap_or_else(|| {
            "You are an expert coding assistant. Generate clean, idiomatic Rust code.".to_string()
        });

        let prompt = match request.context {
            Some(ctx) => format!("{}\n\nContext:\n{}\n\nUser: {}", system, ctx, request.message),
            None => format!("{}\n\nUser: {}", system, request.message),
        };

        CompletionRequest {
            prompt,
            model: self.model.clone(),
            max_tokens: request.max_tokens.or(self.max_tokens),
            temperature: request.temperature,
            stream: Some(stream),
        }
    }

    async fn send_completion(&self, request: CompletionRequest) -> Result<ChatResponse, String> {
        let json_body = to_string(&request).map_err(|e| e.to_string())?;

        let response = self
            .client
            .post_json(&format!("{}/v1/completions", self.base_url), &json_body)
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = String::from_utf8_lossy(response.body()).to_string();
            return Err(format!("API error {}: {}", status.as_u16(), body));
        }

        let body_str = String::from_utf8_lossy(response.body()).to_string();
        let result: CompletionResponse = from_str(&body_str)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let reply = result
            .choices
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            reply: reply.trim().to_string(),
            usage: result.usage,
            error: None,
        })
    }

    pub async fn chat_streaming<F>(&self, request: ChatRequest, mut on_chunk: F) -> Result<ChatResponse, String>
    where
        F: FnMut(StreamChunk) -> bool + Send,
    {
        let req = self.build_completion_request(request, true);
        let json_body = to_string(&req).map_err(|e| e.to_string())?;

        let host = self.base_url.trim_start_matches("http://");
        let (host, port) = if let Some((h, p)) = host.rsplit_once(':') {
            (h.to_string(), p.parse().unwrap_or(5001))
        } else {
            (host.to_string(), 5001)
        };

        let addr: SocketAddr = format!("{}:{}", host, port).parse()
            .map_err(|e| format!("Invalid address: {}", e))?;
        let socket = TcpSocket::new_v4()
            .map_err(|e| format!("Failed to create socket: {}", e))?;
        let mut stream = socket.connect(addr)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        let path = format!("/v1/completions");
        let request_line = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: text/event-stream\r\nConnection: keep-alive\r\nContent-Length: {}\r\n\r\n",
            path,
            host,
            json_body.len()
        );

        stream.write_all(request_line.as_bytes()).await
            .map_err(|e| format!("Write failed: {}", e))?;
        stream.write_all(json_body.as_bytes()).await
            .map_err(|e| format!("Write failed: {}", e))?;
        stream.flush().await
            .map_err(|e| format!("Flush failed: {}", e))?;

        let mut reader = edgerun_rt::BufReader::new(stream);
        let mut full_reply = String::new();
        let mut usage: Option<Usage> = None;
        let mut in_body = false;

        loop {
            let line_result = reader.read_line().await;
            let line = match line_result {
                Ok(Some(l)) => l,
                Ok(None) => break,
                Err(_) => break,
            };

            let line = line.trim();
            if line.is_empty() {
                in_body = true;
                continue;
            }

            if !in_body {
                if line.starts_with("HTTP/") {
                    if !line.contains("200 ") {
                        return Err(format!("HTTP error: {}", line));
                    }
                }
                if line.starts_with("content-length:") || line.starts_with("transfer-encoding:") {
                    continue;
                }
                if line.is_empty() {
                    in_body = true;
                }
                continue;
            }

            if line.starts_with("data:") {
                let data = line[5..].trim();
                if data == "[DONE]" {
                    let done = on_chunk(StreamChunk {
                        text: String::new(),
                        done: true,
                    });
                    if done {
                        break;
                    }
                    continue;
                }

                if let Ok(resp) = from_str::<CompletionResponse>(data) {
                    if let Some(choice) = resp.choices.first() {
                        let new_text = choice.text.clone();
                        if !new_text.is_empty() {
                            let delta = new_text.trim().to_string();
                            full_reply.push_str(&delta);
                            let should_continue = on_chunk(StreamChunk {
                                text: delta,
                                done: false,
                            });
                            if !should_continue {
                                break;
                            }
                        }
                        usage = resp.usage;
                    }
                }
            }
        }

        Ok(ChatResponse {
            reply: full_reply.trim().to_string(),
            usage,
            error: None,
        })
    }
}

impl Default for TabbyClient {
    fn default() -> Self {
        Self::new("http://10.10.10.1:5001", "Devstral-Small-2-24B-Instruct-2512-exl3-4.5bpw-optimized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let client = TabbyClient::default();
        assert_eq!(client.base_url, "http://10.10.10.1:5001");
        assert_eq!(client.model, "Devstral-Small-2-24B-Instruct-2512-exl3-4.5bpw-optimized");
    }
}
// Benchmark comment