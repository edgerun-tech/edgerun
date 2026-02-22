// SPDX-License-Identifier: Apache-2.0
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Child;
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct JobCreateResponse {
    job_id: String,
}

pub(crate) async fn create_assigned_job(
    client: &reqwest::Client,
    sched_url: &str,
    worker: &str,
) -> Result<String> {
    create_assigned_job_with_abi(client, sched_url, worker, 2).await
}

pub(crate) async fn create_assigned_job_with_abi(
    client: &reqwest::Client,
    sched_url: &str,
    worker: &str,
    abi_version: u8,
) -> Result<String> {
    let body = json!({
        "runtime_id":"0000000000000000000000000000000000000000000000000000000000000000",
        "abi_version": abi_version,
        "wasm_base64":"AA==",
        "input_base64":"",
        "limits":{"max_memory_bytes":1048576,"max_instructions":10000},
        "escrow_lamports":1,
        "assignment_worker_pubkey":worker
    });
    let response: JobCreateResponse = client
        .post(format!("{sched_url}/v1/job/create"))
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response.job_id)
}

pub(crate) async fn wait_for_failure_phase(
    client: &reqwest::Client,
    sched_url: &str,
    job_id: &str,
    expected_phase: &str,
    invert: bool,
) -> Result<()> {
    for _ in 0..240 {
        let status: Value = client
            .get(format!("{sched_url}/v1/job/{job_id}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let phase = status["failures"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|x| x["phase"].as_str())
            .unwrap_or("");
        if (!invert && phase == expected_phase) || (invert && !phase.is_empty() && phase != expected_phase) {
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
    let path = std::env::temp_dir().join(format!("{}-{}-{}", prefix, now_unix_s, std::process::id()));
    std::fs::create_dir_all(&path)?;
    ensure!(path.is_dir(), "failed to create temp dir at {}", path.display());
    Ok(path)
}
