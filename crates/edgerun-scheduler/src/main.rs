#![allow(deprecated)]

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use axum::{
    body::Bytes,
    extract::Query,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_program,
    transaction::Transaction,
};

#[derive(Clone)]
struct AppState {
    data_dir: PathBuf,
    public_base_url: String,
    retention: RetentionConfig,
    assignments: Arc<Mutex<HashMap<String, Vec<QueuedAssignment>>>>,
    results: Arc<Mutex<HashMap<String, Vec<WorkerResultReport>>>>,
    failures: Arc<Mutex<HashMap<String, Vec<WorkerFailureReport>>>>,
    replay_artifacts: Arc<Mutex<HashMap<String, Vec<WorkerReplayArtifactReport>>>>,
    job_last_update: Arc<Mutex<HashMap<String, u64>>>,
    worker_registry: Arc<Mutex<HashMap<String, WorkerRegistryEntry>>>,
    job_quorum: Arc<Mutex<HashMap<String, JobQuorumState>>>,
    policy_signing_key: SigningKey,
    policy_key_id: String,
    policy_version: u32,
    policy_ttl_secs: u64,
    committee_size: usize,
    quorum: usize,
    heartbeat_ttl_secs: u64,
    require_worker_signatures: bool,
    require_result_attestation: bool,
    quorum_requires_attestation: bool,
    chain_auto_submit: bool,
    job_timeout_secs: u64,
    chain: Option<Arc<ChainContext>>,
}

#[derive(Debug, Clone, Copy)]
struct RetentionConfig {
    max_reports_per_job: usize,
    max_failures_per_job: usize,
    max_replays_per_job: usize,
    max_jobs_tracked: usize,
}

