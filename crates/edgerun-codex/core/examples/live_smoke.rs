use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use codex_core::Prompt;
use codex_core::Provider;
use codex_core::TurnRequest;
use codex_core::api::AuthProvider;
use codex_core::api::RetryConfig;
use codex_core::protocol::models::ContentItem;
use codex_core::protocol::models::ResponseItem;
use edgerun_http::HeaderMap;
use edgerun_http::HeaderValue;
use edgerun_http::header::AUTHORIZATION;
use edgerun_json::Value;

#[derive(Debug)]
struct ChatGptAuth {
    access_token: String,
    account_id: Option<String>,
}

impl AuthProvider for ChatGptAuth {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let bearer = format!("Bearer {}", self.access_token);
        if let Ok(value) = HeaderValue::from_str(&bearer) {
            headers.insert(AUTHORIZATION, value);
        }

        if let Some(account_id) = self.account_id.as_deref()
            && let Ok(value) = HeaderValue::from_str(account_id)
        {
            headers.insert("ChatGPT-Account-ID", value);
        }
    }
}

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .expect("CODEX_HOME or HOME must be set")
}

fn read_chatgpt_auth() -> Result<Arc<ChatGptAuth>, Box<dyn Error>> {
    let auth_path = codex_home().join("auth.json");
    let auth: Value = edgerun_json::from_serde_slice(&std::fs::read(&auth_path)?)?;
    let access_token = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| format!("missing tokens.access_token in {}", auth_path.display()))?
        .to_string();
    let account_id = auth
        .get("tokens")
        .and_then(|tokens| tokens.get("account_id"))
        .and_then(Value::as_str)
        .filter(|account_id| !account_id.is_empty())
        .map(ToString::to_string);

    Ok(Arc::new(ChatGptAuth {
        access_token,
        account_id,
    }))
}

fn provider() -> Provider {
    let mut headers = HeaderMap::new();
    headers.insert(
        "version",
        HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
    );

    Provider {
        name: "OpenAI".to_string(),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        query_params: None::<HashMap<String, String>>,
        headers,
        retry: RetryConfig {
            max_attempts: 1,
            base_delay: Duration::from_millis(200),
            retry_429: false,
            retry_5xx: false,
            retry_transport: false,
        },
        stream_idle_timeout: Duration::from_secs(60),
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let model = std::env::var("CODEX_SMOKE_MODEL").unwrap_or_else(|_| "gpt-5.5".to_string());
    let auth = read_chatgpt_auth()?;
    let client = codex_core::ModelClient::new_native(model, provider(), auth);
    let started = Instant::now();

    let output = client
        .collect_turn(TurnRequest {
            prompt: Prompt {
                input: vec![ResponseItem::Message {
                    id: None,
                    role: "user".to_string(),
                    content: vec![ContentItem::InputText {
                        text: "Reply with exactly: edgerun-codex live smoke ok".to_string(),
                    }],
                    phase: None,
                }],
                ..Prompt::default()
            },
            store: false,
            ..TurnRequest::default()
        })
        .await?;

    println!("model: {}", client.model());
    println!("elapsed_ms: {}", started.elapsed().as_millis());
    println!(
        "response_id: {}",
        output.response_id.as_deref().unwrap_or("<none>")
    );
    println!("end_turn: {:?}", output.end_turn);
    println!("output: {}", output.output_text.trim());
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    edgerun_tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}
