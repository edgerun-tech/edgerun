//! TabbyAPI client with retry logic.

use edgerun_json::{from_str, to_string};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MAX_RETRIES: u32 = 2;
const RETRY_DELAY_MS: u64 = 1000;

#[derive(Clone)]
pub struct TabbyClient {
    base_url: String,
    model: String,
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

#[derive(Debug, serde::Serialize)]
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
    #[allow(dead_code)]
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

impl TabbyClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            max_tokens: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    fn build_completion_request(&self, request: ChatRequest, _stream: bool) -> CompletionRequest {
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
            stream: Some(false),
        }
    }

    fn parse_url(&self) -> Result<(String, u16), String> {
        let url = &self.base_url;
        let url = url.trim_start_matches("http://").trim_start_matches("https://");
        
        if let Some(colon) = url.find(':') {
            let host = &url[..colon];
            let port: u16 = url[colon+1..].parse().map_err(|_| "Invalid port")?;
            Ok((host.to_string(), port))
        } else {
            Ok((url.to_string(), 80))
        }
    }

    fn http_request(&self, path: &str, body: &str) -> Result<String, String> {
        let (host, port) = self.parse_url()?;
        
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr).map_err(|e| format!("Connection failed: {}: {}", addr, e))?;
        
        stream.set_read_timeout(Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS))).ok();

        let request = format!(
            "POST {} HTTP/1.1\r\n\
            Host: {}:{}\r\n\
            Content-Type: application/json\r\n\
            Content-Length: {}\r\n\
            Connection: close\r\n\
            \r\n\
            {}",
            path, host, port,
            body.len(),
            body
        );

        stream.write_all(request.as_bytes()).map_err(|e| format!("Write failed: {}", e))?;
        
        let mut response = Vec::new();
        let mut buf = [0u8; 8192];
        let deadline = Instant::now() + Duration::from_secs(DEFAULT_TIMEOUT_SECS);
        
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            stream.set_read_timeout(Some(remaining)).ok();
            
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => response.extend_from_slice(&buf[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        break;
                    }
                    continue;
                }
                Ok(_) | Err(_) => break,
            }
        }

        let response = String::from_utf8_lossy(&response);
        
        // Find body start (after headers)
        if let Some(pos) = response.find("\r\n\r\n") {
            let headers = &response[..pos];
            let body = &response[pos + 4..];
            
            // Check for chunked encoding
            if headers.to_lowercase().contains("transfer-encoding: chunked") {
                return Err("Chunked encoding not supported".to_string());
            }
            
            Ok(body.to_string())
        } else {
            Err("Invalid response".to_string())
        }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let req = self.build_completion_request(request, false);
        let json_body = to_string(&req).map_err(|e| format!("Serialize error: {}", e))?;

        let mut last_err = String::new();

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                edgerun_rt::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }

            match self.http_request("/v1/completions", &json_body) {
                Ok(body) => {
                    // Check for HTTP error in body
                    if let Ok(val) = from_str::<edgerun_json::Value>(&body) {
                        if let Some(detail) = val.get("detail") {
                            let status = detail.get("loc")
                                .and_then(|l| l.as_array())
                                .and_then(|a| a.first())
                                .and_then(|v| v.as_str());
                            
                            if status == Some("body") || status == Some("body") {
                                return Err(format!("API validation error: {}", body));
                            }
                        }
                    }

                    // Try to parse as JSON
                    let result: CompletionResponse = match from_str(&body) {
                        Ok(r) => r,
                        Err(e) => {
                            // Check if it's an HTTP error response
                            if body.starts_with("<!") || body.starts_with("<html") {
                                return Err(format!("HTTP error: {}", body));
                            }
                            return Err(format!("Failed to parse response: {} - body: {}", e, &body[..body.len().min(500)]));
                        }
                    };

                    let reply = result
                        .choices
                        .first()
                        .map(|c| c.text.clone())
                        .unwrap_or_default();

                    return Ok(ChatResponse {
                        reply: reply.trim().to_string(),
                        usage: result.usage,
                        error: None,
                    });
                }
                Err(e) => {
                    last_err = e;
                    continue;
                }
            }
        }

        Err(format!("All {} retries failed: {}", MAX_RETRIES, last_err))
    }

    pub async fn health_check(&self) -> Result<String, String> {
        match self.http_request("/v1/models", "") {
            Ok(_) => Ok("ok".to_string()),
            Err(e) => Err(format!("Health check failed: {}", e)),
        }
    }
}

impl Default for TabbyClient {
    fn default() -> Self {
        Self::new("http://10.10.10.1:5001", "devstral-small-2:24b")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let client = TabbyClient::default();
        assert_eq!(client.base_url, "http://10.10.10.1:5001");
        assert_eq!(client.model, "devstral-small-2:24b");
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            message: "hello".to_string(),
            context: Some("context".to_string()),
            system_prompt: None,
            max_tokens: Some(100),
            temperature: Some(0.5),
        };
        let json = to_string(&req).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("context"));
    }
}