struct ChainContext {
    rpc_url: String,
    rpc: RpcClient,
    program_id: Pubkey,
    payer: Keypair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedAssignment {
    job_id: String,
    bundle_hash: String,
    bundle_url: String,
    runtime_id: String,
    #[serde(default = "default_abi_version")]
    abi_version: u8,
    limits: edgerun_types::Limits,
    escrow_lamports: u64,
    #[serde(default)]
    policy_signer_pubkey: String,
    #[serde(default)]
    policy_signature: String,
    #[serde(default = "default_policy_key_id")]
    policy_key_id: String,
    #[serde(default = "default_policy_version")]
    policy_version: u32,
    #[serde(default)]
    policy_valid_after_unix_s: u64,
    #[serde(default)]
    policy_valid_until_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerResultReport {
    #[serde(default)]
    idempotency_key: String,
    worker_pubkey: String,
    job_id: String,
    bundle_hash: String,
    output_hash: String,
    output_len: usize,
    #[serde(default)]
    attestation_sig: Option<String>,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerFailureReport {
    #[serde(default)]
    idempotency_key: String,
    worker_pubkey: String,
    job_id: String,
    bundle_hash: String,
    phase: String,
    error_code: String,
    error_message: String,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplayArtifactPayload {
    bundle_hash: String,
    ok: bool,
    abi_version: Option<u8>,
    runtime_id: Option<String>,
    output_hash: Option<String>,
    output_len: Option<usize>,
    input_len: Option<usize>,
    max_memory_bytes: Option<u32>,
    max_instructions: Option<u64>,
    fuel_limit: Option<u64>,
    fuel_remaining: Option<u64>,
    error_code: Option<String>,
    error_message: Option<String>,
    trap_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerReplayArtifactReport {
    #[serde(default)]
    idempotency_key: String,
    worker_pubkey: String,
    job_id: String,
    artifact: ReplayArtifactPayload,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerRegistryEntry {
    worker_pubkey: String,
    runtime_ids: Vec<String>,
    version: String,
    #[serde(default)]
    max_concurrent: Option<u32>,
    #[serde(default)]
    mem_bytes: Option<u64>,
    last_heartbeat_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobQuorumState {
    #[serde(default)]
    expected_bundle_hash: String,
    #[serde(default)]
    expected_runtime_id: String,
    committee_workers: Vec<String>,
    committee_size: u8,
    quorum: u8,
    #[serde(default)]
    assign_tx: Option<String>,
    #[serde(default)]
    assign_sig: Option<String>,
    #[serde(default)]
    assign_submitted: bool,
    quorum_reached: bool,
    winning_output_hash: Option<String>,
    winning_workers: Vec<String>,
    #[serde(default)]
    finalize_triggered: bool,
    finalize_tx: Option<String>,
    finalize_sig: Option<String>,
    #[serde(default)]
    finalize_submitted: bool,
    #[serde(default)]
    cancel_triggered: bool,
    #[serde(default)]
    cancel_tx: Option<String>,
    #[serde(default)]
    cancel_sig: Option<String>,
    #[serde(default)]
    cancel_submitted: bool,
    #[serde(default)]
    onchain_status: Option<String>,
    #[serde(default)]
    onchain_last_observed_slot: Option<u64>,
    #[serde(default)]
    onchain_last_update_unix_s: Option<u64>,
    #[serde(default)]
    onchain_deadline_slot: Option<u64>,
    created_at_unix_s: u64,
    quorum_reached_at_unix_s: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct PersistedState {
    assignments: HashMap<String, Vec<QueuedAssignment>>,
    results: HashMap<String, Vec<WorkerResultReport>>,
    failures: HashMap<String, Vec<WorkerFailureReport>>,
    replay_artifacts: HashMap<String, Vec<WorkerReplayArtifactReport>>,
    job_last_update: HashMap<String, u64>,
    worker_registry: HashMap<String, WorkerRegistryEntry>,
    job_quorum: HashMap<String, JobQuorumState>,
}

#[derive(Debug, Deserialize)]
struct AssignmentsQuery {
    worker_pubkey: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    ok: bool,
    service: &'static str,
}

#[derive(Debug, Serialize)]
struct PolicyInfoResponse {
    key_id: String,
    version: u32,
    signer_pubkey: String,
    ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
struct HeartbeatRequest {
    worker_pubkey: String,
    runtime_ids: Vec<String>,
    version: String,
    #[serde(default)]
    capacity: Option<WorkerCapacity>,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerCapacity {
    max_concurrent: u32,
    mem_bytes: u64,
}

#[derive(Debug, Serialize)]
struct HeartbeatResponse {
    ok: bool,
    next_poll_ms: u64,
    server_time_unix_s: u64,
}

#[derive(Debug, Serialize)]
struct AssignmentsResponse {
    jobs: Vec<QueuedAssignment>,
}

#[derive(Debug, Deserialize)]
struct JobCreateRequest {
    runtime_id: String,
    wasm_base64: String,
    input_base64: String,
    abi_version: Option<u8>,
    limits: edgerun_types::Limits,
    escrow_lamports: u64,
    assignment_worker_pubkey: Option<String>,
}

#[derive(Debug, Serialize)]
struct JobCreateResponse {
    job_id: String,
    bundle_hash: String,
    bundle_url: String,
    post_job_tx: String,
    post_job_sig: Option<String>,
    assign_workers_tx: Option<String>,
    assign_workers_sig: Option<String>,
}

#[derive(Debug, Serialize)]
struct JobStatusResponse {
    job_id: String,
    reports: Vec<WorkerResultReport>,
    failures: Vec<WorkerFailureReport>,
    replay_artifacts: Vec<WorkerReplayArtifactReport>,
    quorum: Option<JobQuorumState>,
}

struct PostJobArgs {
    client: Pubkey,
    job_id: [u8; 32],
    bundle_hash: [u8; 32],
    runtime_id: [u8; 32],
    max_memory_bytes: u32,
    max_instructions: u64,
    escrow_lamports: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let data_dir = std::env::var("EDGERUN_SCHEDULER_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".edgerun-scheduler-data"));
    std::fs::create_dir_all(data_dir.join("bundles"))?;

    let persisted = load_state(&data_dir)?;
    let require_chain = read_env_bool("EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT", false);

    let chain = match init_chain_context() {
        Ok(ctx) => {
            tracing::info!(
                rpc = %ctx.rpc_url,
                payer = %ctx.payer.pubkey(),
                program_id = %ctx.program_id,
                "chain context initialized"
            );
            Some(Arc::new(ctx))
        }
        Err(err) => {
            if require_chain {
                anyhow::bail!(
                    "chain context required but unavailable (set EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT=false to allow placeholder mode): {err}"
                );
            } else {
                tracing::warn!(
                    error = %err,
                    "chain context unavailable; post_job_tx will be placeholder"
                );
                None
            }
        }
    };

    let addr: SocketAddr = std::env::var("EDGERUN_SCHEDULER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .context("invalid EDGERUN_SCHEDULER_ADDR")?;
    let configured_committee_size = read_env_usize("EDGERUN_SCHEDULER_COMMITTEE_SIZE", 3);
    if configured_committee_size != 3 {
        tracing::warn!(
            configured = configured_committee_size,
            "MVP scheduler enforces committee_size=3; ignoring configured value"
        );
    }
    let configured_quorum = read_env_usize("EDGERUN_SCHEDULER_QUORUM", 2);
    if configured_quorum != 2 {
        tracing::warn!(
            configured = configured_quorum,
            "MVP scheduler enforces quorum=2; ignoring configured value"
        );
    }

    let state = AppState {
        data_dir,
        public_base_url: std::env::var("EDGERUN_SCHEDULER_BASE_URL")
            .unwrap_or_else(|_| format!("http://{addr}")),
        retention: load_retention_config(),
        assignments: Arc::new(Mutex::new(persisted.assignments)),
        results: Arc::new(Mutex::new(persisted.results)),
        failures: Arc::new(Mutex::new(persisted.failures)),
        replay_artifacts: Arc::new(Mutex::new(persisted.replay_artifacts)),
        job_last_update: Arc::new(Mutex::new(persisted.job_last_update)),
        worker_registry: Arc::new(Mutex::new(persisted.worker_registry)),
        job_quorum: Arc::new(Mutex::new(persisted.job_quorum)),
        policy_signing_key: load_policy_signing_key()?,
        policy_key_id: std::env::var("EDGERUN_SCHEDULER_POLICY_KEY_ID")
            .unwrap_or_else(|_| default_policy_key_id()),
        policy_version: read_env_u32("EDGERUN_SCHEDULER_POLICY_VERSION", default_policy_version()),
        policy_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_POLICY_TTL_SECS", 300),
        committee_size: 3,
        quorum: 2,
        heartbeat_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_HEARTBEAT_TTL_SECS", 15),
        require_worker_signatures: read_env_bool(
            "EDGERUN_SCHEDULER_REQUIRE_WORKER_SIGNATURES",
            false,
        ),
        require_result_attestation: read_env_bool(
            "EDGERUN_SCHEDULER_REQUIRE_RESULT_ATTESTATION",
            false,
        ),
        quorum_requires_attestation: read_env_bool(
            "EDGERUN_SCHEDULER_QUORUM_REQUIRES_ATTESTATION",
            true,
        ),
        chain_auto_submit: read_env_bool("EDGERUN_SCHEDULER_CHAIN_AUTO_SUBMIT", false),
        job_timeout_secs: read_env_u64("EDGERUN_SCHEDULER_JOB_TIMEOUT_SECS", 60),
        chain,
    };
    enforce_history_retention(&state);

    let housekeeping_state = state.clone();
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/policy/info", get(policy_info))
        .route("/v1/worker/heartbeat", post(worker_heartbeat))
        .route("/v1/worker/assignments", get(worker_assignments))
        .route("/v1/worker/result", post(worker_result))
        .route("/v1/worker/failure", post(worker_failure))
        .route("/v1/worker/replay", post(worker_replay_artifact))
        .route("/v1/job/create", post(job_create))
        .route("/v1/job/{job_id}", get(get_job_status))
        .route("/bundle/{bundle_hash}", get(get_bundle))
        .with_state(state);
    tokio::spawn(async move {
        housekeeping_loop(housekeeping_state).await;
    });
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

async fn policy_info(State(state): State<AppState>) -> Json<PolicyInfoResponse> {
    Json(PolicyInfoResponse {
        key_id: state.policy_key_id.clone(),
        version: state.policy_version,
        signer_pubkey: hex::encode(state.policy_signing_key.verifying_key().as_bytes()),
        ttl_secs: state.policy_ttl_secs,
    })
}

async fn worker_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, String)> {
    if !verify_worker_message_signature(
        &state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &heartbeat_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    prune_expired_workers(&state);
    tracing::info!(
        worker = %payload.worker_pubkey,
        runtime_count = payload.runtime_ids.len(),
        version = %payload.version,
        "received worker heartbeat"
    );
    let now = now_unix_seconds();
    let (max_concurrent, mem_bytes) = payload
        .capacity
        .as_ref()
        .map(|c| (Some(c.max_concurrent), Some(c.mem_bytes)))
        .unwrap_or((None, None));
    {
        let mut registry = state.worker_registry.lock().expect("lock poisoned");
        registry.insert(
            payload.worker_pubkey.clone(),
            WorkerRegistryEntry {
                worker_pubkey: payload.worker_pubkey,
                runtime_ids: payload.runtime_ids,
                version: payload.version,
                max_concurrent,
                mem_bytes,
                last_heartbeat_unix_s: now,
            },
        );
    }
    if let Err(err) = write_state_snapshot(&state) {
        tracing::warn!(error = %err, "failed to persist state after worker heartbeat");
    }
    if let Err(err) = evaluate_expired_jobs(&state) {
        tracing::warn!(error = %err, "failed to evaluate expired jobs");
    }

    Ok(Json(HeartbeatResponse {
        ok: true,
        next_poll_ms: 2000,
        server_time_unix_s: now,
    }))
}

async fn worker_assignments(
    State(state): State<AppState>,
    Query(query): Query<AssignmentsQuery>,
) -> Result<Json<AssignmentsResponse>, (StatusCode, String)> {
    tracing::info!(worker = %query.worker_pubkey, "assignment poll");
    let mut assignments = state.assignments.lock().expect("lock poisoned");
    let jobs = assignments
        .remove(&query.worker_pubkey)
        .unwrap_or_else(Vec::new);
    drop(assignments);

    write_state_snapshot(&state).map_err(internal_err)?;
    Ok(Json(AssignmentsResponse { jobs }))
}

async fn worker_result(
    State(state): State<AppState>,
    Json(payload): Json<WorkerResultReport>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !verify_worker_message_signature(
        &state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &result_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    if !is_assigned_worker(&state, &payload.job_id, &payload.worker_pubkey) {
        return Err((
            StatusCode::FORBIDDEN,
            "worker is not assigned to this job".to_string(),
        ));
    }
    if !matches_expected_bundle_hash(&state, &payload.job_id, &payload.bundle_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            "bundle_hash does not match job expectation".to_string(),
        ));
    }
    if !verify_result_attestation(&state, &payload)? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid result attestation".to_string(),
        ));
    }
    tracing::info!(
        worker = %payload.worker_pubkey,
        job_id = %payload.job_id,
        output_hash = %payload.output_hash,
        output_len = payload.output_len,
        "worker result received"
    );

    let job_id = payload.job_id.clone();
    let mut results = state.results.lock().expect("lock poisoned");
    let entries = results.entry(payload.job_id.clone()).or_default();
    if entries.iter().any(|existing| {
        is_duplicate_idempotency(&existing.idempotency_key, &payload.idempotency_key)
    }) {
        drop(results);
        let quorum_reached = recompute_job_quorum(&state, &job_id).map_err(internal_err)?;
        return Ok(Json(serde_json::json!({
            "ok": true,
            "duplicate": true,
            "quorum_reached": quorum_reached
        })));
    }
    entries.push(payload);
    drop(results);
    let quorum_reached = recompute_job_quorum(&state, &job_id).map_err(internal_err)?;
    touch_job_last_update(&state, &job_id);
    enforce_history_retention(&state);
    evaluate_expired_jobs(&state).map_err(internal_err)?;

    write_state_snapshot(&state).map_err(internal_err)?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "duplicate": false,
        "quorum_reached": quorum_reached
    })))
}

async fn worker_failure(
    State(state): State<AppState>,
    Json(payload): Json<WorkerFailureReport>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !verify_worker_message_signature(
        &state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &failure_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    if !is_assigned_worker(&state, &payload.job_id, &payload.worker_pubkey) {
        return Err((
            StatusCode::FORBIDDEN,
            "worker is not assigned to this job".to_string(),
        ));
    }
    if !matches_expected_bundle_hash(&state, &payload.job_id, &payload.bundle_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            "bundle_hash does not match job expectation".to_string(),
        ));
    }
    tracing::warn!(
        worker = %payload.worker_pubkey,
        job_id = %payload.job_id,
        phase = %payload.phase,
        error_code = %payload.error_code,
        "worker failure received"
    );

    let job_id = payload.job_id.clone();
    let mut failures = state.failures.lock().expect("lock poisoned");
    let entries = failures.entry(payload.job_id.clone()).or_default();
    if entries.iter().any(|existing| {
        is_duplicate_idempotency(&existing.idempotency_key, &payload.idempotency_key)
    }) {
        drop(failures);
        return Ok(Json(serde_json::json!({ "ok": true, "duplicate": true })));
    }
    entries.push(payload);
    drop(failures);
    touch_job_last_update(&state, &job_id);
    enforce_history_retention(&state);
    evaluate_expired_jobs(&state).map_err(internal_err)?;

    write_state_snapshot(&state).map_err(internal_err)?;
    Ok(Json(serde_json::json!({ "ok": true, "duplicate": false })))
}

async fn worker_replay_artifact(
    State(state): State<AppState>,
    Json(payload): Json<WorkerReplayArtifactReport>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !verify_worker_message_signature(
        &state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &replay_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    if !is_assigned_worker(&state, &payload.job_id, &payload.worker_pubkey) {
        return Err((
            StatusCode::FORBIDDEN,
            "worker is not assigned to this job".to_string(),
        ));
    }
    if !matches_expected_bundle_hash(&state, &payload.job_id, &payload.artifact.bundle_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            "artifact.bundle_hash does not match job expectation".to_string(),
        ));
    }
    if let Some(runtime_id) = payload.artifact.runtime_id.as_deref() {
        if !matches_expected_runtime_id(&state, &payload.job_id, runtime_id) {
            return Err((
                StatusCode::BAD_REQUEST,
                "artifact.runtime_id does not match job expectation".to_string(),
            ));
        }
    }
    tracing::info!(
        worker = %payload.worker_pubkey,
        job_id = %payload.job_id,
        ok = payload.artifact.ok,
        error_code = ?payload.artifact.error_code,
        "worker replay artifact received"
    );

    let job_id = payload.job_id.clone();
    let mut replay_artifacts = state.replay_artifacts.lock().expect("lock poisoned");
    let entries = replay_artifacts.entry(payload.job_id.clone()).or_default();
    if entries.iter().any(|existing| {
        is_duplicate_idempotency(&existing.idempotency_key, &payload.idempotency_key)
    }) {
        drop(replay_artifacts);
        return Ok(Json(serde_json::json!({ "ok": true, "duplicate": true })));
    }
    entries.push(payload);
    drop(replay_artifacts);
    touch_job_last_update(&state, &job_id);
    enforce_history_retention(&state);
    evaluate_expired_jobs(&state).map_err(internal_err)?;

    write_state_snapshot(&state).map_err(internal_err)?;
    Ok(Json(serde_json::json!({ "ok": true, "duplicate": false })))
}

async fn job_create(
    State(state): State<AppState>,
    Json(payload): Json<JobCreateRequest>,
) -> Result<Json<JobCreateResponse>, (StatusCode, String)> {
    let wasm = base64::engine::general_purpose::STANDARD
        .decode(payload.wasm_base64.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid wasm_base64".to_string()))?;
    let input = base64::engine::general_purpose::STANDARD
        .decode(payload.input_base64.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid input_base64".to_string()))?;
    let runtime_id_bytes = hex::decode(payload.runtime_id.as_bytes()).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid runtime_id hex".to_string(),
        )
    })?;
    if runtime_id_bytes.len() != 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "runtime_id must be 32 bytes".to_string(),
        ));
    }
    let abi_version = payload
        .abi_version
        .unwrap_or(edgerun_types::BUNDLE_ABI_CURRENT);
    if !(edgerun_types::BUNDLE_ABI_MIN_SUPPORTED..=edgerun_types::BUNDLE_ABI_CURRENT)
        .contains(&abi_version)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "abi_version must be in [{}..={}], got {}",
                edgerun_types::BUNDLE_ABI_MIN_SUPPORTED,
                edgerun_types::BUNDLE_ABI_CURRENT,
                abi_version
            ),
        ));
    }
    let mut runtime_id = [0_u8; 32];
    runtime_id.copy_from_slice(&runtime_id_bytes);

    let bundle_payload = edgerun_types::BundlePayload {
        v: abi_version,
        runtime_id,
        wasm,
        input,
        limits: payload.limits.clone(),
        meta: None,
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
        abi_version = bundle_payload.v,
        max_memory = payload.limits.max_memory_bytes,
        max_instructions = payload.limits.max_instructions,
        escrow = payload.escrow_lamports,
        "job create requested"
    );

    let bundle_path = bundle_path(&state, &bundle_hash_hex);
    std::fs::write(&bundle_path, &bundle_payload_bytes).map_err(internal_err)?;

    prune_expired_workers(&state);
    let committee_workers = if let Some(worker_pubkey) = payload.assignment_worker_pubkey.as_ref() {
        vec![worker_pubkey.clone()]
    } else {
        let selected = select_committee_workers(
            &state,
            &payload.runtime_id,
            &bundle_hash_hex,
            state.committee_size,
        );
        if selected.len() < state.quorum.max(1) {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!(
                    "insufficient eligible workers for quorum: have {}, need {}",
                    selected.len(),
                    state.quorum.max(1)
                ),
            ));
        }
        selected
    };

    let effective_quorum = if payload.assignment_worker_pubkey.is_some() {
        1
    } else {
        state.quorum.max(1).min(committee_workers.len())
    };
    let now = now_unix_seconds();
    let mut queued: Vec<(String, QueuedAssignment)> = Vec::with_capacity(committee_workers.len());
    for worker_pubkey in &committee_workers {
        let mut assignment = QueuedAssignment {
            job_id: bundle_hash_hex.clone(),
            bundle_hash: bundle_hash_hex.clone(),
            bundle_url: format!("{}/bundle/{bundle_hash_hex}", state.public_base_url),
            runtime_id: payload.runtime_id.clone(),
            abi_version: bundle_payload.v,
            limits: payload.limits.clone(),
            escrow_lamports: payload.escrow_lamports,
            policy_signer_pubkey: hex::encode(state.policy_signing_key.verifying_key().as_bytes()),
            policy_signature: String::new(),
            policy_key_id: state.policy_key_id.clone(),
            policy_version: state.policy_version,
            policy_valid_after_unix_s: now,
            policy_valid_until_unix_s: now.saturating_add(state.policy_ttl_secs),
        };
        assignment.policy_signature =
            sign_assignment_policy(&state.policy_signing_key, &assignment);
        queued.push((worker_pubkey.clone(), assignment));
    }

    {
        let mut assignments = state.assignments.lock().expect("lock poisoned");
        for (worker_pubkey, assignment) in queued {
            assignments
                .entry(worker_pubkey)
                .or_default()
                .push(assignment);
        }
    }
    {
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum.insert(
            bundle_hash_hex.clone(),
            JobQuorumState {
                expected_bundle_hash: bundle_hash_hex.clone(),
                expected_runtime_id: payload.runtime_id.clone(),
                committee_size: committee_workers.len().min(u8::MAX as usize) as u8,
                quorum: effective_quorum.min(u8::MAX as usize) as u8,
                committee_workers,
                assign_tx: None,
                assign_sig: None,
                assign_submitted: false,
                quorum_reached: false,
                winning_output_hash: None,
                winning_workers: Vec::new(),
                finalize_triggered: false,
                finalize_tx: None,
                finalize_sig: None,
                finalize_submitted: false,
                cancel_triggered: false,
                cancel_tx: None,
                cancel_sig: None,
                cancel_submitted: false,
                onchain_status: None,
                onchain_last_observed_slot: None,
                onchain_last_update_unix_s: None,
                onchain_deadline_slot: None,
                created_at_unix_s: now,
                quorum_reached_at_unix_s: None,
            },
        );
    }
    touch_job_last_update(&state, &bundle_hash_hex);
    let (assign_workers_tx, assign_workers_sig, assign_workers_submitted) =
        build_assign_workers_artifact(&state, &bundle_hash_hex).map_err(internal_err)?;
    {
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        if let Some(quorum_state) = job_quorum.get_mut(&bundle_hash_hex) {
            quorum_state.assign_tx = assign_workers_tx.clone();
            quorum_state.assign_sig = assign_workers_sig.clone();
            quorum_state.assign_submitted = assign_workers_submitted;
        }
    }
    write_state_snapshot(&state).map_err(internal_err)?;
    evaluate_expired_jobs(&state).map_err(internal_err)?;

    let (post_job_tx, post_job_sig) = if let Some(chain) = state.chain.as_ref() {
        let tx_args = PostJobArgs {
            client: chain.payer.pubkey(),
            job_id: bundle_hash,
            bundle_hash,
            runtime_id,
            max_memory_bytes: payload.limits.max_memory_bytes,
            max_instructions: payload.limits.max_instructions,
            escrow_lamports: payload.escrow_lamports,
        };
        match build_post_job_tx_base64(chain, tx_args) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, "failed to build chain post_job tx");
                ("UNAVAILABLE_BUILD_FAILED".to_string(), None)
            }
        }
    } else {
        ("UNAVAILABLE_NO_CHAIN_CONTEXT".to_string(), None)
    };

    Ok(Json(JobCreateResponse {
        // Job identity is bundle-hash keyed at MVP scaffold level.
        job_id: bundle_hash_hex.clone(),
        bundle_hash: bundle_hash_hex.clone(),
        bundle_url: format!("{}/bundle/{bundle_hash_hex}", state.public_base_url),
        post_job_tx,
        post_job_sig,
        assign_workers_tx,
        assign_workers_sig,
    }))
}

