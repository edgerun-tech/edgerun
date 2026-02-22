// SPDX-License-Identifier: Apache-2.0
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, ensure, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Child;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

static CONTROL_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize)]
struct JobCreateResponse {
    job_id: String,
}

#[derive(Debug, Deserialize)]
struct ControlWsServerMessage {
    request_id: String,
    ok: bool,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    status: Option<u16>,
}

fn control_ws_url(base: &str) -> Result<Url> {
    let mut url = Url::parse(base)?;
    let scheme = match url.scheme() {
        "http" => "ws".to_string(),
        "https" => "wss".to_string(),
        "ws" | "wss" => url.scheme().to_string(),
        other => anyhow::bail!("unsupported scheduler scheme for control ws: {other}"),
    };
    url.set_scheme(&scheme)
        .map_err(|_| anyhow!("failed to set websocket scheme"))?;
    url.set_path("/v1/control/ws");
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("client_id", "cli-integration-helper");
    Ok(url)
}

fn next_request_id() -> String {
    let seq = CONTROL_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("cli-int-{seq}")
}

pub(crate) async fn control_ws_request<T: for<'de> Deserialize<'de>>(
    sched_url: &str,
    op: &str,
    payload: serde_json::Value,
) -> Result<T> {
    let ws_url = control_ws_url(sched_url)?;
    let (mut socket, _) = tokio_tungstenite::connect_async(ws_url.as_str()).await?;
    let request_id = next_request_id();
    let request = json!({
        "request_id": request_id,
        "op": op,
        "payload": payload
    });
    socket
        .send(Message::Text(request.to_string().into()))
        .await?;
    while let Some(frame) = socket.next().await {
        let frame = frame?;
        let Message::Text(text) = frame else {
            continue;
        };
        let response: ControlWsServerMessage = serde_json::from_str(&text)?;
        if response.request_id != request_id {
            continue;
        }
        if !response.ok {
            let status = response
                .status
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();
            let err = response
                .error
                .unwrap_or_else(|| format!("control ws request failed{status}"));
            anyhow::bail!("{err}");
        }
        let data = response
            .data
            .ok_or_else(|| anyhow!("control ws response missing data"))?;
        return Ok(serde_json::from_value(data)?);
    }
    Err(anyhow!("scheduler closed control ws before response"))
}

pub(crate) async fn create_assigned_job(
    client: &reqwest::Client,
    sched_url: &str,
    worker: &str,
) -> Result<String> {
    create_assigned_job_with_abi(client, sched_url, worker, 2).await
}

pub(crate) async fn create_assigned_job_with_abi(
    _client: &reqwest::Client,
    sched_url: &str,
    worker: &str,
    abi_version: u8,
) -> Result<String> {
    let payload = json!({
        "runtime_id":"0000000000000000000000000000000000000000000000000000000000000000",
        "abi_version": abi_version,
        "wasm_base64":"AA==",
        "input_base64":"",
        "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
        "escrow_lamports":100,
        "assignment_worker_pubkey":worker
    });
    let response: JobCreateResponse = control_ws_request(sched_url, "job.create", payload).await?;
    Ok(response.job_id)
}

pub(crate) async fn wait_for_failure_phase(
    _client: &reqwest::Client,
    sched_url: &str,
    job_id: &str,
    expected_phase: &str,
    invert: bool,
) -> Result<()> {
    for _ in 0..240 {
        let status: Value =
            control_ws_request(sched_url, "job.status", json!({ "job_id": job_id })).await?;
        let phase = status["failures"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|x| x["phase"].as_str())
            .unwrap_or("");
        if (!invert && phase == expected_phase)
            || (invert && !phase.is_empty() && phase != expected_phase)
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow!("timed out waiting for expected failure phase"))
}

pub(crate) async fn wait_for_runtime_execute_failure(
    client: &reqwest::Client,
    sched_url: &str,
    job_id: &str,
) -> Result<()> {
    wait_for_failure_phase(client, sched_url, job_id, "runtime_execute", false).await
}

pub(crate) async fn wait_for_health(
    client: &reqwest::Client,
    sched_url: &str,
    scheduler: &mut Child,
) -> Result<()> {
    for _ in 0..240 {
        if scheduler.try_wait()?.is_some() {
            break;
        }
        if client
            .get(format!("{sched_url}/health"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow!("scheduler failed to become healthy"))
}

pub(crate) async fn kill_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

pub(crate) fn pick_free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

pub(crate) fn create_temp_dir(prefix: &str) -> Result<PathBuf> {
    let now_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path =
        std::env::temp_dir().join(format!("{}-{}-{}", prefix, now_unix_s, std::process::id()));
    std::fs::create_dir_all(&path)?;
    ensure!(
        path.is_dir(),
        "failed to create temp dir at {}",
        path.display()
    );
    Ok(path)
}
