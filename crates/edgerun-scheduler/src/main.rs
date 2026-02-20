#![allow(deprecated)]

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
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
use ed25519_dalek::{Signature, SigningKey};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
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
    policy_signing_key: SigningKey,
    policy_key_id: String,
    policy_version: u32,
    policy_ttl_secs: u64,
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
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
struct PersistedState {
    assignments: HashMap<String, Vec<QueuedAssignment>>,
    results: HashMap<String, Vec<WorkerResultReport>>,
    failures: HashMap<String, Vec<WorkerFailureReport>>,
    replay_artifacts: HashMap<String, Vec<WorkerReplayArtifactReport>>,
    job_last_update: HashMap<String, u64>,
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
}

#[derive(Debug, Serialize)]
struct HeartbeatResponse {
    ok: bool,
    next_poll_ms: u64,
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
}

#[derive(Debug, Serialize)]
struct JobStatusResponse {
    job_id: String,
    reports: Vec<WorkerResultReport>,
    failures: Vec<WorkerFailureReport>,
    replay_artifacts: Vec<WorkerReplayArtifactReport>,
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
            tracing::warn!(error = %err, "chain context unavailable; post_job_tx will be placeholder");
            None
        }
    };

    let addr: SocketAddr = std::env::var("EDGERUN_SCHEDULER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .context("invalid EDGERUN_SCHEDULER_ADDR")?;

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
        policy_signing_key: load_policy_signing_key()?,
        policy_key_id: std::env::var("EDGERUN_SCHEDULER_POLICY_KEY_ID")
            .unwrap_or_else(|_| default_policy_key_id()),
        policy_version: read_env_u32("EDGERUN_SCHEDULER_POLICY_VERSION", default_policy_version()),
        policy_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_POLICY_TTL_SECS", 300),
        chain,
    };
    enforce_history_retention(&state);

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
    if entries
        .iter()
        .any(|existing| existing.idempotency_key == payload.idempotency_key)
    {
        drop(results);
        return Ok(Json(serde_json::json!({ "ok": true, "duplicate": true })));
    }
    entries.push(payload);
    drop(results);
    touch_job_last_update(&state, &job_id);
    enforce_history_retention(&state);

    write_state_snapshot(&state).map_err(internal_err)?;
    Ok(Json(serde_json::json!({ "ok": true, "duplicate": false })))
}

async fn worker_failure(
    State(state): State<AppState>,
    Json(payload): Json<WorkerFailureReport>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
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
    if entries
        .iter()
        .any(|existing| existing.idempotency_key == payload.idempotency_key)
    {
        drop(failures);
        return Ok(Json(serde_json::json!({ "ok": true, "duplicate": true })));
    }
    entries.push(payload);
    drop(failures);
    touch_job_last_update(&state, &job_id);
    enforce_history_retention(&state);

    write_state_snapshot(&state).map_err(internal_err)?;
    Ok(Json(serde_json::json!({ "ok": true, "duplicate": false })))
}

async fn worker_replay_artifact(
    State(state): State<AppState>,
    Json(payload): Json<WorkerReplayArtifactReport>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
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
    if entries
        .iter()
        .any(|existing| existing.idempotency_key == payload.idempotency_key)
    {
        drop(replay_artifacts);
        return Ok(Json(serde_json::json!({ "ok": true, "duplicate": true })));
    }
    entries.push(payload);
    drop(replay_artifacts);
    touch_job_last_update(&state, &job_id);
    enforce_history_retention(&state);

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

    let bundle_path = bundle_path(&state, &bundle_hash_hex);
    std::fs::write(&bundle_path, &bundle_payload_bytes).map_err(internal_err)?;

    if let Some(worker_pubkey) = payload.assignment_worker_pubkey.as_ref() {
        let now = now_unix_seconds();
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
        let mut assignments = state.assignments.lock().expect("lock poisoned");
        assignments
            .entry(worker_pubkey.clone())
            .or_default()
            .push(assignment);
        drop(assignments);
        write_state_snapshot(&state).map_err(internal_err)?;
    }

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
    Json(JobStatusResponse {
        job_id,
        reports,
        failures,
        replay_artifacts,
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
    let payer = read_keypair_file(&wallet_path).map_err(|e| {
        anyhow::anyhow!("failed to read EDGERUN_CHAIN_WALLET {wallet_path}: {e}")
    })?;
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
    };
    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    std::fs::write(state.data_dir.join("state.json"), bytes)?;
    Ok(())
}

fn internal_err<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn default_abi_version() -> u8 {
    1
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
        job_last_update.remove(job_id);
    }
}