async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Json<JobStatusResponse> {
    let results = state.results.lock().expect("lock poisoned");
    let reports = results.get(&job_id).cloned().unwrap_or_default();
    drop(results);

    let failures = state.failures.lock().expect("lock poisoned");
    let failures = failures.get(&job_id).cloned().unwrap_or_default();

    let replay_artifacts = state.replay_artifacts.lock().expect("lock poisoned");
    let replay_artifacts = replay_artifacts.get(&job_id).cloned().unwrap_or_default();
    let quorum = state
        .job_quorum
        .lock()
        .expect("lock poisoned")
        .get(&job_id)
        .cloned();
    Json(JobStatusResponse {
        job_id,
        reports,
        failures,
        replay_artifacts,
        quorum,
    })
}

async fn get_bundle(
    State(state): State<AppState>,
    Path(bundle_hash): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let path = bundle_path(&state, &bundle_hash);
    let bytes =
        std::fs::read(path).map_err(|_| (StatusCode::NOT_FOUND, "bundle not found".to_string()))?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/cbor")],
        Bytes::from(bytes),
    ))
}

fn init_chain_context() -> Result<ChainContext> {
    let rpc_url = std::env::var("EDGERUN_CHAIN_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let program_id_str = std::env::var("EDGERUN_CHAIN_PROGRAM_ID")
        .unwrap_or_else(|_| "AgjxA2CoMmmWXrcsJtvvpmqdRHLVHrhYf6DAuBCL4s5T".to_string());
    let wallet_path = std::env::var("EDGERUN_CHAIN_WALLET")
        .unwrap_or_else(|_| "program/.solana/id.json".to_string());

    let program_id = program_id_str
        .parse::<Pubkey>()
        .context("invalid EDGERUN_CHAIN_PROGRAM_ID")?;
    let payer = read_keypair_file(&wallet_path)
        .map_err(|e| anyhow::anyhow!("failed to read EDGERUN_CHAIN_WALLET {wallet_path}: {e}"))?;
    let rpc = RpcClient::new(rpc_url.clone());

    Ok(ChainContext {
        rpc_url,
        rpc,
        program_id,
        payer,
    })
}

fn build_post_job_tx_base64(
    chain: &ChainContext,
    args: PostJobArgs,
) -> Result<(String, Option<String>)> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &chain.program_id);
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &args.job_id], &chain.program_id);

    let ix = Instruction {
        program_id: chain.program_id,
        accounts: vec![
            AccountMeta::new(args.client, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(job_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: encode_post_job_data(args),
    };

    let blockhash = chain
        .rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&chain.payer.pubkey()),
        &[&chain.payer],
        blockhash,
    );

    let signature = tx.signatures.first().map(ToString::to_string);
    let tx_bytes = bincode::serialize(&tx).context("failed to serialize transaction")?;
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    Ok((tx_b64, signature))
}

fn build_finalize_job_tx_base64(
    chain: &ChainContext,
    job_id: [u8; 32],
    committee: [Pubkey; 3],
    winners: Vec<Pubkey>,
    winning_output_hash_hex: &str,
    auto_submit: bool,
) -> Result<(String, Option<String>, bool)> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &chain.program_id);
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &chain.program_id);
    let (worker_stake_0, _) =
        Pubkey::find_program_address(&[b"worker_stake", committee[0].as_ref()], &chain.program_id);
    let (worker_stake_1, _) =
        Pubkey::find_program_address(&[b"worker_stake", committee[1].as_ref()], &chain.program_id);
    let (worker_stake_2, _) =
        Pubkey::find_program_address(&[b"worker_stake", committee[2].as_ref()], &chain.program_id);
    let winning_output_hash =
        parse_hex32(winning_output_hash_hex).context("winning output hash must be 32-byte hex")?;
    let winner_count = winners.len().min(u8::MAX as usize) as u8;

    let mut accounts = vec![
        AccountMeta::new(chain.payer.pubkey(), true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new(job_pda, false),
        AccountMeta::new(worker_stake_0, false),
        AccountMeta::new(worker_stake_1, false),
        AccountMeta::new(worker_stake_2, false),
    ];
    for winner in winners {
        accounts.push(AccountMeta::new(winner, false));
    }
    let ix = Instruction {
        program_id: chain.program_id,
        accounts,
        data: encode_finalize_job_data(winning_output_hash, winner_count),
    };

    let blockhash = chain
        .rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&chain.payer.pubkey()),
        &[&chain.payer],
        blockhash,
    );

    let signature = tx.signatures.first().map(ToString::to_string);
    let tx_bytes = bincode::serialize(&tx).context("failed to serialize transaction")?;
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    if auto_submit {
        let sent = chain
            .rpc
            .send_and_confirm_transaction(&tx)
            .context("failed to send finalize_job transaction")?;
        return Ok((tx_b64, Some(sent.to_string()), true));
    }
    Ok((tx_b64, signature, false))
}

fn build_assign_workers_tx_base64(
    chain: &ChainContext,
    job_id: [u8; 32],
    workers: [Pubkey; 3],
    auto_submit: bool,
) -> Result<(String, Option<String>, bool)> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &chain.program_id);
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &chain.program_id);
    let (worker_stake_0, _) =
        Pubkey::find_program_address(&[b"worker_stake", workers[0].as_ref()], &chain.program_id);
    let (worker_stake_1, _) =
        Pubkey::find_program_address(&[b"worker_stake", workers[1].as_ref()], &chain.program_id);
    let (worker_stake_2, _) =
        Pubkey::find_program_address(&[b"worker_stake", workers[2].as_ref()], &chain.program_id);

    let ix = Instruction {
        program_id: chain.program_id,
        accounts: vec![
            AccountMeta::new(chain.payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(job_pda, false),
            AccountMeta::new(worker_stake_0, false),
            AccountMeta::new(worker_stake_1, false),
            AccountMeta::new(worker_stake_2, false),
        ],
        data: encode_assign_workers_data(workers),
    };

    let blockhash = chain
        .rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&chain.payer.pubkey()),
        &[&chain.payer],
        blockhash,
    );
    let signature = tx.signatures.first().map(ToString::to_string);
    let tx_bytes = bincode::serialize(&tx).context("failed to serialize transaction")?;
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    if auto_submit {
        let sent = chain
            .rpc
            .send_and_confirm_transaction(&tx)
            .context("failed to send assign_workers transaction")?;
        return Ok((tx_b64, Some(sent.to_string()), true));
    }
    Ok((tx_b64, signature, false))
}

fn build_cancel_expired_job_tx_base64(
    chain: &ChainContext,
    job_id: [u8; 32],
    client: Pubkey,
    auto_submit: bool,
) -> Result<(String, Option<String>, bool)> {
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &chain.program_id);
    let ix = Instruction {
        program_id: chain.program_id,
        accounts: vec![
            AccountMeta::new(chain.payer.pubkey(), true),
            AccountMeta::new(job_pda, false),
            AccountMeta::new(client, false),
        ],
        data: encode_cancel_expired_job_data(),
    };
    let blockhash = chain
        .rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&chain.payer.pubkey()),
        &[&chain.payer],
        blockhash,
    );
    let signature = tx.signatures.first().map(ToString::to_string);
    let tx_bytes = bincode::serialize(&tx).context("failed to serialize transaction")?;
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    if auto_submit {
        let sent = chain
            .rpc
            .send_and_confirm_transaction(&tx)
            .context("failed to send cancel_expired_job transaction")?;
        return Ok((tx_b64, Some(sent.to_string()), true));
    }
    Ok((tx_b64, signature, false))
}

fn encode_post_job_data(args: PostJobArgs) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32 + 32 + 32 + 4 + 8 + 8);
    data.extend_from_slice(&anchor_discriminator("post_job"));
    data.extend_from_slice(&args.job_id);
    data.extend_from_slice(&args.bundle_hash);
    data.extend_from_slice(&args.runtime_id);
    data.extend_from_slice(&args.max_memory_bytes.to_le_bytes());
    data.extend_from_slice(&args.max_instructions.to_le_bytes());
    data.extend_from_slice(&args.escrow_lamports.to_le_bytes());
    data
}

fn encode_assign_workers_data(workers: [Pubkey; 3]) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32 * 3);
    data.extend_from_slice(&anchor_discriminator("assign_workers"));
    data.extend_from_slice(workers[0].as_ref());
    data.extend_from_slice(workers[1].as_ref());
    data.extend_from_slice(workers[2].as_ref());
    data
}

fn encode_finalize_job_data(winning_output_hash: [u8; 32], winner_count: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32 + 1);
    data.extend_from_slice(&anchor_discriminator("finalize_job"));
    data.extend_from_slice(&winning_output_hash);
    data.push(winner_count);
    data
}

