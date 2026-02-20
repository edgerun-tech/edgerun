use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    extract::Query,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct AssignmentsQuery {
    worker_pubkey: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
}

#[derive(Debug, Deserialize)]
struct HeartbeatRequest {
    worker_pubkey: String,
    runtime_ids: Vec<String>,
    version: String,
}

#[derive(Debug, Serialize)]
struct HeartbeatResponse {
    ok: bool,
    next_poll_ms: u64,
}

#[derive(Debug, Serialize)]
struct AssignmentsResponse {
    jobs: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct JobCreateRequest {
    runtime_id: String,
    wasm_base64: String,
    input_base64: String,
    limits: edgerun_types::Limits,
    escrow_lamports: u64,
}

#[derive(Debug, Serialize)]
struct JobCreateResponse {
    job_id: String,
    bundle_hash: String,
    bundle_url: String,
    post_job_tx: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/worker/heartbeat", post(worker_heartbeat))
        .route("/v1/worker/assignments", get(worker_assignments))
        .route("/v1/job/create", post(job_create));

    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    tracing::info!(%addr, "scheduler listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "edgerun-scheduler",
    })
}

async fn worker_heartbeat(Json(payload): Json<HeartbeatRequest>) -> Json<HeartbeatResponse> {
    tracing::info!(
        worker = %payload.worker_pubkey,
        runtime_count = payload.runtime_ids.len(),
        version = %payload.version,
        "received worker heartbeat"
    );

    Json(HeartbeatResponse {
        ok: true,
        next_poll_ms: 2000,
    })
}

async fn worker_assignments(Query(query): Query<AssignmentsQuery>) -> Json<AssignmentsResponse> {
    tracing::info!(worker = %query.worker_pubkey, "assignment poll");
    Json(AssignmentsResponse { jobs: Vec::new() })
}

async fn job_create(
    Json(payload): Json<JobCreateRequest>,
) -> Result<Json<JobCreateResponse>, (StatusCode, String)> {
    let wasm = base64::engine::general_purpose::STANDARD
        .decode(payload.wasm_base64.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid wasm_base64".to_string()))?;
    let input = base64::engine::general_purpose::STANDARD
        .decode(payload.input_base64.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid input_base64".to_string()))?;
    let runtime_id_bytes = hex::decode(payload.runtime_id.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid runtime_id hex".to_string()))?;
    if runtime_id_bytes.len() != 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "runtime_id must be 32 bytes".to_string(),
        ));
    }
    let mut runtime_id = [0_u8; 32];
    runtime_id.copy_from_slice(&runtime_id_bytes);

    let bundle_payload = edgerun_types::BundlePayload {
        v: 1,
        runtime_id,
        wasm,
        input,
        limits: payload.limits.clone(),
    };
    let bundle_payload_bytes = edgerun_types::encode_bundle_payload_canonical(&bundle_payload)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let bundle_hash = edgerun_crypto::compute_bundle_hash(&bundle_payload_bytes);
    let bundle_hash_hex = hex::encode(bundle_hash);

    tracing::info!(
        runtime_id = %payload.runtime_id,
        wasm_b64_len = payload.wasm_base64.len(),
        input_b64_len = payload.input_base64.len(),
        bundle_payload_len = bundle_payload_bytes.len(),
        max_memory = payload.limits.max_memory_bytes,
        max_instructions = payload.limits.max_instructions,
        escrow = payload.escrow_lamports,
        "job create requested"
    );

    Ok(Json(JobCreateResponse {
        // Job identity is bundle-hash keyed at MVP scaffold level.
        job_id: bundle_hash_hex.clone(),
        bundle_hash: bundle_hash_hex.clone(),
        bundle_url: format!("http://127.0.0.1:8081/bundle/{bundle_hash_hex}"),
        post_job_tx: "TODO_BASE64_TX".to_string(),
    }))
}
