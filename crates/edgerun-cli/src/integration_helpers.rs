// SPDX-License-Identifier: Apache-2.0
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, ensure, Result};
use edgerun_types::control_plane::{
    ControlWsClientMessage, ControlWsRequestPayload, ControlWsResponsePayload,
    ControlWsServerMessage, JobCreateRequest, JobCreateResponse, JobStatusRequest,
    JobStatusResponse, WorkerFailureReport, WorkerReplayArtifactReport, WorkerResultReport,
};
use edgerun_types::Limits;
use futures_util::{SinkExt, StreamExt};
use tokio::process::Child;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

static CONTROL_SEQ: AtomicU64 = AtomicU64::new(1);

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

pub(crate) async fn control_ws_request(
    sched_url: &str,
    payload: ControlWsRequestPayload,
) -> Result<ControlWsResponsePayload> {
    let ws_url = control_ws_url(sched_url)?;
    let (mut socket, _) = tokio_tungstenite::connect_async(ws_url.as_str()).await?;
    let request_id = next_request_id();
    let request = ControlWsClientMessage {
        request_id: request_id.clone(),
        payload,
    };
    let encoded = bincode::serialize(&request)?;
    socket.send(Message::Binary(encoded.into())).await?;
    while let Some(frame) = socket.next().await {
        let frame = frame?;
        let Message::Binary(bytes) = frame else {
            continue;
        };
        let response: ControlWsServerMessage = bincode::deserialize(&bytes)?;
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
        return Ok(data);
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
    let request = JobCreateRequest {
        runtime_id: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        wasm_base64: "AA==".to_string(),
        input_base64: String::new(),
        abi_version: Some(abi_version),
        limits: Limits {
            max_memory_bytes: 1_048_576,
            max_instructions: 10_000,
        },
        escrow_lamports: 100,
        assignment_worker_pubkey: Some(worker.to_string()),
        client_pubkey: None,
        client_signed_at_unix_s: None,
        client_signature: None,
    };
    let response =
        control_ws_request(sched_url, ControlWsRequestPayload::JobCreate(request)).await?;
    match response {
        ControlWsResponsePayload::JobCreate(JobCreateResponse { job_id, .. }) => Ok(job_id),
        other => Err(anyhow!(
            "unexpected control payload for job.create: {other:?}"
        )),
    }
}

pub(crate) async fn wait_for_failure_phase(
    _client: &reqwest::Client,
    sched_url: &str,
    job_id: &str,
    expected_phase: &str,
    invert: bool,
) -> Result<()> {
    for _ in 0..240 {
        let response = control_ws_request(
            sched_url,
            ControlWsRequestPayload::JobStatus(JobStatusRequest {
                job_id: job_id.to_string(),
            }),
        )
        .await?;
        let status = match response {
            ControlWsResponsePayload::JobStatus(body) => *body,
            other => {
                return Err(anyhow!(
                    "unexpected control payload for job.status: {other:?}"
                ))
            }
        };
        let phase = status
            .failures
            .last()
            .map(|f| f.phase.as_str())
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
    _client: &reqwest::Client,
    sched_url: &str,
    scheduler: &mut Child,
) -> Result<()> {
    for _ in 0..240 {
        if scheduler.try_wait()?.is_some() {
            break;
        }
        let ping = control_ws_request(
            sched_url,
            ControlWsRequestPayload::JobStatus(JobStatusRequest {
                job_id: "health-probe".to_string(),
            }),
        )
        .await;
        if ping.is_ok() {
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

pub(crate) async fn submit_worker_result(
    sched_url: &str,
    payload: WorkerResultReport,
) -> Result<bool> {
    match control_ws_request(sched_url, ControlWsRequestPayload::WorkerResult(payload)).await? {
        ControlWsResponsePayload::WorkerResult(ack) => Ok(ack.duplicate),
        other => Err(anyhow!(
            "unexpected control payload for worker.result: {other:?}"
        )),
    }
}

pub(crate) async fn submit_worker_failure(
    sched_url: &str,
    payload: WorkerFailureReport,
) -> Result<bool> {
    match control_ws_request(sched_url, ControlWsRequestPayload::WorkerFailure(payload)).await? {
        ControlWsResponsePayload::WorkerFailure(ack) => Ok(ack.duplicate),
        other => Err(anyhow!(
            "unexpected control payload for worker.failure: {other:?}"
        )),
    }
}

pub(crate) async fn submit_worker_replay(
    sched_url: &str,
    payload: WorkerReplayArtifactReport,
) -> Result<bool> {
    match control_ws_request(sched_url, ControlWsRequestPayload::WorkerReplay(payload)).await? {
        ControlWsResponsePayload::WorkerReplay(ack) => Ok(ack.duplicate),
        other => Err(anyhow!(
            "unexpected control payload for worker.replay: {other:?}"
        )),
    }
}

pub(crate) async fn fetch_job_status(sched_url: &str, job_id: &str) -> Result<JobStatusResponse> {
    match control_ws_request(
        sched_url,
        ControlWsRequestPayload::JobStatus(JobStatusRequest {
            job_id: job_id.to_string(),
        }),
    )
    .await?
    {
        ControlWsResponsePayload::JobStatus(body) => Ok(*body),
        other => Err(anyhow!(
            "unexpected control payload for job.status: {other:?}"
        )),
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