fn encode_cancel_expired_job_data() -> Vec<u8> {
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&anchor_discriminator("cancel_expired_job"));
    data
}

fn anchor_discriminator(ix_name: &str) -> [u8; 8] {
    let preimage = format!("global:{ix_name}");
    let h = hash(preimage.as_bytes());
    let mut out = [0_u8; 8];
    out.copy_from_slice(&h.to_bytes()[..8]);
    out
}

fn bundle_path(state: &AppState, bundle_hash: &str) -> PathBuf {
    state
        .data_dir
        .join("bundles")
        .join(format!("{bundle_hash}.cbor"))
}

fn load_state(data_dir: &FsPath) -> Result<PersistedState> {
    let path = data_dir.join("state.json");
    if !path.exists() {
        return Ok(PersistedState::default());
    }
    let bytes = std::fs::read(path)?;
    let state = serde_json::from_slice::<PersistedState>(&bytes)?;
    Ok(state)
}

fn write_state_snapshot(state: &AppState) -> Result<()> {
    let assignments = state.assignments.lock().expect("lock poisoned").clone();
    let results = state.results.lock().expect("lock poisoned").clone();

    let snapshot = PersistedState {
        assignments,
        results,
        failures: state.failures.lock().expect("lock poisoned").clone(),
        replay_artifacts: state
            .replay_artifacts
            .lock()
            .expect("lock poisoned")
            .clone(),
        job_last_update: state.job_last_update.lock().expect("lock poisoned").clone(),
        worker_registry: state.worker_registry.lock().expect("lock poisoned").clone(),
        job_quorum: state.job_quorum.lock().expect("lock poisoned").clone(),
    };
    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    std::fs::write(state.data_dir.join("state.json"), bytes)?;
    Ok(())
}

fn internal_err<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn default_abi_version() -> u8 {
    edgerun_types::BUNDLE_ABI_CURRENT
}

fn default_policy_key_id() -> String {
    "dev-key-1".to_string()
}

fn default_policy_version() -> u32 {
    1
}

fn load_policy_signing_key() -> Result<SigningKey> {
    let hex_key = std::env::var("EDGERUN_SCHEDULER_POLICY_SIGNING_KEY_HEX").unwrap_or_else(|_| {
        "0101010101010101010101010101010101010101010101010101010101010101".to_string()
    });
    let bytes = hex::decode(hex_key.trim()).context("policy signing key must be hex")?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("policy signing key must decode to 32 bytes"))?;
    Ok(SigningKey::from_bytes(&arr))
}

fn assignment_policy_message(assignment: &QueuedAssignment) -> String {
    format!(
        "edgerun-assignment-v2|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        assignment.job_id,
        assignment.bundle_hash,
        assignment.runtime_id,
        assignment.abi_version,
        assignment.limits.max_memory_bytes,
        assignment.limits.max_instructions,
        assignment.escrow_lamports,
        assignment.bundle_url,
        assignment.policy_key_id,
        assignment.policy_version,
        assignment.policy_valid_after_unix_s,
        assignment.policy_valid_until_unix_s
    )
}

fn sign_assignment_policy(signing_key: &SigningKey, assignment: &QueuedAssignment) -> String {
    let message = assignment_policy_message(assignment);
    let sig: Signature = edgerun_crypto::sign(signing_key, message.as_bytes());
    hex::encode(sig.to_bytes())
}

fn prune_expired_workers(state: &AppState) {
    let cutoff = now_unix_seconds().saturating_sub(state.heartbeat_ttl_secs);
    let mut registry = state.worker_registry.lock().expect("lock poisoned");
    registry.retain(|_, entry| entry.last_heartbeat_unix_s >= cutoff);
}

fn select_committee_workers(
    state: &AppState,
    runtime_id: &str,
    seed: &str,
    committee_size: usize,
) -> Vec<String> {
    let now = now_unix_seconds();
    let cutoff = now.saturating_sub(state.heartbeat_ttl_secs);
    let registry = state.worker_registry.lock().expect("lock poisoned");
    let mut eligible = registry
        .values()
        .filter(|entry| {
            entry.last_heartbeat_unix_s >= cutoff
                && entry
                    .runtime_ids
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(runtime_id))
        })
        .map(|entry| entry.worker_pubkey.clone())
        .collect::<Vec<_>>();
    drop(registry);

    eligible.sort_by(|a, b| {
        let a_score = hash(format!("{seed}|{runtime_id}|{a}").as_bytes());
        let b_score = hash(format!("{seed}|{runtime_id}|{b}").as_bytes());
        a_score
            .to_bytes()
            .cmp(&b_score.to_bytes())
            .then_with(|| a.cmp(b))
    });
    eligible.truncate(committee_size.max(1));
    eligible
}

fn is_assigned_worker(state: &AppState, job_id: &str, worker_pubkey: &str) -> bool {
    let job_quorum = state.job_quorum.lock().expect("lock poisoned");
    let Some(quorum_state) = job_quorum.get(job_id) else {
        return false;
    };
    quorum_state
        .committee_workers
        .iter()
        .any(|worker| worker == worker_pubkey)
}

fn matches_expected_bundle_hash(state: &AppState, job_id: &str, bundle_hash: &str) -> bool {
    let job_quorum = state.job_quorum.lock().expect("lock poisoned");
    let Some(quorum_state) = job_quorum.get(job_id) else {
        return false;
    };
    if quorum_state.expected_bundle_hash.is_empty() {
        return true;
    }
    quorum_state
        .expected_bundle_hash
        .eq_ignore_ascii_case(bundle_hash)
}

fn matches_expected_runtime_id(state: &AppState, job_id: &str, runtime_id: &str) -> bool {
    let job_quorum = state.job_quorum.lock().expect("lock poisoned");
    let Some(quorum_state) = job_quorum.get(job_id) else {
        return false;
    };
    if quorum_state.expected_runtime_id.is_empty() {
        return true;
    }
    quorum_state
        .expected_runtime_id
        .eq_ignore_ascii_case(runtime_id)
}

fn recompute_job_quorum(state: &AppState, job_id: &str) -> Result<bool> {
    let reports = {
        let results = state.results.lock().expect("lock poisoned");
        results.get(job_id).cloned().unwrap_or_default()
    };
    let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
    let Some(quorum_state) = job_quorum.get_mut(job_id) else {
        return Ok(false);
    };
    let quorum_target = usize::from(quorum_state.quorum.max(1));
    let expected_bundle_hash = quorum_state.expected_bundle_hash.clone();
    let quorum_requires_attestation = state.quorum_requires_attestation;
    let filtered_reports = reports
        .into_iter()
        .filter(|report| {
            (expected_bundle_hash.is_empty()
                || expected_bundle_hash.eq_ignore_ascii_case(&report.bundle_hash))
                && (!quorum_requires_attestation
                    || (report.attestation_sig.is_some()
                        && verify_result_attestation(state, report).unwrap_or(false)))
        })
        .collect::<Vec<_>>();
    let Some((winning_hash, winning_workers)) =
        find_winning_output_hash(&filtered_reports, quorum_target)
    else {
        return Ok(false);
    };

    let was_reached = quorum_state.quorum_reached;
    let changed_hash = quorum_state.winning_output_hash.as_deref() != Some(winning_hash.as_str());
    quorum_state.quorum_reached = true;
    quorum_state.winning_output_hash = Some(winning_hash.clone());
    quorum_state.winning_workers = winning_workers;
    if quorum_state.quorum_reached_at_unix_s.is_none() {
        quorum_state.quorum_reached_at_unix_s = Some(now_unix_seconds());
    }

    let should_trigger_finalize =
        !quorum_state.finalize_triggered && (changed_hash || !was_reached);
    let committee_workers = quorum_state.committee_workers.clone();
    let winners = quorum_state.winning_workers.clone();
    drop(job_quorum);

    if should_trigger_finalize {
        let (finalize_tx, finalize_sig, finalize_submitted) = build_finalize_trigger_payload(
            state,
            job_id,
            &winning_hash,
            &committee_workers,
            &winners,
        );
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        if let Some(quorum_state) = job_quorum.get_mut(job_id) {
            quorum_state.finalize_triggered = true;
            quorum_state.finalize_tx = Some(finalize_tx);
            quorum_state.finalize_sig = finalize_sig;
            quorum_state.finalize_submitted = finalize_submitted;
        }
    }
    Ok(true)
}

fn find_winning_output_hash(
    reports: &[WorkerResultReport],
    quorum_target: usize,
) -> Option<(String, Vec<String>)> {
    let mut seen_workers: HashSet<&str> = HashSet::new();
    let mut counts: HashMap<&str, Vec<String>> = HashMap::new();
    for report in reports {
        if !seen_workers.insert(report.worker_pubkey.as_str()) {
            continue;
        }
        counts
            .entry(report.output_hash.as_str())
            .or_default()
            .push(report.worker_pubkey.clone());
    }

    counts
        .into_iter()
        .filter_map(|(output_hash, workers)| {
            if workers.len() >= quorum_target {
                Some((output_hash.to_string(), workers))
            } else {
                None
            }
        })
        .max_by(|(a_hash, a_workers), (b_hash, b_workers)| {
            a_workers
                .len()
                .cmp(&b_workers.len())
                .then_with(|| b_hash.cmp(a_hash))
        })
}

fn build_finalize_trigger_payload_inner(
    state: &AppState,
    job_id_hex: &str,
    winning_output_hash: &str,
    committee_workers: &[String],
    winning_workers: &[String],
) -> (String, Option<String>, bool) {
    if let Some(chain) = state.chain.as_ref() {
        if let Ok(job_id) = parse_hex32(job_id_hex) {
            if let Some((committee, winners)) =
                parse_finalize_accounts(committee_workers, winning_workers)
            {
                match build_finalize_job_tx_base64(
                    chain,
                    job_id,
                    committee,
                    winners,
                    winning_output_hash,
                    state.chain_auto_submit,
                ) {
                    Ok((tx, sig, submitted)) => return (tx, sig, submitted),
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            job_id = %job_id_hex,
                            "failed to build finalize tx"
                        )
                    }
                }
            } else {
                tracing::warn!(
                    job_id = %job_id_hex,
                    "unable to derive finalize account metas from committee/winner pubkeys"
                );
            }
        } else {
            tracing::warn!(job_id = %job_id_hex, "invalid job id hex for finalize tx build");
        }
    }
    (
        format!("UNAVAILABLE_FINALIZE_{winning_output_hash}"),
        None,
        false,
    )
}

fn parse_finalize_accounts(
    committee_workers: &[String],
    winning_workers: &[String],
) -> Option<([Pubkey; 3], Vec<Pubkey>)> {
    if committee_workers.len() != 3 {
        return None;
    }
    let committee = [
        committee_workers.first()?.parse::<Pubkey>().ok()?,
        committee_workers.get(1)?.parse::<Pubkey>().ok()?,
        committee_workers.get(2)?.parse::<Pubkey>().ok()?,
    ];
    let committee_set: HashSet<Pubkey> = committee.iter().copied().collect();
    let winners = winning_workers
        .iter()
        .filter_map(|worker| worker.parse::<Pubkey>().ok())
        .filter(|worker| committee_set.contains(worker))
        .collect::<Vec<_>>();
    if winners.is_empty() {
        return None;
    }
    Some((committee, winners))
}

