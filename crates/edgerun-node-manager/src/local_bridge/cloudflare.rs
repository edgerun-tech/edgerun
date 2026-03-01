// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Context, Result};
use reqwest::{header::CONTENT_TYPE, Client};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};

pub(crate) fn cloudflare_token_from(raw: &str) -> Result<String> {
    let token = raw.trim();
    if token.len() < 20 {
        return Err(anyhow!("cloudflare account api token is missing or invalid"));
    }
    Ok(token.to_string())
}

pub(crate) async fn cloudflare_api_verify_token(token: &str) -> Result<Value> {
    let client = Client::new();
    let url = "https://api.cloudflare.com/client/v4/user/tokens/verify";
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await
        .context("cloudflare token verify request failed")?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .context("failed to read cloudflare token verify response")?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("cloudflare token verify failed ({status}): {detail}"));
    }
    let payload: Value = sonic_rs::from_slice(&bytes).context("failed to parse cloudflare token verify json")?;
    if payload["success"].as_bool().unwrap_or(false) {
        return Ok(payload);
    }
    let first_error = payload["errors"]
        .as_array()
        .and_then(|errors| errors.first())
        .and_then(|error| error["message"].as_str())
        .unwrap_or("token verify returned unsuccessful response");
    Err(anyhow!("cloudflare token verify rejected token: {first_error}"))
}

pub(crate) async fn cloudflare_api_get_user(token: &str) -> Result<Value> {
    let client = Client::new();
    let url = "https://api.cloudflare.com/client/v4/user";
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await
        .context("cloudflare user request failed")?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .context("failed to read cloudflare user response")?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("cloudflare user request failed ({status}): {detail}"));
    }
    let payload: Value = sonic_rs::from_slice(&bytes).context("failed to parse cloudflare user json")?;
    if payload["success"].as_bool().unwrap_or(false) {
        return Ok(payload);
    }
    let first_error = payload["errors"]
        .as_array()
        .and_then(|errors| errors.first())
        .and_then(|error| error["message"].as_str())
        .unwrap_or("user request returned unsuccessful response");
    Err(anyhow!("cloudflare user lookup rejected token: {first_error}"))
}

fn cloudflare_payload_error(payload: &Value, fallback: &str) -> String {
    payload["errors"]
        .as_array()
        .and_then(|errors| errors.first())
        .and_then(|error| error["message"].as_str())
        .map(|message| message.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) async fn cloudflare_api_request(
    token: &str,
    method: reqwest::Method,
    url: &str,
    body: Option<Value>,
) -> Result<Value> {
    let client = Client::new();
    let mut request = client
        .request(method, url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json");
    if let Some(payload) = body {
        let encoded = sonic_rs::to_string(&payload).context("failed to encode cloudflare request body")?;
        request = request.header(CONTENT_TYPE, "application/json").body(encoded);
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("cloudflare request failed: {url}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read cloudflare response: {url}"))?;
    if !status.is_success() {
        let detail = String::from_utf8_lossy(&bytes).to_string();
        return Err(anyhow!("cloudflare request failed ({status}): {detail}"));
    }
    let payload: Value = sonic_rs::from_slice(&bytes)
        .with_context(|| format!("failed to parse cloudflare response: {url}"))?;
    if payload["success"].as_bool().unwrap_or(false) {
        return Ok(payload);
    }
    Err(anyhow!(
        "cloudflare request rejected: {}",
        cloudflare_payload_error(&payload, "cloudflare API returned unsuccessful response")
    ))
}

pub(crate) async fn cloudflare_resolve_account_id(token: &str, requested: Option<&str>) -> Result<String> {
    let explicit = requested.unwrap_or_default().trim();
    if !explicit.is_empty() {
        return Ok(explicit.to_string());
    }
    let payload = cloudflare_api_request(
        token,
        reqwest::Method::GET,
        "https://api.cloudflare.com/client/v4/memberships",
        None,
    )
    .await?;
    let account_id = payload["result"]
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["account"]["id"].as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if account_id.is_empty() {
        return Err(anyhow!("cloudflare account membership not found for token"));
    }
    Ok(account_id)
}
