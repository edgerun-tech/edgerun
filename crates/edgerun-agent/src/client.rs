//! TabbyAPI client with retry logic.

use edgerun_http::HttpClient;
use edgerun_json::{from_str, to_string};
use std::time::Duration;

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

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let req = self.build_completion_request(request, false);
        let json_body = to_string(&req).map_err(|e| format!("Serialize error: {}", e))?;

        let client = HttpClient::new()
            .with_connect_timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .with_read_timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS));

        let mut last_err = String::new();

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                edgerun_rt::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }

            match client
                .post_json(&format!("{}/v1/completions", self.base_url), &json_body)
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let body = String::from_utf8_lossy(response.body()).to_string();
                        last_err = format!("API error {}: {}", status.as_u16(), body);
                        if status.as_u16() >= 500 {
                            continue;
                        }
                        return Err(last_err);
                    }

                    let body_str = String::from_utf8_lossy(response.body()).to_string();
                    let result: CompletionResponse = match from_str(&body_str) {
                        Ok(r) => r,
                        Err(e) => return Err(format!("Failed to parse response: {}", e)),
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
                    last_err = format!("Request failed: {}", e);
                    continue;
                }
            }
        }

        Err(format!("All {} retries failed: {}", MAX_RETRIES, last_err))
    }

    pub async fn health_check(&self) -> Result<String, String> {
        let client = HttpClient::new()
            .with_connect_timeout(Duration::from_secs(5))
            .with_read_timeout(Duration::from_secs(5));

        match client.get(&format!("{}/v1/models", self.base_url)).await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok("ok".to_string())
                } else {
                    Err(format!(
                        "Health check failed: status {}",
                        response.status().as_u16()
                    ))
                }
            }
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