fn build_assign_workers_artifact(
    state: &AppState,
    job_id_hex: &str,
) -> Result<(Option<String>, Option<String>, bool)> {
    let Some(chain) = state.chain.as_ref() else {
        return Ok((None, None, false));
    };
    let job_id = parse_hex32(job_id_hex)?;
    let committee_workers = {
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum
            .get(job_id_hex)
            .map(|q| q.committee_workers.clone())
            .unwrap_or_default()
    };
    if committee_workers.len() != 3 {
        return Ok((
            Some("UNAVAILABLE_ASSIGN_COMMITTEE_NOT_FIXED3".to_string()),
            None,
            false,
        ));
    }
    let workers = [
        committee_workers[0]
            .parse::<Pubkey>()
            .context("committee worker pubkey invalid")?,
        committee_workers[1]
            .parse::<Pubkey>()
            .context("committee worker pubkey invalid")?,
        committee_workers[2]
            .parse::<Pubkey>()
            .context("committee worker pubkey invalid")?,
    ];
    let (tx, sig, submitted) =
        build_assign_workers_tx_base64(chain, job_id, workers, state.chain_auto_submit)?;
    Ok((Some(tx), sig, submitted))
}

fn evaluate_expired_jobs(state: &AppState) -> Result<()> {
    let now = now_unix_seconds();
    let timeout = state.job_timeout_secs.max(1);
    let chain_slot = state
        .chain
        .as_ref()
        .and_then(|chain| match chain.rpc.get_slot() {
            Ok(slot) => Some(slot),
            Err(err) => {
                tracing::warn!(error = %err, "failed to read current chain slot for expiry evaluation");
                None
            }
        });
    let candidates = {
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum
            .iter()
            .filter_map(|(job_id, quorum_state)| {
                let expired = if let Some(deadline_slot) = quorum_state.onchain_deadline_slot {
                    chain_slot
                        .map(|slot| slot >= deadline_slot)
                        .unwrap_or_else(|| {
                            now.saturating_sub(quorum_state.created_at_unix_s) >= timeout
                        })
                } else {
                    now.saturating_sub(quorum_state.created_at_unix_s) >= timeout
                };
                if expired && !quorum_state.quorum_reached && !quorum_state.cancel_triggered {
                    Some(job_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    if candidates.is_empty() {
        return Ok(());
    }

    for job_id in candidates {
        let (cancel_tx, cancel_sig, cancel_submitted) =
            build_cancel_expired_artifact(state, &job_id)?;
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        if let Some(quorum_state) = job_quorum.get_mut(&job_id) {
            quorum_state.cancel_triggered = true;
            quorum_state.cancel_tx = Some(cancel_tx);
            quorum_state.cancel_sig = cancel_sig;
            quorum_state.cancel_submitted = cancel_submitted;
        }
    }

    write_state_snapshot(state)?;
    Ok(())
}

fn build_cancel_expired_artifact(
    state: &AppState,
    job_id_hex: &str,
) -> Result<(String, Option<String>, bool)> {
    let Some(chain) = state.chain.as_ref() else {
        return Ok(("UNAVAILABLE_NO_CHAIN_CONTEXT".to_string(), None, false));
    };
    let job_id = parse_hex32(job_id_hex)?;
    let (tx, sig, submitted) = build_cancel_expired_job_tx_base64(
        chain,
        job_id,
        chain.payer.pubkey(),
        state.chain_auto_submit,
    )?;
    Ok((tx, sig, submitted))
}

const ANCHOR_DISCRIMINATOR_LEN: usize = 8;
const JOB_ACCOUNT_MIN_LEN: usize = ANCHOR_DISCRIMINATOR_LEN + 303;
const JOB_ID_OFFSET_FROM_ANCHOR: usize = 0;
const JOB_ESCROW_LAMPORTS_OFFSET_FROM_ANCHOR: usize = 64;
const JOB_BUNDLE_HASH_OFFSET_FROM_ANCHOR: usize = 72;
const JOB_RUNTIME_ID_OFFSET_FROM_ANCHOR: usize = 104;
const JOB_MAX_MEMORY_OFFSET_FROM_ANCHOR: usize = 136;
const JOB_MAX_INSTRUCTIONS_OFFSET_FROM_ANCHOR: usize = 140;
const JOB_COMMITTEE_SIZE_OFFSET_FROM_ANCHOR: usize = 148;
const JOB_QUORUM_OFFSET_FROM_ANCHOR: usize = 149;
const JOB_CREATED_SLOT_OFFSET_FROM_ANCHOR: usize = 150;
const JOB_DEADLINE_SLOT_OFFSET_FROM_ANCHOR: usize = 158;
const JOB_STATUS_OFFSET_FROM_ANCHOR: usize = 302;

#[derive(Debug, Clone)]
struct OnchainJobView {
    job_id: [u8; 32],
    bundle_hash: [u8; 32],
    runtime_id: [u8; 32],
    max_memory_bytes: u32,
    max_instructions: u64,
    escrow_lamports: u64,
    committee_size: u8,
    quorum: u8,
    created_slot: u64,
    deadline_slot: u64,
    status: u8,
}

fn discover_posted_jobs_from_chain(state: &AppState) -> Result<()> {
    let Some(chain) = state.chain.as_ref() else {
        return Ok(());
    };
    let accounts = chain
        .rpc
        .get_program_accounts(&chain.program_id)
        .context("failed to fetch program accounts for discovery")?;
    if accounts.is_empty() {
        return Ok(());
    }

    let mut changed = false;
    for (addr, account) in accounts {
        let Some(view) = parse_onchain_job_view(&account.data) else {
            continue;
        };
        if !is_valid_job_account_address(&chain.program_id, &addr, &view.job_id) {
            continue;
        }
        if view.status != 0 {
            continue;
        }
        if seed_discovered_posted_job(state, &view)? {
            changed = true;
        }
    }

    if changed {
        write_state_snapshot(state)?;
    }
    Ok(())
}

fn seed_discovered_posted_job(state: &AppState, view: &OnchainJobView) -> Result<bool> {
    let job_id_hex = hex::encode(view.job_id);
    if view.committee_size != 3 || view.quorum != 2 {
        tracing::warn!(
            job_id = %job_id_hex,
            committee_size = view.committee_size,
            quorum = view.quorum,
            "skipping discovered posted job: unsupported committee/quorum for MVP policy"
        );
        return Ok(false);
    }
    let bundle_hash_hex = hex::encode(view.bundle_hash);
    if !bundle_path(state, &bundle_hash_hex).exists() {
        tracing::warn!(
            job_id = %job_id_hex,
            bundle_hash = %bundle_hash_hex,
            "skipping discovered posted job because bundle is unavailable in local store"
        );
        return Ok(false);
    }
    let already_tracked = {
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum.contains_key(&job_id_hex)
    };
    if already_tracked {
        return Ok(false);
    }
    let runtime_id_hex = hex::encode(view.runtime_id);
    let committee_workers = select_committee_workers(
        state,
        &runtime_id_hex,
        &job_id_hex,
        usize::from(view.committee_size.max(1)),
    );
    let quorum_target = usize::from(view.quorum.max(1));
    if committee_workers.len() < quorum_target {
        tracing::warn!(
            job_id = %job_id_hex,
            runtime_id = %runtime_id_hex,
            have = committee_workers.len(),
            need = quorum_target,
            "skipping discovered posted job due to insufficient eligible workers"
        );
        return Ok(false);
    }

    let now = now_unix_seconds();
    let mut queued: Vec<(String, QueuedAssignment)> = Vec::with_capacity(committee_workers.len());
    for worker_pubkey in &committee_workers {
        let mut assignment = QueuedAssignment {
            job_id: job_id_hex.clone(),
            bundle_hash: bundle_hash_hex.clone(),
            bundle_url: format!("{}/bundle/{}", state.public_base_url, bundle_hash_hex),
            runtime_id: runtime_id_hex.clone(),
            abi_version: edgerun_types::BUNDLE_ABI_CURRENT,
            limits: edgerun_types::Limits {
                max_memory_bytes: view.max_memory_bytes,
                max_instructions: view.max_instructions,
            },
            escrow_lamports: view.escrow_lamports,
            policy_signer_pubkey: hex::encode(state.policy_signing_key.verifying_key().as_bytes()),
            policy_signature: String::new(),
            policy_key_id: state.policy_key_id.clone(),
            policy_version: state.policy_version,
            policy_valid_after_unix_s: now,
            policy_valid_until_unix_s: now.saturating_add(state.policy_ttl_secs),
        };
        assignment.policy_signature =
            sign_assignment_policy(&state.policy_signing_key, &assignment);
        queued.push((worker_pubkey.clone(), assignment));
    }
    {
        let mut assignments = state.assignments.lock().expect("lock poisoned");
        for (worker_pubkey, assignment) in queued {
            assignments
                .entry(worker_pubkey)
                .or_default()
                .push(assignment);
        }
    }
    {
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum.insert(
            job_id_hex.clone(),
            JobQuorumState {
                expected_bundle_hash: hex::encode(view.bundle_hash),
                expected_runtime_id: runtime_id_hex.clone(),
                committee_workers: committee_workers.clone(),
                committee_size: committee_workers.len().min(u8::MAX as usize) as u8,
                quorum: quorum_target.min(u8::MAX as usize) as u8,
                assign_tx: None,
                assign_sig: None,
                assign_submitted: false,
                quorum_reached: false,
                winning_output_hash: None,
                winning_workers: Vec::new(),
                finalize_triggered: false,
                finalize_tx: None,
                finalize_sig: None,
                finalize_submitted: false,
                cancel_triggered: false,
                cancel_tx: None,
                cancel_sig: None,
                cancel_submitted: false,
                onchain_status: Some("posted".to_string()),
                onchain_last_observed_slot: None,
                onchain_last_update_unix_s: Some(now),
                onchain_deadline_slot: Some(view.deadline_slot),
                created_at_unix_s: now,
                quorum_reached_at_unix_s: None,
            },
        );
    }
    let (assign_workers_tx, assign_workers_sig, assign_workers_submitted) =
        build_assign_workers_artifact(state, &job_id_hex)?;
    {
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        if let Some(quorum_state) = job_quorum.get_mut(&job_id_hex) {
            quorum_state.assign_tx = assign_workers_tx;
            quorum_state.assign_sig = assign_workers_sig;
            quorum_state.assign_submitted = assign_workers_submitted;
        }
    }
    touch_job_last_update(state, &job_id_hex);
    tracing::info!(
        job_id = %job_id_hex,
        created_slot = view.created_slot,
        deadline_slot = view.deadline_slot,
        "discovered posted on-chain job and queued assignments"
    );
    Ok(true)
}

fn reconcile_onchain_job_statuses(state: &AppState) -> Result<()> {
    let Some(chain) = state.chain.as_ref() else {
        return Ok(());
    };
    let job_ids = {
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum.keys().cloned().collect::<Vec<_>>()
    };
    if job_ids.is_empty() {
        return Ok(());
    }

    let mut changed = false;
    for job_id_hex in job_ids {
        let Ok(job_id) = parse_hex32(&job_id_hex) else {
            continue;
        };
        let (status, observed_slot) = match fetch_onchain_job_status(chain, job_id) {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(err) => {
                tracing::warn!(error = %err, job_id = %job_id_hex, "failed to fetch on-chain job account");
                continue;
            }
        };

        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        let Some(entry) = job_quorum.get_mut(&job_id_hex) else {
            continue;
        };
        let status_label = onchain_status_label(status).to_string();
        if entry.onchain_status.as_deref() != Some(status_label.as_str()) {
            entry.onchain_status = Some(status_label);
            changed = true;
        }
        if entry.onchain_last_observed_slot != Some(observed_slot) {
            entry.onchain_last_observed_slot = Some(observed_slot);
            changed = true;
        }
        entry.onchain_last_update_unix_s = Some(now_unix_seconds());

        if status >= 1 && !entry.assign_submitted {
            entry.assign_submitted = true;
            changed = true;
        }
        if status == 2 && !entry.finalize_submitted {
            entry.finalize_submitted = true;
            entry.finalize_triggered = true;
            changed = true;
        }
        if status == 3 && !entry.cancel_submitted {
            entry.cancel_submitted = true;
            entry.cancel_triggered = true;
            changed = true;
        }
    }
    if changed {
        write_state_snapshot(state)?;
    }
    Ok(())
}

fn fetch_onchain_job_status(chain: &ChainContext, job_id: [u8; 32]) -> Result<Option<(u8, u64)>> {
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &chain.program_id);
    let resp = chain
        .rpc
        .get_account_with_commitment(&job_pda, CommitmentConfig::processed())
        .context("rpc get_account_with_commitment failed")?;
    let Some(account) = resp.value else {
        return Ok(None);
    };
    let Some(status) = parse_onchain_job_status(&account.data) else {
        return Ok(None);
    };
    Ok(Some((status, resp.context.slot)))
}

fn parse_onchain_job_status(data: &[u8]) -> Option<u8> {
    let status_offset = ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR;
    data.get(status_offset).copied()
}

fn parse_onchain_job_view(data: &[u8]) -> Option<OnchainJobView> {
    if data.len() < JOB_ACCOUNT_MIN_LEN {
        return None;
    }
    if !has_anchor_account_discriminator(data, "Job") {
        return None;
    }
    let job_id = read_fixed_32(data, JOB_ID_OFFSET_FROM_ANCHOR)?;
    let bundle_hash = read_fixed_32(data, JOB_BUNDLE_HASH_OFFSET_FROM_ANCHOR)?;
    let runtime_id = read_fixed_32(data, JOB_RUNTIME_ID_OFFSET_FROM_ANCHOR)?;
    let escrow_lamports = read_u64_from_anchor(data, JOB_ESCROW_LAMPORTS_OFFSET_FROM_ANCHOR)?;
    let max_memory_bytes = read_u32_from_anchor(data, JOB_MAX_MEMORY_OFFSET_FROM_ANCHOR)?;
    let max_instructions = read_u64_from_anchor(data, JOB_MAX_INSTRUCTIONS_OFFSET_FROM_ANCHOR)?;
    let committee_size = read_u8_from_anchor(data, JOB_COMMITTEE_SIZE_OFFSET_FROM_ANCHOR)?;
    let quorum = read_u8_from_anchor(data, JOB_QUORUM_OFFSET_FROM_ANCHOR)?;
    let created_slot = read_u64_from_anchor(data, JOB_CREATED_SLOT_OFFSET_FROM_ANCHOR)?;
    let deadline_slot = read_u64_from_anchor(data, JOB_DEADLINE_SLOT_OFFSET_FROM_ANCHOR)?;
    let status = read_u8_from_anchor(data, JOB_STATUS_OFFSET_FROM_ANCHOR)?;
    if status > 4 || committee_size == 0 || quorum == 0 {
        return None;
    }
    Some(OnchainJobView {
        job_id,
        bundle_hash,
        runtime_id,
        max_memory_bytes,
        max_instructions,
        escrow_lamports,
        committee_size,
        quorum,
        created_slot,
        deadline_slot,
        status,
    })
}

fn has_anchor_account_discriminator(data: &[u8], account_name: &str) -> bool {
    if data.len() < ANCHOR_DISCRIMINATOR_LEN {
        return false;
    }
    let expected = anchor_account_discriminator(account_name);
    data[..ANCHOR_DISCRIMINATOR_LEN] == expected
}

fn anchor_account_discriminator(account_name: &str) -> [u8; 8] {
    let preimage = format!("account:{account_name}");
    let h = hash(preimage.as_bytes());
    let mut out = [0_u8; 8];
    out.copy_from_slice(&h.to_bytes()[..8]);
    out
}

fn is_valid_job_account_address(program_id: &Pubkey, addr: &Pubkey, job_id: &[u8; 32]) -> bool {
    let (expected, _) = Pubkey::find_program_address(&[b"job", job_id], program_id);
    expected == *addr
}

fn read_u8_from_anchor(data: &[u8], offset_from_anchor: usize) -> Option<u8> {
    data.get(ANCHOR_DISCRIMINATOR_LEN + offset_from_anchor)
        .copied()
}

fn read_u32_from_anchor(data: &[u8], offset_from_anchor: usize) -> Option<u32> {
    let start = ANCHOR_DISCRIMINATOR_LEN + offset_from_anchor;
    let bytes = data.get(start..start + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64_from_anchor(data: &[u8], offset_from_anchor: usize) -> Option<u64> {
    let start = ANCHOR_DISCRIMINATOR_LEN + offset_from_anchor;
    let bytes = data.get(start..start + 8)?;
    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn read_fixed_32(data: &[u8], offset_from_anchor: usize) -> Option<[u8; 32]> {
    let start = ANCHOR_DISCRIMINATOR_LEN + offset_from_anchor;
    let slice = data.get(start..start + 32)?;
    let mut out = [0_u8; 32];
    out.copy_from_slice(slice);
    Some(out)
}

fn onchain_status_label(status: u8) -> &'static str {
    match status {
        0 => "posted",
        1 => "assigned",
        2 => "finalized",
        3 => "cancelled",
        4 => "slashed",
        _ => "unknown",
    }
}

fn build_finalize_trigger_payload(
    state: &AppState,
    job_id_hex: &str,
    winning_output_hash: &str,
    committee_workers: &[String],
    winning_workers: &[String],
) -> (String, Option<String>, bool) {
    build_finalize_trigger_payload_inner(
        state,
        job_id_hex,
        winning_output_hash,
        committee_workers,
        winning_workers,
    )
}

fn parse_hex32(value: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(value).context("value must be 32-byte hex")?;
    if bytes.len() != 32 {
        anyhow::bail!("value must be 32-byte hex");
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn verify_worker_message_signature(
    state: &AppState,
    worker_pubkey: &str,
    signature_b64: Option<&str>,
    message: &str,
) -> Result<bool, (StatusCode, String)> {
    let Some(signature_b64) = signature_b64 else {
        if state.require_worker_signatures {
            return Ok(false);
        }
        return Ok(true);
    };

    let worker_key = worker_pubkey.parse::<Pubkey>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "worker_pubkey must be base58 pubkey".to_string(),
        )
    })?;
    let worker_pk_bytes: [u8; 32] = worker_key.to_bytes();
    let worker_vk = VerifyingKey::from_bytes(&worker_pk_bytes).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid worker pubkey bytes".to_string(),
        )
    })?;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.as_bytes())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "signature must be base64".to_string(),
            )
        })?;
    let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "signature must decode to 64 bytes".to_string(),
        )
    })?;
    let signature = Signature::from_bytes(&sig_arr);
    let digest = edgerun_crypto::blake3_256(message.as_bytes());
    Ok(edgerun_crypto::verify(&worker_vk, &digest, &signature))
}

fn verify_result_attestation(
    state: &AppState,
    payload: &WorkerResultReport,
) -> Result<bool, (StatusCode, String)> {
    let Some(attestation_b64) = payload.attestation_sig.as_deref() else {
        if state.require_result_attestation {
            return Ok(false);
        }
        return Ok(true);
    };
    let worker = payload.worker_pubkey.parse::<Pubkey>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "worker_pubkey must be base58 pubkey".to_string(),
        )
    })?;
    let worker_bytes = worker.to_bytes();
    let worker_vk = VerifyingKey::from_bytes(&worker_bytes).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid worker pubkey bytes".to_string(),
        )
    })?;
    let job_id = parse_hex32(&payload.job_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "job_id must be 32-byte hex".to_string(),
        )
    })?;
    let output_hash = parse_hex32(&payload.output_hash).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "output_hash must be 32-byte hex".to_string(),
        )
    })?;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(attestation_b64.as_bytes())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "attestation_sig must be base64".to_string(),
            )
        })?;
    let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "attestation_sig must decode to 64 bytes".to_string(),
        )
    })?;
    let signature = Signature::from_bytes(&sig_arr);
    let message = build_worker_attestation_message(&job_id, &worker, &output_hash);
    Ok(edgerun_crypto::verify(&worker_vk, &message, &signature))
}

fn build_worker_attestation_message(
    job_id: &[u8; 32],
    worker: &Pubkey,
    output_hash: &[u8; 32],
) -> [u8; 98] {
    let mut msg = [0_u8; 98];
    msg[0..2].copy_from_slice(b"ER");
    msg[2..34].copy_from_slice(job_id);
    msg[34..66].copy_from_slice(&worker.to_bytes());
    msg[66..98].copy_from_slice(output_hash);
    msg
}

fn heartbeat_signing_message(payload: &HeartbeatRequest) -> String {
    let runtime_ids = payload.runtime_ids.join(",");
    let (max_concurrent, mem_bytes) = payload
        .capacity
        .as_ref()
        .map(|c| (c.max_concurrent.to_string(), c.mem_bytes.to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));
    format!(
        "heartbeat|{}|{}|{}|{}|{}",
        payload.worker_pubkey, runtime_ids, payload.version, max_concurrent, mem_bytes
    )
}

fn result_signing_message(payload: &WorkerResultReport) -> String {
    format!(
        "result|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.bundle_hash,
        payload.output_hash,
        payload.output_len
    )
}

fn failure_signing_message(payload: &WorkerFailureReport) -> String {
    format!(
        "failure|{}|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.bundle_hash,
        payload.phase,
        payload.error_code,
        payload.error_message
    )
}

fn replay_signing_message(payload: &WorkerReplayArtifactReport) -> String {
    let ok_flag = if payload.artifact.ok { "1" } else { "0" };
    let artifact_output_hash = payload.artifact.output_hash.clone().unwrap_or_default();
    format!(
        "replay|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.artifact.bundle_hash,
        ok_flag,
        artifact_output_hash
    )
}

fn load_retention_config() -> RetentionConfig {
    RetentionConfig {
        max_reports_per_job: read_env_usize("EDGERUN_SCHEDULER_MAX_REPORTS_PER_JOB", 32),
        max_failures_per_job: read_env_usize("EDGERUN_SCHEDULER_MAX_FAILURES_PER_JOB", 64),
        max_replays_per_job: read_env_usize("EDGERUN_SCHEDULER_MAX_REPLAYS_PER_JOB", 64),
        max_jobs_tracked: read_env_usize("EDGERUN_SCHEDULER_MAX_JOBS_TRACKED", 10_000),
    }
}

fn read_env_usize(key: &str, default_value: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default_value)
}

fn read_env_u32(key: &str, default_value: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default_value)
}

fn read_env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default_value)
}

fn read_env_bool(key: &str, default_value: bool) -> bool {
    std::env::var(key)
        .ok()
        .and_then(|v| match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default_value)
}

async fn housekeeping_loop(state: AppState) {
    let interval_secs = read_env_u64("EDGERUN_SCHEDULER_HOUSEKEEPING_INTERVAL_SECS", 5).max(1);
    loop {
        if let Err(err) = discover_posted_jobs_from_chain(&state) {
            tracing::warn!(error = %err, "housekeeping posted-job discovery failed");
        }
        if let Err(err) = evaluate_expired_jobs(&state) {
            tracing::warn!(error = %err, "housekeeping evaluate_expired_jobs failed");
        }
        if let Err(err) = reconcile_onchain_job_statuses(&state) {
            tracing::warn!(error = %err, "housekeeping on-chain reconciliation failed");
        }
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn touch_job_last_update(state: &AppState, job_id: &str) {
    let mut map = state.job_last_update.lock().expect("lock poisoned");
    map.insert(job_id.to_string(), now_unix_seconds());
}

fn is_duplicate_idempotency(existing: &str, incoming: &str) -> bool {
    !incoming.is_empty() && existing == incoming
}

fn trim_vec<T>(items: &mut Vec<T>, max_len: usize) {
    if items.len() > max_len {
        let excess = items.len() - max_len;
        items.drain(0..excess);
    }
}

fn enforce_history_retention(state: &AppState) {
    let limits = state.retention;
    let mut results = state.results.lock().expect("lock poisoned");
    let mut failures = state.failures.lock().expect("lock poisoned");
    let mut replay_artifacts = state.replay_artifacts.lock().expect("lock poisoned");
    let mut job_last_update = state.job_last_update.lock().expect("lock poisoned");
    let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");

    for entries in results.values_mut() {
        trim_vec(entries, limits.max_reports_per_job);
    }
    for entries in failures.values_mut() {
        trim_vec(entries, limits.max_failures_per_job);
    }
    for entries in replay_artifacts.values_mut() {
        trim_vec(entries, limits.max_replays_per_job);
    }

    let mut job_ids: HashSet<String> = HashSet::new();
    job_ids.extend(results.keys().cloned());
    job_ids.extend(failures.keys().cloned());
    job_ids.extend(replay_artifacts.keys().cloned());

    if job_ids.len() <= limits.max_jobs_tracked {
        return;
    }

    let mut ordered = job_ids.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        let a_ts = job_last_update.get(a).copied().unwrap_or(0);
        let b_ts = job_last_update.get(b).copied().unwrap_or(0);
        a_ts.cmp(&b_ts).then_with(|| a.cmp(b))
    });

    let to_prune = ordered.len() - limits.max_jobs_tracked;
    for job_id in ordered.iter().take(to_prune) {
        results.remove(job_id);
        failures.remove(job_id);
        replay_artifacts.remove(job_id);
        job_quorum.remove(job_id);
        job_last_update.remove(job_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        let data_dir =
            std::env::temp_dir().join(format!("edgerun-scheduler-tests-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(data_dir.join("bundles"));
        AppState {
            data_dir,
            public_base_url: "http://127.0.0.1:8080".to_string(),
            retention: RetentionConfig {
                max_reports_per_job: 32,
                max_failures_per_job: 32,
                max_replays_per_job: 32,
                max_jobs_tracked: 100,
            },
            assignments: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            failures: Arc::new(Mutex::new(HashMap::new())),
            replay_artifacts: Arc::new(Mutex::new(HashMap::new())),
            job_last_update: Arc::new(Mutex::new(HashMap::new())),
            worker_registry: Arc::new(Mutex::new(HashMap::new())),
            job_quorum: Arc::new(Mutex::new(HashMap::new())),
            policy_signing_key: SigningKey::from_bytes(&[1_u8; 32]),
            policy_key_id: "dev-key-1".to_string(),
            policy_version: 1,
            policy_ttl_secs: 300,
            committee_size: 3,
            quorum: 2,
            heartbeat_ttl_secs: 15,
            require_worker_signatures: false,
            require_result_attestation: false,
            quorum_requires_attestation: true,
            chain_auto_submit: false,
            job_timeout_secs: 60,
            chain: None,
        }
    }

    #[test]
    fn winning_output_reaches_quorum() {
        let reports = vec![
            WorkerResultReport {
                idempotency_key: "a".to_string(),
                worker_pubkey: "w1".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-a".to_string(),
                output_len: 10,
                attestation_sig: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "b".to_string(),
                worker_pubkey: "w2".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-a".to_string(),
                output_len: 12,
                attestation_sig: None,
                signature: None,
            },
        ];

        let winning = find_winning_output_hash(&reports, 2).expect("quorum expected");
        assert_eq!(winning.0, "out-a");
        assert_eq!(winning.1.len(), 2);
    }

    #[test]
    fn duplicate_worker_reports_do_not_count_twice() {
        let reports = vec![
            WorkerResultReport {
                idempotency_key: "a".to_string(),
                worker_pubkey: "w1".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-a".to_string(),
                output_len: 10,
                attestation_sig: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "b".to_string(),
                worker_pubkey: "w1".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-a".to_string(),
                output_len: 10,
                attestation_sig: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "c".to_string(),
                worker_pubkey: "w2".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-b".to_string(),
                output_len: 10,
                attestation_sig: None,
                signature: None,
            },
        ];

        assert!(find_winning_output_hash(&reports, 2).is_none());
    }

    #[test]
    fn committee_selection_uses_runtime_and_live_heartbeats() {
        let state = test_state();
        let now = now_unix_seconds();
        {
            let mut registry = state.worker_registry.lock().expect("lock poisoned");
            registry.insert(
                "w1".to_string(),
                WorkerRegistryEntry {
                    worker_pubkey: "w1".to_string(),
                    runtime_ids: vec!["r1".to_string()],
                    version: "1".to_string(),
                    max_concurrent: Some(1),
                    mem_bytes: Some(1024),
                    last_heartbeat_unix_s: now,
                },
            );
            registry.insert(
                "w2".to_string(),
                WorkerRegistryEntry {
                    worker_pubkey: "w2".to_string(),
                    runtime_ids: vec!["r1".to_string()],
                    version: "1".to_string(),
                    max_concurrent: Some(1),
                    mem_bytes: Some(1024),
                    last_heartbeat_unix_s: now,
                },
            );
            registry.insert(
                "stale".to_string(),
                WorkerRegistryEntry {
                    worker_pubkey: "stale".to_string(),
                    runtime_ids: vec!["r1".to_string()],
                    version: "1".to_string(),
                    max_concurrent: Some(1),
                    mem_bytes: Some(1024),
                    last_heartbeat_unix_s: now.saturating_sub(60),
                },
            );
            registry.insert(
                "wrong-runtime".to_string(),
                WorkerRegistryEntry {
                    worker_pubkey: "wrong-runtime".to_string(),
                    runtime_ids: vec!["r2".to_string()],
                    version: "1".to_string(),
                    max_concurrent: Some(1),
                    mem_bytes: Some(1024),
                    last_heartbeat_unix_s: now,
                },
            );
        }

        let selected = select_committee_workers(&state, "r1", "seed-1", 3);
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&"w1".to_string()));
        assert!(selected.contains(&"w2".to_string()));
    }

    #[test]
    fn verifies_worker_signature_when_present() {
        let state = test_state();
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let worker_pubkey = Pubkey::new_from_array(*signing_key.verifying_key().as_bytes());
        let message = "result|worker|job|bundle|output|10";
        let digest = edgerun_crypto::blake3_256(message.as_bytes());
        let sig = edgerun_crypto::sign(&signing_key, &digest);
        let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let ok = verify_worker_message_signature(
            &state,
            &worker_pubkey.to_string(),
            Some(&sig_b64),
            message,
        )
        .expect("verification should not error");
        assert!(ok);
    }

    #[test]
    fn expired_job_without_quorum_gets_cancel_artifact() {
        let state = test_state();
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                JobQuorumState {
                    expected_bundle_hash:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    expected_runtime_id: "r1".to_string(),
                    committee_workers: vec!["w1".to_string(), "w2".to_string(), "w3".to_string()],
                    committee_size: 3,
                    quorum: 2,
                    assign_tx: None,
                    assign_sig: None,
                    assign_submitted: false,
                    quorum_reached: false,
                    winning_output_hash: None,
                    winning_workers: Vec::new(),
                    finalize_triggered: false,
                    finalize_tx: None,
                    finalize_sig: None,
                    finalize_submitted: false,
                    cancel_triggered: false,
                    cancel_tx: None,
                    cancel_sig: None,
                    cancel_submitted: false,
                    onchain_status: None,
                    onchain_last_observed_slot: None,
                    onchain_last_update_unix_s: None,
                    onchain_deadline_slot: None,
                    created_at_unix_s: now_unix_seconds().saturating_sub(3600),
                    quorum_reached_at_unix_s: None,
                },
            );
        }

        evaluate_expired_jobs(&state).expect("expiry evaluation");
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        let entry = job_quorum
            .get("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .expect("entry exists");
        assert!(entry.cancel_triggered);
        assert!(entry.cancel_tx.is_some());
    }

    #[test]
    fn worker_must_be_in_committee() {
        let state = test_state();
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                "job-1".to_string(),
                JobQuorumState {
                    expected_bundle_hash:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    expected_runtime_id: "r1".to_string(),
                    committee_workers: vec!["w1".to_string(), "w2".to_string(), "w3".to_string()],
                    committee_size: 3,
                    quorum: 2,
                    assign_tx: None,
                    assign_sig: None,
                    assign_submitted: false,
                    quorum_reached: false,
                    winning_output_hash: None,
                    winning_workers: Vec::new(),
                    finalize_triggered: false,
                    finalize_tx: None,
                    finalize_sig: None,
                    finalize_submitted: false,
                    cancel_triggered: false,
                    cancel_tx: None,
                    cancel_sig: None,
                    cancel_submitted: false,
                    onchain_status: None,
                    onchain_last_observed_slot: None,
                    onchain_last_update_unix_s: None,
                    onchain_deadline_slot: None,
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }

        assert!(is_assigned_worker(&state, "job-1", "w2"));
        assert!(!is_assigned_worker(&state, "job-1", "intruder"));
        assert!(matches_expected_bundle_hash(
            &state,
            "job-1",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        ));
        assert!(!matches_expected_bundle_hash(&state, "job-1", "deadbeef"));
    }

    #[test]
    fn empty_idempotency_is_not_deduped() {
        assert!(!is_duplicate_idempotency("abc", ""));
        assert!(is_duplicate_idempotency("abc", "abc"));
    }

    #[test]
    fn parses_onchain_job_status_byte() {
        let mut data = vec![0_u8; ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR + 1];
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR] = 2;
        assert_eq!(parse_onchain_job_status(&data), Some(2));
    }

    #[test]
    fn parses_onchain_job_view_fields() {
        let mut data = vec![0_u8; ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR + 1];
        data[..ANCHOR_DISCRIMINATOR_LEN].copy_from_slice(&anchor_account_discriminator("Job"));
        let job_id = [0x11_u8; 32];
        let bundle_hash = [0x22_u8; 32];
        let runtime_id = [0x33_u8; 32];
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_ID_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_ID_OFFSET_FROM_ANCHOR + 32]
            .copy_from_slice(&job_id);
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_BUNDLE_HASH_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_BUNDLE_HASH_OFFSET_FROM_ANCHOR + 32]
            .copy_from_slice(&bundle_hash);
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_RUNTIME_ID_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_RUNTIME_ID_OFFSET_FROM_ANCHOR + 32]
            .copy_from_slice(&runtime_id);
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_ESCROW_LAMPORTS_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_ESCROW_LAMPORTS_OFFSET_FROM_ANCHOR + 8]
            .copy_from_slice(&123_u64.to_le_bytes());
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_MAX_MEMORY_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_MAX_MEMORY_OFFSET_FROM_ANCHOR + 4]
            .copy_from_slice(&456_u32.to_le_bytes());
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_MAX_INSTRUCTIONS_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_MAX_INSTRUCTIONS_OFFSET_FROM_ANCHOR + 8]
            .copy_from_slice(&789_u64.to_le_bytes());
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_COMMITTEE_SIZE_OFFSET_FROM_ANCHOR] = 3;
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_QUORUM_OFFSET_FROM_ANCHOR] = 2;
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_CREATED_SLOT_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_CREATED_SLOT_OFFSET_FROM_ANCHOR + 8]
            .copy_from_slice(&900_u64.to_le_bytes());
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_DEADLINE_SLOT_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_DEADLINE_SLOT_OFFSET_FROM_ANCHOR + 8]
            .copy_from_slice(&901_u64.to_le_bytes());
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR] = 0;

        let parsed = parse_onchain_job_view(&data).expect("parse");
        assert_eq!(parsed.job_id, job_id);
        assert_eq!(parsed.bundle_hash, bundle_hash);
        assert_eq!(parsed.runtime_id, runtime_id);
        assert_eq!(parsed.escrow_lamports, 123);
        assert_eq!(parsed.max_memory_bytes, 456);
        assert_eq!(parsed.max_instructions, 789);
        assert_eq!(parsed.committee_size, 3);
        assert_eq!(parsed.quorum, 2);
        assert_eq!(parsed.created_slot, 900);
        assert_eq!(parsed.deadline_slot, 901);
        assert_eq!(parsed.status, 0);
    }

    #[test]
    fn rejects_non_job_discriminator_in_job_parser() {
        let mut data = vec![0_u8; ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR + 1];
        data[..ANCHOR_DISCRIMINATOR_LEN]
            .copy_from_slice(&anchor_account_discriminator("WorkerStake"));
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_COMMITTEE_SIZE_OFFSET_FROM_ANCHOR] = 3;
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_QUORUM_OFFSET_FROM_ANCHOR] = 2;
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR] = 0;
        assert!(parse_onchain_job_view(&data).is_none());
    }

    #[test]
    fn validates_job_pda_against_job_id() {
        let program_id = Pubkey::new_unique();
        let job_id = [0x55_u8; 32];
        let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &program_id);
        assert!(is_valid_job_account_address(&program_id, &job_pda, &job_id));
        assert!(!is_valid_job_account_address(
            &program_id,
            &Pubkey::new_unique(),
            &job_id
        ));
    }

    #[test]
    fn discovered_job_lifecycle_generates_assign_and_finalize_artifacts() {
        let mut state = test_state();
        state.quorum_requires_attestation = false;
        let state = state;
        let now = now_unix_seconds();
        let runtime_id = [0_u8; 32];
        let runtime_hex = hex::encode(runtime_id);
        {
            let mut registry = state.worker_registry.lock().expect("lock poisoned");
            for worker in ["w1", "w2", "w3"] {
                registry.insert(
                    worker.to_string(),
                    WorkerRegistryEntry {
                        worker_pubkey: worker.to_string(),
                        runtime_ids: vec![runtime_hex.clone()],
                        version: "1".to_string(),
                        max_concurrent: Some(1),
                        mem_bytes: Some(1024),
                        last_heartbeat_unix_s: now,
                    },
                );
            }
        }

        let view = OnchainJobView {
            job_id: [0x44_u8; 32],
            bundle_hash: [0x55_u8; 32],
            runtime_id,
            max_memory_bytes: 64 * 1024 * 1024,
            max_instructions: 1_000_000,
            escrow_lamports: 1_000,
            committee_size: 3,
            quorum: 2,
            created_slot: 10,
            deadline_slot: 20,
            status: 0,
        };
        let bundle_hash_hex = hex::encode(view.bundle_hash);
        std::fs::write(bundle_path(&state, &bundle_hash_hex), b"bundle").expect("write bundle");
        let inserted = seed_discovered_posted_job(&state, &view).expect("seed");
        assert!(inserted);

        let job_id_hex = hex::encode(view.job_id);
        let queued_assignments = state.assignments.lock().expect("lock poisoned");
        let queued_total = queued_assignments.values().map(Vec::len).sum::<usize>();
        assert_eq!(queued_total, 3);
        drop(queued_assignments);

        {
            let mut results = state.results.lock().expect("lock poisoned");
            let entries = results.entry(job_id_hex.clone()).or_default();
            entries.push(WorkerResultReport {
                idempotency_key: "r1".to_string(),
                worker_pubkey: "w1".to_string(),
                job_id: job_id_hex.clone(),
                bundle_hash: bundle_hash_hex.clone(),
                output_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                output_len: 10,
                attestation_sig: None,
                signature: None,
            });
            entries.push(WorkerResultReport {
                idempotency_key: "r2".to_string(),
                worker_pubkey: "w2".to_string(),
                job_id: job_id_hex.clone(),
                bundle_hash: bundle_hash_hex,
                output_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
                output_len: 10,
                attestation_sig: None,
                signature: None,
            });
        }

        let reached = recompute_job_quorum(&state, &job_id_hex).expect("recompute");
        assert!(reached);
        let jq = state.job_quorum.lock().expect("lock poisoned");
        let entry = jq.get(&job_id_hex).expect("quorum entry");
        assert!(entry.finalize_triggered);
        assert!(entry
            .finalize_tx
            .as_deref()
            .unwrap_or_default()
            .starts_with("UNAVAILABLE_FINALIZE_"));
    }

    #[test]
    fn quorum_requires_attestation_by_default() {
        let state = test_state();
        let job_id = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string();
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                job_id.clone(),
                JobQuorumState {
                    expected_bundle_hash:
                        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                            .to_string(),
                    expected_runtime_id: "r1".to_string(),
                    committee_workers: vec!["w1".to_string(), "w2".to_string(), "w3".to_string()],
                    committee_size: 3,
                    quorum: 2,
                    assign_tx: None,
                    assign_sig: None,
                    assign_submitted: false,
                    quorum_reached: false,
                    winning_output_hash: None,
                    winning_workers: Vec::new(),
                    finalize_triggered: false,
                    finalize_tx: None,
                    finalize_sig: None,
                    finalize_submitted: false,
                    cancel_triggered: false,
                    cancel_tx: None,
                    cancel_sig: None,
                    cancel_submitted: false,
                    onchain_status: None,
                    onchain_last_observed_slot: None,
                    onchain_last_update_unix_s: None,
                    onchain_deadline_slot: None,
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }
        {
            let mut results = state.results.lock().expect("lock poisoned");
            let entries = results.entry(job_id.clone()).or_default();
            entries.push(WorkerResultReport {
                idempotency_key: "q1".to_string(),
                worker_pubkey: "w1".to_string(),
                job_id: job_id.clone(),
                bundle_hash: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                    .to_string(),
                output_hash: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                    .to_string(),
                output_len: 1,
                attestation_sig: None,
                signature: None,
            });
            entries.push(WorkerResultReport {
                idempotency_key: "q2".to_string(),
                worker_pubkey: "w2".to_string(),
                job_id: job_id.clone(),
                bundle_hash: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                    .to_string(),
                output_hash: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                    .to_string(),
                output_len: 1,
                attestation_sig: None,
                signature: None,
            });
        }
        let reached = recompute_job_quorum(&state, &job_id).expect("recompute");
        assert!(!reached);
    }

    #[test]
    fn timeout_lifecycle_generates_cancel_artifact_without_chain() {
        let mut state = test_state();
        state.job_timeout_secs = 1;
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                JobQuorumState {
                    expected_bundle_hash:
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    expected_runtime_id: "r1".to_string(),
                    committee_workers: vec!["w1".to_string(), "w2".to_string(), "w3".to_string()],
                    committee_size: 3,
                    quorum: 2,
                    assign_tx: None,
                    assign_sig: None,
                    assign_submitted: false,
                    quorum_reached: false,
                    winning_output_hash: None,
                    winning_workers: Vec::new(),
                    finalize_triggered: false,
                    finalize_tx: None,
                    finalize_sig: None,
                    finalize_submitted: false,
                    cancel_triggered: false,
                    cancel_tx: None,
                    cancel_sig: None,
                    cancel_submitted: false,
                    onchain_status: None,
                    onchain_last_observed_slot: None,
                    onchain_last_update_unix_s: None,
                    onchain_deadline_slot: None,
                    created_at_unix_s: now_unix_seconds().saturating_sub(60),
                    quorum_reached_at_unix_s: None,
                },
            );
        }
        evaluate_expired_jobs(&state).expect("expire");
        let jq = state.job_quorum.lock().expect("lock poisoned");
        let entry = jq
            .get("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .expect("entry");
        assert!(entry.cancel_triggered);
        assert_eq!(
            entry.cancel_tx.as_deref(),
            Some("UNAVAILABLE_NO_CHAIN_CONTEXT")
        );
    }

    #[test]
    fn discovered_job_without_local_bundle_is_skipped() {
        let state = test_state();
        let now = now_unix_seconds();
        let runtime_id = [0_u8; 32];
        let runtime_hex = hex::encode(runtime_id);
        {
            let mut registry = state.worker_registry.lock().expect("lock poisoned");
            for worker in ["w1", "w2", "w3"] {
                registry.insert(
                    worker.to_string(),
                    WorkerRegistryEntry {
                        worker_pubkey: worker.to_string(),
                        runtime_ids: vec![runtime_hex.clone()],
                        version: "1".to_string(),
                        max_concurrent: Some(1),
                        mem_bytes: Some(1024),
                        last_heartbeat_unix_s: now,
                    },
                );
            }
        }
        let view = OnchainJobView {
            job_id: [0x66_u8; 32],
            bundle_hash: [0x77_u8; 32],
            runtime_id,
            max_memory_bytes: 1,
            max_instructions: 1,
            escrow_lamports: 1,
            committee_size: 3,
            quorum: 2,
            created_slot: 10,
            deadline_slot: 20,
            status: 0,
        };
        let inserted = seed_discovered_posted_job(&state, &view).expect("seed");
        assert!(!inserted);
        let job_id_hex = hex::encode(view.job_id);
        let jq = state.job_quorum.lock().expect("lock poisoned");
        assert!(!jq.contains_key(&job_id_hex));
    }

    #[test]
    fn discovered_job_with_non_mvp_committee_policy_is_skipped() {
        let state = test_state();
        let now = now_unix_seconds();
        let runtime_id = [0_u8; 32];
        let runtime_hex = hex::encode(runtime_id);
        {
            let mut registry = state.worker_registry.lock().expect("lock poisoned");
            for worker in ["w1", "w2", "w3"] {
                registry.insert(
                    worker.to_string(),
                    WorkerRegistryEntry {
                        worker_pubkey: worker.to_string(),
                        runtime_ids: vec![runtime_hex.clone()],
                        version: "1".to_string(),
                        max_concurrent: Some(1),
                        mem_bytes: Some(1024),
                        last_heartbeat_unix_s: now,
                    },
                );
            }
        }
        let view = OnchainJobView {
            job_id: [0x68_u8; 32],
            bundle_hash: [0x69_u8; 32],
            runtime_id,
            max_memory_bytes: 1,
            max_instructions: 1,
            escrow_lamports: 1,
            committee_size: 5,
            quorum: 3,
            created_slot: 10,
            deadline_slot: 20,
            status: 0,
        };
        let bundle_hash_hex = hex::encode(view.bundle_hash);
        std::fs::write(bundle_path(&state, &bundle_hash_hex), b"bundle").expect("write bundle");
        let inserted = seed_discovered_posted_job(&state, &view).expect("seed");
        assert!(!inserted);
    }

    #[test]
    fn verifies_result_attestation() {
        let state = test_state();
        let signing_key = SigningKey::from_bytes(&[9_u8; 32]);
        let worker = Pubkey::new_from_array(*signing_key.verifying_key().as_bytes());
        let job_id = [1_u8; 32];
        let output_hash = [2_u8; 32];
        let message = build_worker_attestation_message(&job_id, &worker, &output_hash);
        let sig = edgerun_crypto::sign(&signing_key, &message);
        let attestation_sig = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let payload = WorkerResultReport {
            idempotency_key: "ik".to_string(),
            worker_pubkey: worker.to_string(),
            job_id: hex::encode(job_id),
            bundle_hash: "b1".to_string(),
            output_hash: hex::encode(output_hash),
            output_len: 7,
            attestation_sig: Some(attestation_sig),
            signature: None,
        };

        let ok = verify_result_attestation(&state, &payload).expect("attestation verify");
        assert!(ok);
    }
}
