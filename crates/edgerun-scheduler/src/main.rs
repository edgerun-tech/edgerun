// SPDX-License-Identifier: LicenseRef-Edgerun-Proprietary
#![allow(deprecated)]

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Query,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcProgramAccountsConfig;
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use tokio::sync::mpsc;

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
    enable_slash_artifacts: bool,
    require_client_signatures: bool,
    client_signature_max_age_secs: u64,
    chain_auto_submit: bool,
    job_timeout_secs: u64,
    session_ttl_secs: u64,
    require_policy_session: bool,
    policy_session_bootstrap_token: Option<String>,
    policy_session_shared: bool,
    policy_session_lock_path: PathBuf,
    sessions: Arc<Mutex<HashMap<String, edgerun_hwvault_primitives::session::SessionState>>>,
    policy_nonces: Arc<Mutex<HashMap<String, u64>>>,
    policy_session_state_path: PathBuf,
    policy_audit_path: PathBuf,
    trust_policy: Arc<Mutex<edgerun_types::SyncTrustPolicy>>,
    trust_policy_path: PathBuf,
    attestation_policy: Arc<Mutex<edgerun_types::AttestationPolicy>>,
    attestation_policy_path: PathBuf,
    route_shared_state_path: Option<PathBuf>,
    device_routes: Arc<Mutex<HashMap<String, DeviceRouteEntry>>>,
    route_challenges: Arc<Mutex<HashMap<String, u64>>>,
    route_heartbeat_tokens: Arc<Mutex<HashMap<String, RouteHeartbeatToken>>>,
    signal_peers: Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
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
    attestation_claim: Option<edgerun_types::AttestationClaim>,
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
    #[serde(default)]
    slash_artifacts: Vec<SlashWorkerArtifact>,
    created_at_unix_s: u64,
    quorum_reached_at_unix_s: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SlashWorkerArtifact {
    worker_pubkey: String,
    tx: String,
    sig: Option<String>,
    submitted: bool,
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
    trust_policy: edgerun_types::SyncTrustPolicy,
    attestation_policy: edgerun_types::AttestationPolicy,
}

#[derive(Debug, Deserialize)]
struct SessionCreateRequest {
    #[serde(default)]
    bound_origin: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionCreateResponse {
    token: String,
    session_key: String,
    ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
struct SessionRotateRequest {
    #[serde(default)]
    bound_origin: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionRotateResponse {
    token: String,
    session_key: String,
    ttl_secs: u64,
}

#[derive(Debug, Deserialize)]
struct SessionInvalidateRequest {
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionInvalidateResponse {
    ok: bool,
}

#[derive(Debug, Deserialize)]
struct PolicyAuditQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct PolicyAuditListResponse {
    events: Vec<edgerun_hwvault_primitives::audit::PolicyAuditEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TrustPolicySetRequest {
    profile: String,
}

#[derive(Debug, Serialize)]
struct TrustPolicyResponse {
    policy: edgerun_types::SyncTrustPolicy,
}

#[derive(Debug, Deserialize, Serialize)]
struct AttestationPolicySetRequest {
    required: bool,
    max_age_secs: u64,
    #[serde(default)]
    allowed_measurements: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AttestationPolicyResponse {
    policy: edgerun_types::AttestationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedPolicySessionState {
    #[serde(default)]
    sessions: HashMap<String, edgerun_hwvault_primitives::session::SessionState>,
    #[serde(default)]
    nonces: HashMap<String, u64>,
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
    client_pubkey: Option<String>,
    client_signed_at_unix_s: Option<u64>,
    client_signature: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceRouteEntry {
    device_id: String,
    owner_pubkey: String,
    reachable_urls: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    relay_session_id: Option<String>,
    online: bool,
    last_seen_unix_s: u64,
    expires_at_unix_s: u64,
    updated_at_unix_s: u64,
}

#[derive(Debug, Deserialize)]
struct RouteRegisterRequest {
    device_id: String,
    owner_pubkey: String,
    reachable_urls: Vec<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    relay_session_id: Option<String>,
    #[serde(default)]
    ttl_secs: Option<u64>,
    challenge_nonce: String,
    signed_at_unix_s: u64,
    signature: String,
}

#[derive(Debug, Serialize)]
struct RouteRegisterResponse {
    ok: bool,
    device_id: String,
    expires_at_unix_s: u64,
    heartbeat_token: String,
}

#[derive(Debug, Deserialize)]
struct RouteChallengeRequest {
    device_id: String,
}

#[derive(Debug, Serialize)]
struct RouteChallengeResponse {
    nonce: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Deserialize)]
struct RouteHeartbeatRequest {
    device_id: String,
    token: String,
    #[serde(default)]
    ttl_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
struct RouteHeartbeatResponse {
    ok: bool,
    device_id: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Serialize)]
struct RouteResolveResponse {
    ok: bool,
    found: bool,
    route: Option<DeviceRouteEntry>,
}

#[derive(Debug, Serialize)]
struct OwnerRoutesResponse {
    ok: bool,
    owner_pubkey: String,
    devices: Vec<DeviceRouteEntry>,
}

#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
struct RouteHeartbeatToken {
    device_id: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedRouteState {
    #[serde(default)]
    device_routes: HashMap<String, DeviceRouteEntry>,
    #[serde(default)]
    route_challenges: HashMap<String, u64>,
    #[serde(default)]
    route_heartbeat_tokens: HashMap<String, RouteHeartbeatToken>,
}

#[derive(Debug, Deserialize)]
struct WebRtcSignalConnectQuery {
    device_id: String,
}

#[derive(Debug, Deserialize)]
struct WebRtcSignalClientMessage {
    #[serde(default)]
    to_device_id: String,
    #[serde(default)]
    to_owner_pubkey: String,
    kind: String,
    #[serde(default)]
    sdp: Option<String>,
    #[serde(default)]
    candidate: Option<String>,
    #[serde(default)]
    sdp_mid: Option<String>,
    #[serde(default)]
    sdp_mline_index: Option<u16>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct WebRtcSignalServerMessage {
    from_device_id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sdp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sdp_mid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sdp_mline_index: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

struct PostJobArgs {
    client: Pubkey,
    job_id: [u8; 32],
    bundle_hash: [u8; 32],
    runtime_id: [u8; 32],
    runtime_proof: Vec<[u8; 32]>,
    max_memory_bytes: u32,
    max_instructions: u64,
    escrow_lamports: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    edgerun_observability::init_service("edgerun-scheduler")?;

    let data_dir = std::env::var("EDGERUN_SCHEDULER_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".edgerun-scheduler-data"));
    std::fs::create_dir_all(data_dir.join("bundles"))?;
    let route_shared_state_path = std::env::var("EDGERUN_SCHEDULER_ROUTE_STATE_PATH")
        .ok()
        .map(PathBuf::from);
    if let Some(path) = route_shared_state_path.as_ref() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

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
    let trust_policy_path = data_dir.join("trust-policy.json");
    let trust_policy = load_trust_policy(&trust_policy_path).unwrap_or_default();
    let attestation_policy_path = data_dir.join("attestation-policy.json");
    let attestation_policy = load_attestation_policy(&attestation_policy_path).unwrap_or_default();
    let policy_session_state_path = data_dir.join("policy-session-state.json");
    let loaded_policy_session_state =
        load_policy_session_state(&policy_session_state_path).unwrap_or_default();

    let state = AppState {
        data_dir: data_dir.clone(),
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
        enable_slash_artifacts: read_env_bool("EDGERUN_SCHEDULER_ENABLE_SLASH_ARTIFACTS", true),
        require_client_signatures: read_env_bool(
            "EDGERUN_SCHEDULER_REQUIRE_CLIENT_SIGNATURES",
            false,
        ),
        client_signature_max_age_secs: read_env_u64(
            "EDGERUN_SCHEDULER_CLIENT_SIGNATURE_MAX_AGE_SECS",
            300,
        ),
        chain_auto_submit: read_env_bool("EDGERUN_SCHEDULER_CHAIN_AUTO_SUBMIT", false),
        job_timeout_secs: read_env_u64("EDGERUN_SCHEDULER_JOB_TIMEOUT_SECS", 60),
        session_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_SESSION_TTL_SECS", 15 * 60),
        require_policy_session: read_env_bool("EDGERUN_SCHEDULER_REQUIRE_POLICY_SESSION", true),
        policy_session_bootstrap_token: std::env::var(
            "EDGERUN_SCHEDULER_POLICY_SESSION_BOOTSTRAP_TOKEN",
        )
        .ok()
        .filter(|v| !v.trim().is_empty()),
        policy_session_shared: read_env_bool("EDGERUN_SCHEDULER_POLICY_SESSION_SHARED", false),
        policy_session_lock_path: data_dir.join("policy-session.lock"),
        sessions: Arc::new(Mutex::new(loaded_policy_session_state.sessions)),
        policy_nonces: Arc::new(Mutex::new(loaded_policy_session_state.nonces)),
        policy_session_state_path,
        policy_audit_path: data_dir.join("policy-audit.jsonl"),
        trust_policy: Arc::new(Mutex::new(trust_policy)),
        trust_policy_path,
        attestation_policy: Arc::new(Mutex::new(attestation_policy)),
        attestation_policy_path,
        route_shared_state_path,
        device_routes: Arc::new(Mutex::new(HashMap::new())),
        route_challenges: Arc::new(Mutex::new(HashMap::new())),
        route_heartbeat_tokens: Arc::new(Mutex::new(HashMap::new())),
        signal_peers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        chain,
    };
    enforce_history_retention(&state);

    let housekeeping_state = state.clone();
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/session/create", post(session_create))
        .route("/v1/session/rotate", post(session_rotate))
        .route("/v1/session/invalidate", post(session_invalidate))
        .route("/v1/route/challenge", post(route_challenge))
        .route("/v1/route/register", post(route_register))
        .route("/v1/route/heartbeat", post(route_heartbeat))
        .route("/v1/route/resolve/{device_id}", get(route_resolve))
        .route("/v1/route/owner/{owner_pubkey}", get(route_owner_resolve))
        .route("/v1/webrtc/ws", get(webrtc_signal_ws))
        .route("/v1/policy/info", get(policy_info))
        .route("/v1/policy/audit", get(policy_audit_list))
        .route("/v1/trust/policy/get", get(trust_policy_get))
        .route("/v1/trust/policy/set", post(trust_policy_set))
        .route("/v1/attestation/policy/get", get(attestation_policy_get))
        .route("/v1/attestation/policy/set", post(attestation_policy_set))
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

fn with_route_maps_mut<R>(
    state: &AppState,
    f: impl FnOnce(
        &mut HashMap<String, DeviceRouteEntry>,
        &mut HashMap<String, u64>,
        &mut HashMap<String, RouteHeartbeatToken>,
    ) -> Result<R, (StatusCode, String)>,
) -> Result<R, (StatusCode, String)> {
    if let Some(path) = state.route_shared_state_path.as_ref() {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(internal_err)?;
        file.lock_exclusive().map_err(internal_err)?;
        let mut raw = String::new();
        file.seek(SeekFrom::Start(0)).map_err(internal_err)?;
        file.read_to_string(&mut raw).map_err(internal_err)?;
        let mut persisted = if raw.trim().is_empty() {
            PersistedRouteState::default()
        } else {
            serde_json::from_str::<PersistedRouteState>(&raw).unwrap_or_default()
        };
        let out = f(
            &mut persisted.device_routes,
            &mut persisted.route_challenges,
            &mut persisted.route_heartbeat_tokens,
        )?;
        let encoded = serde_json::to_vec_pretty(&persisted).map_err(internal_err)?;
        file.set_len(0).map_err(internal_err)?;
        file.seek(SeekFrom::Start(0)).map_err(internal_err)?;
        file.write_all(&encoded).map_err(internal_err)?;
        file.flush().map_err(internal_err)?;
        file.unlock().map_err(internal_err)?;
        return Ok(out);
    }

    let mut routes = state.device_routes.lock().expect("lock poisoned");
    let mut challenges = state.route_challenges.lock().expect("lock poisoned");
    let mut tokens = state.route_heartbeat_tokens.lock().expect("lock poisoned");
    f(&mut routes, &mut challenges, &mut tokens)
}

async fn route_challenge(
    State(state): State<AppState>,
    Json(payload): Json<RouteChallengeRequest>,
) -> Result<Json<RouteChallengeResponse>, (StatusCode, String)> {
    let device_id = payload.device_id.trim();
    if device_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "device_id is required".to_string()));
    }
    let now = now_unix_seconds();
    let nonce = edgerun_hwvault_primitives::hardware::random_token_b64url(24);
    let expires_at = now.saturating_add(120);
    with_route_maps_mut(&state, |_, challenges, _| {
        prune_expired_route_challenges(challenges, now);
        challenges.insert(nonce.clone(), expires_at);
        Ok(())
    })?;

    Ok(Json(RouteChallengeResponse {
        nonce,
        expires_at_unix_s: expires_at,
    }))
}

async fn route_register(
    State(state): State<AppState>,
    Json(payload): Json<RouteRegisterRequest>,
) -> Result<Json<RouteRegisterResponse>, (StatusCode, String)> {
    let device_id = payload.device_id.trim();
    let owner_pubkey = payload.owner_pubkey.trim();
    if device_id.is_empty() || owner_pubkey.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "device_id and owner_pubkey are required".to_string(),
        ));
    }
    if payload.reachable_urls.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "at least one reachable_urls entry is required".to_string(),
        ));
    }

    let sanitized_urls = payload
        .reachable_urls
        .into_iter()
        .map(|v| v.trim().to_string())
        .filter(|v| v.starts_with("https://") || v.starts_with("http://"))
        .collect::<Vec<_>>();
    if sanitized_urls.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "reachable_urls must include http(s) endpoints".to_string(),
        ));
    }

    let now = now_unix_seconds();
    if now.saturating_sub(payload.signed_at_unix_s) > 180 {
        return Err((
            StatusCode::UNAUTHORIZED,
            "route register signature expired".to_string(),
        ));
    }

    let msg = route_register_signing_message(
        owner_pubkey,
        device_id,
        &sanitized_urls,
        payload.challenge_nonce.trim(),
        payload.signed_at_unix_s,
    );
    if !verify_any_ed25519_signature(owner_pubkey, &payload.signature, msg.as_bytes())? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid route signature".to_string(),
        ));
    }

    let ttl = payload.ttl_secs.unwrap_or(90).clamp(15, 3600);
    let expires_at = now.saturating_add(ttl);
    let heartbeat_token = edgerun_hwvault_primitives::hardware::random_token_b64url(24);
    let heartbeat_exp = now.saturating_add(24 * 60 * 60);
    with_route_maps_mut(&state, |routes, challenges, heartbeat_tokens| {
        prune_expired_route_challenges(challenges, now);
        let Some(exp) = challenges.remove(payload.challenge_nonce.trim()) else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "unknown or expired route challenge".to_string(),
            ));
        };
        if exp <= now {
            return Err((
                StatusCode::UNAUTHORIZED,
                "route challenge expired".to_string(),
            ));
        }

        prune_expired_routes(routes, now);
        routes.insert(
            device_id.to_string(),
            DeviceRouteEntry {
                device_id: device_id.to_string(),
                owner_pubkey: owner_pubkey.to_string(),
                reachable_urls: sanitized_urls,
                capabilities: payload.capabilities,
                relay_session_id: payload.relay_session_id,
                online: true,
                last_seen_unix_s: now,
                expires_at_unix_s: expires_at,
                updated_at_unix_s: now,
            },
        );

        prune_expired_route_tokens(heartbeat_tokens, now);
        heartbeat_tokens.insert(
            heartbeat_token.clone(),
            RouteHeartbeatToken {
                device_id: device_id.to_string(),
                expires_at_unix_s: heartbeat_exp,
            },
        );
        Ok(())
    })?;

    Ok(Json(RouteRegisterResponse {
        ok: true,
        device_id: device_id.to_string(),
        expires_at_unix_s: expires_at,
        heartbeat_token,
    }))
}

async fn route_heartbeat(
    State(state): State<AppState>,
    Json(payload): Json<RouteHeartbeatRequest>,
) -> Result<Json<RouteHeartbeatResponse>, (StatusCode, String)> {
    let device_id = payload.device_id.trim();
    if device_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "device_id is required".to_string()));
    }
    if payload.token.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "token is required".to_string()));
    }
    let now = now_unix_seconds();
    let ttl = payload.ttl_secs.unwrap_or(90).clamp(15, 3600);
    let expires_at = now.saturating_add(ttl);
    with_route_maps_mut(&state, |routes, _, heartbeat_tokens| {
        prune_expired_route_tokens(heartbeat_tokens, now);
        let Some(token) = heartbeat_tokens.get(payload.token.trim()) else {
            return Err((StatusCode::UNAUTHORIZED, "invalid route token".to_string()));
        };
        if token.device_id != device_id {
            return Err((
                StatusCode::FORBIDDEN,
                "route token/device mismatch".to_string(),
            ));
        }
        prune_expired_routes(routes, now);
        let Some(entry) = routes.get_mut(device_id) else {
            return Err((StatusCode::NOT_FOUND, "route not found".to_string()));
        };
        entry.online = true;
        entry.last_seen_unix_s = now;
        entry.expires_at_unix_s = expires_at;
        entry.updated_at_unix_s = now;
        Ok(())
    })?;

    Ok(Json(RouteHeartbeatResponse {
        ok: true,
        device_id: device_id.to_string(),
        expires_at_unix_s: expires_at,
    }))
}

async fn route_resolve(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Json<RouteResolveResponse> {
    let now = now_unix_seconds();
    let route = with_route_maps_mut(&state, |routes, _, _| {
        prune_expired_routes(routes, now);
        Ok(routes.get(device_id.trim()).cloned())
    })
    .ok()
    .flatten();
    Json(RouteResolveResponse {
        ok: true,
        found: route.is_some(),
        route,
    })
}

async fn route_owner_resolve(
    State(state): State<AppState>,
    Path(owner_pubkey): Path<String>,
) -> Json<OwnerRoutesResponse> {
    let owner_pubkey = owner_pubkey.trim().to_string();
    let now = now_unix_seconds();
    let devices = with_route_maps_mut(&state, |routes, _, _| {
        prune_expired_routes(routes, now);
        Ok(routes
            .values()
            .filter(|route| route.owner_pubkey == owner_pubkey)
            .cloned()
            .collect::<Vec<_>>())
    })
    .unwrap_or_default();
    Json(OwnerRoutesResponse {
        ok: true,
        owner_pubkey,
        devices,
    })
}

async fn webrtc_signal_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WebRtcSignalConnectQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| webrtc_signal_socket(state, query.device_id, socket))
}

async fn webrtc_signal_socket(state: AppState, device_id: String, mut socket: WebSocket) {
    let device_id = device_id.trim().to_string();
    if device_id.is_empty() {
        let _ = socket
            .send(Message::Text(
                r#"{"kind":"error","error":"device_id is required"}"#.into(),
            ))
            .await;
        return;
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    {
        let mut peers = state.signal_peers.lock().await;
        peers.insert(device_id.clone(), tx);
    }
    {
        let now = now_unix_seconds();
        let _ = with_route_maps_mut(&state, |routes, _, _| {
            if let Some(entry) = routes.get_mut(&device_id) {
                entry.online = true;
                entry.last_seen_unix_s = now;
                entry.updated_at_unix_s = now;
                entry.expires_at_unix_s = now.saturating_add(90);
            }
            Ok(())
        });
    }

    loop {
        tokio::select! {
            Some(outbound) = rx.recv() => {
                if socket.send(Message::Text(outbound.into())).await.is_err() {
                    break;
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if handle_signal_client_message(&state, &device_id, &text).await.is_err() {
                            let _ = socket.send(Message::Text(r#"{"kind":"error","error":"invalid signaling message"}"#.into())).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }

    {
        let mut peers = state.signal_peers.lock().await;
        peers.remove(&device_id);
    }
    {
        let now = now_unix_seconds();
        let _ = with_route_maps_mut(&state, |routes, _, _| {
            if let Some(entry) = routes.get_mut(&device_id) {
                entry.online = false;
                entry.updated_at_unix_s = now;
            }
            Ok(())
        });
    }
}

async fn handle_signal_client_message(
    state: &AppState,
    from_device_id: &str,
    text: &str,
) -> Result<(), ()> {
    let msg: WebRtcSignalClientMessage = serde_json::from_str(text).map_err(|_| ())?;
    let to_device_id = msg.to_device_id.trim().to_string();
    let to_owner_pubkey = msg.to_owner_pubkey.trim().to_string();
    if to_device_id.is_empty() && to_owner_pubkey.is_empty() {
        return Err(());
    }

    let outbound = WebRtcSignalServerMessage {
        from_device_id: from_device_id.to_string(),
        kind: msg.kind,
        sdp: msg.sdp,
        candidate: msg.candidate,
        sdp_mid: msg.sdp_mid,
        sdp_mline_index: msg.sdp_mline_index,
        metadata: msg.metadata,
    };
    let encoded = serde_json::to_string(&outbound).map_err(|_| ())?;

    if !to_device_id.is_empty() {
        let sender = {
            let peers = state.signal_peers.lock().await;
            peers.get(to_device_id.as_str()).cloned()
        };
        let Some(sender) = sender else {
            return Err(());
        };
        sender.send(encoded).map_err(|_| ())?;
        return Ok(());
    }

    let target_device_ids = match with_route_maps_mut(state, |routes, _, _| {
        let now = now_unix_seconds();
        prune_expired_routes(routes, now);
        Ok(routes
            .values()
            .filter(|route| route.owner_pubkey == to_owner_pubkey)
            .map(|route| route.device_id.clone())
            .collect::<Vec<_>>())
    }) {
        Ok(ids) => ids,
        Err(_) => return Err(()),
    };
    if target_device_ids.is_empty() {
        return Err(());
    }

    let peers = state.signal_peers.lock().await;
    let mut delivered = 0usize;
    for device_id in target_device_ids {
        if device_id == from_device_id {
            continue;
        }
        if let Some(sender) = peers.get(device_id.as_str()) {
            if sender.send(encoded.clone()).is_ok() {
                delivered = delivered.saturating_add(1);
            }
        }
    }
    if delivered == 0 {
        return Err(());
    }
    Ok(())
}

fn prune_expired_routes(routes: &mut HashMap<String, DeviceRouteEntry>, now: u64) {
    routes.retain(|_, route| route.expires_at_unix_s > now);
}

fn prune_expired_route_challenges(challenges: &mut HashMap<String, u64>, now: u64) {
    challenges.retain(|_, exp| *exp > now);
}

fn prune_expired_route_tokens(tokens: &mut HashMap<String, RouteHeartbeatToken>, now: u64) {
    tokens.retain(|_, tok| tok.expires_at_unix_s > now);
}

fn route_register_signing_message(
    owner_pubkey: &str,
    device_id: &str,
    reachable_urls: &[String],
    challenge_nonce: &str,
    signed_at_unix_s: u64,
) -> String {
    let urls = reachable_urls.join(",");
    format!(
        "edgerun:route_register:v1|{}|{}|{}|{}|{}",
        owner_pubkey, device_id, urls, challenge_nonce, signed_at_unix_s
    )
}

fn parse_any_ed25519_pubkey(value: &str) -> Result<[u8; 32], (StatusCode, String)> {
    if let Ok(pubkey) = value.parse::<Pubkey>() {
        return Ok(pubkey.to_bytes());
    }

    let decoded_std = base64::engine::general_purpose::STANDARD.decode(value.as_bytes());
    if let Ok(bytes) = decoded_std {
        let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "owner_pubkey must decode to 32 bytes".to_string(),
            )
        })?;
        return Ok(arr);
    }

    let bytes = URL_SAFE_NO_PAD.decode(value.as_bytes()).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "owner_pubkey must be base58 or base64/base64url ed25519 key".to_string(),
        )
    })?;
    let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "owner_pubkey must decode to 32 bytes".to_string(),
        )
    })?;
    Ok(arr)
}

fn parse_any_signature(value: &str) -> Result<Signature, (StatusCode, String)> {
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(value.as_bytes())
        .or_else(|_| URL_SAFE_NO_PAD.decode(value.as_bytes()))
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "signature must be base64 or base64url".to_string(),
            )
        })?;
    let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "signature must decode to 64 bytes".to_string(),
        )
    })?;
    Ok(Signature::from_bytes(&sig_arr))
}

fn verify_any_ed25519_signature(
    owner_pubkey: &str,
    signature_encoded: &str,
    message: &[u8],
) -> Result<bool, (StatusCode, String)> {
    let pubkey_bytes = parse_any_ed25519_pubkey(owner_pubkey)?;
    let vk = VerifyingKey::from_bytes(&pubkey_bytes).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid owner_pubkey bytes".to_string(),
        )
    })?;
    let signature = parse_any_signature(signature_encoded)?;
    Ok(edgerun_crypto::verify(&vk, message, &signature))
}

async fn session_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SessionCreateRequest>,
) -> Result<Json<SessionCreateResponse>, (StatusCode, String)> {
    if let Some(expected) = state.policy_session_bootstrap_token.as_deref() {
        let provided = header_value(&headers, "x-edgerun-bootstrap-token").unwrap_or_default();
        if provided != expected {
            return Err((
                StatusCode::UNAUTHORIZED,
                "invalid bootstrap token".to_string(),
            ));
        }
    }

    let now = now_unix_seconds();
    let cfg = edgerun_hwvault_primitives::session::SessionConfig {
        ttl_secs: state.session_ttl_secs.max(1),
        ..edgerun_hwvault_primitives::session::SessionConfig::default()
    };
    let issue = with_policy_session_store_mut(&state, |sessions, _nonces| {
        Ok(edgerun_hwvault_primitives::session::create_session(
            sessions,
            now,
            &cfg,
            payload.bound_origin,
        ))
    })?;
    Ok(Json(SessionCreateResponse {
        token: issue.token,
        session_key: issue.signing_key,
        ttl_secs: issue.ttl_secs,
    }))
}

async fn session_rotate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SessionRotateRequest>,
) -> Result<Json<SessionRotateResponse>, (StatusCode, String)> {
    let now = now_unix_seconds();
    let cfg = edgerun_hwvault_primitives::session::SessionConfig {
        ttl_secs: state.session_ttl_secs.max(1),
        ..edgerun_hwvault_primitives::session::SessionConfig::default()
    };
    let auth_header = header_value(&headers, "authorization");
    let origin_header = header_value(&headers, "origin");
    let ts_header = header_value(&headers, "x-hwv-ts");
    let nonce_header = header_value(&headers, "x-hwv-nonce");
    let sig_header = header_value(&headers, "x-hwv-sig");
    let issue = with_policy_session_store_mut(&state, |sessions, nonces| {
        let token = edgerun_hwvault_primitives::session::verify_session_request(
            sessions,
            nonces,
            edgerun_hwvault_primitives::session::SessionAuthInput {
                auth_header,
                origin_header,
                ts_header,
                nonce_header,
                sig_header,
                method: "POST",
                path: "/v1/session/rotate",
                body: b"",
            },
            now,
            &cfg,
        )
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
        invalidate_session_token(sessions, nonces, &token);
        Ok(edgerun_hwvault_primitives::session::create_session(
            sessions,
            now,
            &cfg,
            payload.bound_origin,
        ))
    })?;
    Ok(Json(SessionRotateResponse {
        token: issue.token,
        session_key: issue.signing_key,
        ttl_secs: issue.ttl_secs,
    }))
}

async fn session_invalidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SessionInvalidateRequest>,
) -> Result<Json<SessionInvalidateResponse>, (StatusCode, String)> {
    let now = now_unix_seconds();
    let cfg = edgerun_hwvault_primitives::session::SessionConfig {
        ttl_secs: state.session_ttl_secs.max(1),
        ..edgerun_hwvault_primitives::session::SessionConfig::default()
    };
    let auth_header = header_value(&headers, "authorization");
    let origin_header = header_value(&headers, "origin");
    let ts_header = header_value(&headers, "x-hwv-ts");
    let nonce_header = header_value(&headers, "x-hwv-nonce");
    let sig_header = header_value(&headers, "x-hwv-sig");
    with_policy_session_store_mut(&state, |sessions, nonces| {
        let caller = edgerun_hwvault_primitives::session::verify_session_request(
            sessions,
            nonces,
            edgerun_hwvault_primitives::session::SessionAuthInput {
                auth_header,
                origin_header,
                ts_header,
                nonce_header,
                sig_header,
                method: "POST",
                path: "/v1/session/invalidate",
                body: b"",
            },
            now,
            &cfg,
        )
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

        if let Some(target) = payload.token.as_deref() {
            if target != caller {
                return Err((
                    StatusCode::FORBIDDEN,
                    "cross-session invalidation is not allowed".to_string(),
                ));
            }
        }
        invalidate_session_token(sessions, nonces, &caller);
        Ok(())
    })?;
    Ok(Json(SessionInvalidateResponse { ok: true }))
}

async fn policy_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PolicyInfoResponse>, (StatusCode, String)> {
    require_policy_session_headers(&state, &headers, "GET", "/v1/policy/info", &[])?;
    Ok(Json(PolicyInfoResponse {
        key_id: state.policy_key_id.clone(),
        version: state.policy_version,
        signer_pubkey: hex::encode(state.policy_signing_key.verifying_key().as_bytes()),
        ttl_secs: state.policy_ttl_secs,
        trust_policy: state.trust_policy.lock().expect("lock poisoned").clone(),
        attestation_policy: state
            .attestation_policy
            .lock()
            .expect("lock poisoned")
            .clone(),
    }))
}

async fn policy_audit_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PolicyAuditQuery>,
) -> Result<Json<PolicyAuditListResponse>, (StatusCode, String)> {
    require_policy_session_headers(&state, &headers, "GET", "/v1/policy/audit", &[])?;
    let limit = query.limit.unwrap_or(100).clamp(1, 10_000);
    let events = edgerun_hwvault_primitives::audit::list_recent_events_jsonl(
        &state.policy_audit_path,
        limit,
    )
    .map_err(internal_err)?;
    Ok(Json(PolicyAuditListResponse { events }))
}

async fn trust_policy_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TrustPolicyResponse>, (StatusCode, String)> {
    require_policy_session_headers(&state, &headers, "GET", "/v1/trust/policy/get", &[])?;
    let policy = state.trust_policy.lock().expect("lock poisoned").clone();
    Ok(Json(TrustPolicyResponse { policy }))
}

async fn trust_policy_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<TrustPolicyResponse>, (StatusCode, String)> {
    require_policy_session_headers(&state, &headers, "POST", "/v1/trust/policy/set", &body)?;
    let payload: TrustPolicySetRequest = serde_json::from_slice(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid trust policy payload".to_string(),
        )
    })?;
    let Some(policy) = edgerun_types::SyncTrustPolicy::from_profile_name(&payload.profile, true)
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            "profile must be one of: strict, balanced, monitor".to_string(),
        ));
    };

    {
        let mut current = state.trust_policy.lock().expect("lock poisoned");
        *current = policy.clone();
    }
    save_trust_policy(&state.trust_policy_path, &policy).map_err(internal_err)?;

    let evt = edgerun_hwvault_primitives::audit::PolicyAuditEvent {
        ts: now_unix_seconds(),
        action: "trust_policy_set".to_string(),
        target: "scheduler".to_string(),
        details: format!(
            "profile={:?} warn_risk={} max_risk={} block_revoked={}",
            policy.profile, policy.warn_risk, policy.max_risk, policy.block_revoked
        ),
    };
    if let Err(err) =
        edgerun_hwvault_primitives::audit::append_event_jsonl(&state.policy_audit_path, &evt)
    {
        tracing::warn!(error = %err, "failed to append trust policy audit event");
    }

    Ok(Json(TrustPolicyResponse { policy }))
}

async fn attestation_policy_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AttestationPolicyResponse>, (StatusCode, String)> {
    require_policy_session_headers(&state, &headers, "GET", "/v1/attestation/policy/get", &[])?;
    let policy = state
        .attestation_policy
        .lock()
        .expect("lock poisoned")
        .clone();
    Ok(Json(AttestationPolicyResponse { policy }))
}

async fn attestation_policy_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AttestationPolicyResponse>, (StatusCode, String)> {
    require_policy_session_headers(
        &state,
        &headers,
        "POST",
        "/v1/attestation/policy/set",
        &body,
    )?;
    let payload: AttestationPolicySetRequest = serde_json::from_slice(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid attestation policy payload".to_string(),
        )
    })?;
    if payload.max_age_secs == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "max_age_secs must be > 0".to_string(),
        ));
    }
    let mut allowed = payload
        .allowed_measurements
        .into_iter()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    allowed.sort();
    allowed.dedup();
    let policy = edgerun_types::AttestationPolicy {
        required: payload.required,
        max_age_secs: payload.max_age_secs,
        allowed_measurements: allowed,
    };
    {
        let mut current = state.attestation_policy.lock().expect("lock poisoned");
        *current = policy.clone();
    }
    save_attestation_policy(&state.attestation_policy_path, &policy).map_err(internal_err)?;
    let evt = edgerun_hwvault_primitives::audit::PolicyAuditEvent {
        ts: now_unix_seconds(),
        action: "attestation_policy_set".to_string(),
        target: "scheduler".to_string(),
        details: format!(
            "required={} max_age_secs={} allowed_measurements={}",
            policy.required,
            policy.max_age_secs,
            policy.allowed_measurements.join(",")
        ),
    };
    if let Err(err) =
        edgerun_hwvault_primitives::audit::append_event_jsonl(&state.policy_audit_path, &evt)
    {
        tracing::warn!(error = %err, "failed to append attestation policy audit event");
    }
    Ok(Json(AttestationPolicyResponse { policy }))
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
    validate_worker_result_payload(&payload)?;
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
    persist_job_activity(&state, &job_id).map_err(internal_err)?;
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
    persist_job_activity(&state, &job_id).map_err(internal_err)?;
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
    persist_job_activity(&state, &job_id).map_err(internal_err)?;
    Ok(Json(serde_json::json!({ "ok": true, "duplicate": false })))
}

async fn job_create(
    State(state): State<AppState>,
    Json(payload): Json<JobCreateRequest>,
) -> Result<Json<JobCreateResponse>, (StatusCode, String)> {
    let now = now_unix_seconds();
    let wasm = base64::engine::general_purpose::STANDARD
        .decode(payload.wasm_base64.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid wasm_base64".to_string()))?;
    let input = base64::engine::general_purpose::STANDARD
        .decode(payload.input_base64.as_bytes())
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid input_base64".to_string()))?;
    let parsed_client = match (
        payload.client_pubkey.as_deref(),
        payload.client_signed_at_unix_s,
        payload.client_signature.as_deref(),
    ) {
        (None, None, None) => None,
        (Some(client_pubkey), Some(client_signed_at_unix_s), Some(client_signature)) => {
            let client = client_pubkey.parse::<Pubkey>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "client_pubkey must be base58 pubkey".to_string(),
                )
            })?;
            if now.saturating_sub(client_signed_at_unix_s) > state.client_signature_max_age_secs {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "client signature expired".to_string(),
                ));
            }
            let message = client_job_create_signing_message(
                client_pubkey,
                &payload.runtime_id,
                payload.limits.max_memory_bytes,
                payload.limits.max_instructions,
                payload.escrow_lamports,
                &wasm,
                &input,
                client_signed_at_unix_s,
            );
            if !verify_client_message_signature(client, client_signature, &message)? {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "invalid client signature".to_string(),
                ));
            }
            Some(client)
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "client auth fields must all be set together".to_string(),
            ));
        }
    };
    {
        let policy = state.trust_policy.lock().expect("lock poisoned").clone();
        if matches!(policy.profile, edgerun_types::SyncTrustProfile::Strict)
            && parsed_client.is_none()
        {
            return Err((
                StatusCode::UNAUTHORIZED,
                "strict trust policy requires authenticated client signatures".to_string(),
            ));
        }
    }
    if state.require_client_signatures && parsed_client.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "client signature is required".to_string(),
        ));
    }
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
    write_bundle_cas(&bundle_path, &bundle_hash_hex, &bundle_payload_bytes)
        .map_err(internal_err)?;

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
                slash_artifacts: Vec::new(),
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
    let audit_event = edgerun_hwvault_primitives::audit::PolicyAuditEvent {
        ts: now,
        action: "job_create".to_string(),
        target: bundle_hash_hex.clone(),
        details: format!(
            "policy_key_id={} policy_version={} quorum={} committee={} escrow_lamports={}",
            state.policy_key_id,
            state.policy_version,
            effective_quorum,
            state.committee_size,
            payload.escrow_lamports
        ),
    };
    if let Err(err) = edgerun_hwvault_primitives::audit::append_event_jsonl(
        &state.policy_audit_path,
        &audit_event,
    ) {
        tracing::warn!(error = %err, "failed to append policy audit event");
    }

    let (post_job_tx, post_job_sig) = if let Some(chain) = state.chain.as_ref() {
        let runtime_proof = match build_runtime_allowlist_proof_for_chain(chain, runtime_id) {
            Ok(proof) => proof,
            Err(err) => {
                tracing::warn!(error = %err, "failed to build runtime allowlist proof");
                return Ok(Json(JobCreateResponse {
                    job_id: bundle_hash_hex.clone(),
                    bundle_hash: bundle_hash_hex.clone(),
                    bundle_url: format!("{}/bundle/{bundle_hash_hex}", state.public_base_url),
                    post_job_tx: "UNAVAILABLE_BUILD_FAILED".to_string(),
                    post_job_sig: None,
                    assign_workers_tx: assign_workers_tx.clone(),
                    assign_workers_sig,
                }));
            }
        };
        let tx_args = PostJobArgs {
            client: parsed_client.unwrap_or_else(|| chain.payer.pubkey()),
            job_id: bundle_hash,
            bundle_hash,
            runtime_id,
            runtime_proof,
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
    let computed = hex::encode(edgerun_crypto::compute_bundle_hash(&bytes));
    if !computed.eq_ignore_ascii_case(&bundle_hash) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "bundle content hash mismatch".to_string(),
        ));
    }
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

    let mut tx = Transaction::new_with_payer(&[ix], Some(&chain.payer.pubkey()));
    tx.partial_sign(&[&chain.payer], blockhash);
    let signature = tx.signatures.first().map(ToString::to_string);
    let tx_bytes = bincode::serialize(&tx).context("failed to serialize transaction")?;
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(tx_bytes);
    Ok((tx_b64, signature))
}

fn build_runtime_allowlist_proof_for_chain(
    chain: &ChainContext,
    runtime_id: [u8; 32],
) -> Result<Vec<[u8; 32]>> {
    let allowed_root = fetch_chain_allowed_runtime_root(chain)?;
    if allowed_root == [0_u8; 32] {
        return Ok(Vec::new());
    }

    let leaves = load_allowed_runtime_ids_from_env()?.ok_or_else(|| {
        anyhow::anyhow!(
            "config.allowed_runtime_root is non-zero but EDGERUN_ALLOWED_RUNTIME_IDS is unset"
        )
    })?;
    let (derived_root, proof) = build_merkle_root_and_proof(&leaves, &runtime_id)
        .ok_or_else(|| anyhow::anyhow!("runtime_id is not in EDGERUN_ALLOWED_RUNTIME_IDS"))?;
    if derived_root != allowed_root {
        anyhow::bail!(
            "runtime allowlist root mismatch: env-derived root does not match on-chain config"
        );
    }
    Ok(proof)
}

fn load_allowed_runtime_ids_from_env() -> Result<Option<Vec<[u8; 32]>>> {
    let Some(raw) = std::env::var("EDGERUN_ALLOWED_RUNTIME_IDS").ok() else {
        return Ok(None);
    };
    let mut leaves = Vec::new();
    for entry in raw.split(',').map(|v| v.trim()).filter(|v| !v.is_empty()) {
        leaves.push(parse_hex32(entry).map_err(|_| {
            anyhow::anyhow!("EDGERUN_ALLOWED_RUNTIME_IDS entries must be 32-byte hex values")
        })?);
    }
    if leaves.is_empty() {
        return Ok(None);
    }
    leaves.sort();
    leaves.dedup();
    Ok(Some(leaves))
}

fn fetch_chain_allowed_runtime_root(chain: &ChainContext) -> Result<[u8; 32]> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &chain.program_id);
    let account = chain
        .rpc
        .get_account(&config_pda)
        .context("failed to fetch config account")?;
    parse_config_allowed_runtime_root(&account.data)
        .ok_or_else(|| anyhow::anyhow!("unable to parse config.allowed_runtime_root"))
}

fn merkle_parent_sorted(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut preimage = [0_u8; 64];
    if a <= b {
        preimage[0..32].copy_from_slice(a);
        preimage[32..64].copy_from_slice(b);
    } else {
        preimage[0..32].copy_from_slice(b);
        preimage[32..64].copy_from_slice(a);
    }
    edgerun_crypto::blake3_256(&preimage)
}

fn build_merkle_root_and_proof(
    leaves: &[[u8; 32]],
    target: &[u8; 32],
) -> Option<([u8; 32], Vec<[u8; 32]>)> {
    if leaves.is_empty() {
        return None;
    }
    let mut layer = leaves.to_vec();
    layer.sort();
    layer.dedup();
    let mut index = layer.iter().position(|leaf| leaf == target)?;
    let mut proof = Vec::new();

    while layer.len() > 1 {
        let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
        let sibling = if sibling_index < layer.len() {
            layer[sibling_index]
        } else {
            layer[index]
        };
        proof.push(sibling);

        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0usize;
        while i < layer.len() {
            let left = layer[i];
            let right = if i + 1 < layer.len() {
                layer[i + 1]
            } else {
                layer[i]
            };
            next.push(merkle_parent_sorted(&left, &right));
            i += 2;
        }
        layer = next;
        index /= 2;
    }

    Some((layer[0], proof))
}

fn build_finalize_job_tx_base64(
    chain: &ChainContext,
    job_id: [u8; 32],
    committee: [Pubkey; 3],
    winners: Vec<Pubkey>,
    auto_submit: bool,
) -> Result<(String, Option<String>, bool)> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &chain.program_id);
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &chain.program_id);
    let (output_availability_pda, _) =
        Pubkey::find_program_address(&[b"output", &job_id], &chain.program_id);
    let (worker_stake_0, _) =
        Pubkey::find_program_address(&[b"worker_stake", committee[0].as_ref()], &chain.program_id);
    let (worker_stake_1, _) =
        Pubkey::find_program_address(&[b"worker_stake", committee[1].as_ref()], &chain.program_id);
    let (worker_stake_2, _) =
        Pubkey::find_program_address(&[b"worker_stake", committee[2].as_ref()], &chain.program_id);
    let (job_result_0, _) = Pubkey::find_program_address(
        &[b"job_result", &job_id, committee[0].as_ref()],
        &chain.program_id,
    );
    let (job_result_1, _) = Pubkey::find_program_address(
        &[b"job_result", &job_id, committee[1].as_ref()],
        &chain.program_id,
    );
    let (job_result_2, _) = Pubkey::find_program_address(
        &[b"job_result", &job_id, committee[2].as_ref()],
        &chain.program_id,
    );

    let mut accounts = vec![
        AccountMeta::new(chain.payer.pubkey(), true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new(job_pda, false),
        AccountMeta::new_readonly(output_availability_pda, false),
        AccountMeta::new(worker_stake_0, false),
        AccountMeta::new(worker_stake_1, false),
        AccountMeta::new(worker_stake_2, false),
        AccountMeta::new_readonly(job_result_0, false),
        AccountMeta::new_readonly(job_result_1, false),
        AccountMeta::new_readonly(job_result_2, false),
    ];
    for winner in winners {
        accounts.push(AccountMeta::new(winner, false));
    }
    let ix = Instruction {
        program_id: chain.program_id,
        accounts,
        data: encode_finalize_job_data(),
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
    committee: [Pubkey; 3],
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
    let ix = Instruction {
        program_id: chain.program_id,
        accounts: vec![
            AccountMeta::new(chain.payer.pubkey(), true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(job_pda, false),
            AccountMeta::new(client, false),
            AccountMeta::new(worker_stake_0, false),
            AccountMeta::new(worker_stake_1, false),
            AccountMeta::new(worker_stake_2, false),
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

fn build_slash_worker_tx_base64(
    chain: &ChainContext,
    job_id: [u8; 32],
    worker: Pubkey,
    auto_submit: bool,
) -> Result<(String, Option<String>, bool)> {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &chain.program_id);
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &chain.program_id);
    let (worker_stake_pda, _) =
        Pubkey::find_program_address(&[b"worker_stake", worker.as_ref()], &chain.program_id);
    let (job_result_pda, _) = Pubkey::find_program_address(
        &[b"job_result", &job_id, worker.as_ref()],
        &chain.program_id,
    );
    let ix = Instruction {
        program_id: chain.program_id,
        accounts: vec![
            AccountMeta::new(chain.payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(job_pda, false),
            AccountMeta::new(worker_stake_pda, false),
            AccountMeta::new_readonly(job_result_pda, false),
        ],
        data: encode_slash_worker_data(),
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
            .context("failed to send slash_worker transaction")?;
        return Ok((tx_b64, Some(sent.to_string()), true));
    }
    Ok((tx_b64, signature, false))
}

fn encode_post_job_data(args: PostJobArgs) -> Vec<u8> {
    let mut data =
        Vec::with_capacity(8 + 32 + 32 + 32 + 4 + (args.runtime_proof.len() * 32) + 4 + 8 + 8);
    data.extend_from_slice(&anchor_discriminator("post_job"));
    data.extend_from_slice(&args.job_id);
    data.extend_from_slice(&args.bundle_hash);
    data.extend_from_slice(&args.runtime_id);
    data.extend_from_slice(&(args.runtime_proof.len() as u32).to_le_bytes());
    for sibling in args.runtime_proof {
        data.extend_from_slice(&sibling);
    }
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

fn encode_finalize_job_data() -> Vec<u8> {
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&anchor_discriminator("finalize_job"));
    data
}

fn encode_cancel_expired_job_data() -> Vec<u8> {
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&anchor_discriminator("cancel_expired_job"));
    data
}

fn encode_slash_worker_data() -> Vec<u8> {
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&anchor_discriminator("slash_worker"));
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

fn write_bundle_cas(path: &PathBuf, expected_bundle_hash_hex: &str, bytes: &[u8]) -> Result<()> {
    let computed = hex::encode(edgerun_crypto::compute_bundle_hash(bytes));
    if !computed.eq_ignore_ascii_case(expected_bundle_hash_hex) {
        anyhow::bail!("bundle bytes do not match expected bundle hash");
    }
    if path.exists() {
        let existing = std::fs::read(path).context("failed to read existing bundle")?;
        if existing != bytes {
            anyhow::bail!("bundle path already exists with different bytes");
        }
        return Ok(());
    }
    std::fs::write(path, bytes).context("failed to write bundle bytes")?;
    Ok(())
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
    let final_path = state.data_dir.join("state.json");
    let tmp_path = state
        .data_dir
        .join(format!("state.json.tmp-{}", std::process::id()));
    std::fs::write(&tmp_path, &bytes)?;
    if let Err(err) = std::fs::rename(&tmp_path, &final_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err.into());
    }
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
    let slash_artifacts_enabled = state.enable_slash_artifacts;
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
            if slash_artifacts_enabled {
                quorum_state.slash_artifacts = build_slash_worker_artifacts(
                    state,
                    job_id,
                    &winning_hash,
                    &committee_workers,
                    &filtered_reports,
                );
            }
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

    let mut best: Option<(String, Vec<String>)> = None;
    let mut saw_tie = false;
    for (output_hash, workers) in counts.into_iter() {
        if workers.len() < quorum_target {
            continue;
        }
        match &best {
            None => {
                best = Some((output_hash.to_string(), workers));
                saw_tie = false;
            }
            Some((_, best_workers)) if workers.len() > best_workers.len() => {
                best = Some((output_hash.to_string(), workers));
                saw_tie = false;
            }
            Some((_, best_workers)) if workers.len() == best_workers.len() => {
                saw_tie = true;
            }
            Some(_) => {}
        }
    }
    if saw_tie {
        return None;
    }
    best
}

fn validate_worker_result_payload(
    payload: &WorkerResultReport,
) -> Result<(), (StatusCode, String)> {
    parse_hex32(&payload.job_id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "job_id must be 32-byte hex".to_string(),
        )
    })?;
    parse_hex32(&payload.bundle_hash).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "bundle_hash must be 32-byte hex".to_string(),
        )
    })?;
    parse_hex32(&payload.output_hash).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "output_hash must be 32-byte hex".to_string(),
        )
    })?;
    Ok(())
}

fn build_slash_worker_artifacts(
    state: &AppState,
    job_id_hex: &str,
    winning_hash: &str,
    committee_workers: &[String],
    reports: &[WorkerResultReport],
) -> Vec<SlashWorkerArtifact> {
    let mut candidate_workers: Vec<String> = Vec::new();
    let committee: HashSet<&str> = committee_workers.iter().map(|w| w.as_str()).collect();
    let mut seen: HashSet<&str> = HashSet::new();
    for report in reports {
        if !seen.insert(report.worker_pubkey.as_str()) {
            continue;
        }
        if !committee.contains(report.worker_pubkey.as_str()) {
            continue;
        }
        if report.output_hash == winning_hash {
            continue;
        }
        if report.attestation_sig.is_none()
            || !verify_result_attestation(state, report).unwrap_or(false)
        {
            continue;
        }
        candidate_workers.push(report.worker_pubkey.clone());
    }
    if candidate_workers.is_empty() {
        return Vec::new();
    }

    let Some(chain) = state.chain.as_ref() else {
        return candidate_workers
            .into_iter()
            .map(|worker_pubkey| SlashWorkerArtifact {
                worker_pubkey,
                tx: "UNAVAILABLE_NO_CHAIN_CONTEXT".to_string(),
                sig: None,
                submitted: false,
            })
            .collect();
    };
    let Ok(job_id) = parse_hex32(job_id_hex) else {
        return candidate_workers
            .into_iter()
            .map(|worker_pubkey| SlashWorkerArtifact {
                worker_pubkey,
                tx: "UNAVAILABLE_INVALID_JOB_ID".to_string(),
                sig: None,
                submitted: false,
            })
            .collect();
    };

    candidate_workers
        .into_iter()
        .map(|worker_pubkey| {
            let Some(worker) = worker_pubkey.parse::<Pubkey>().ok() else {
                return SlashWorkerArtifact {
                    worker_pubkey,
                    tx: "UNAVAILABLE_INVALID_WORKER_PUBKEY".to_string(),
                    sig: None,
                    submitted: false,
                };
            };
            match build_slash_worker_tx_base64(chain, job_id, worker, state.chain_auto_submit) {
                Ok((tx, sig, submitted)) => SlashWorkerArtifact {
                    worker_pubkey,
                    tx,
                    sig,
                    submitted,
                },
                Err(_) => SlashWorkerArtifact {
                    worker_pubkey,
                    tx: "UNAVAILABLE_SLASH_BUILD_FAILED".to_string(),
                    sig: None,
                    submitted: false,
                },
            }
        })
        .collect()
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
    let committee = parse_committee_workers(committee_workers)?;
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

fn parse_committee_workers(committee_workers: &[String]) -> Option<[Pubkey; 3]> {
    if committee_workers.len() != 3 {
        return None;
    }
    Some([
        committee_workers.first()?.parse::<Pubkey>().ok()?,
        committee_workers.get(1)?.parse::<Pubkey>().ok()?,
        committee_workers.get(2)?.parse::<Pubkey>().ok()?,
    ])
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
    let committee_workers = {
        let jq = state.job_quorum.lock().expect("lock poisoned");
        jq.get(job_id_hex)
            .map(|entry| entry.committee_workers.clone())
            .unwrap_or_default()
    };
    let committee = parse_committee_workers(&committee_workers)
        .ok_or_else(|| anyhow::anyhow!("invalid committee workers for cancel_expired_job"))?;
    let (tx, sig, submitted) = build_cancel_expired_job_tx_base64(
        chain,
        job_id,
        chain.payer.pubkey(),
        committee,
        state.chain_auto_submit,
    )?;
    Ok((tx, sig, submitted))
}

const ANCHOR_DISCRIMINATOR_LEN: usize = 8;
const GLOBAL_CONFIG_ALLOWED_RUNTIME_ROOT_OFFSET_FROM_ANCHOR: usize = 96;
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
const JOB_RESULT_ACCOUNT_MIN_LEN: usize = ANCHOR_DISCRIMINATOR_LEN + 168;
const JOB_RESULT_JOB_ID_OFFSET_FROM_ANCHOR: usize = 0;
const JOB_RESULT_WORKER_OFFSET_FROM_ANCHOR: usize = 32;
const JOB_RESULT_OUTPUT_HASH_OFFSET_FROM_ANCHOR: usize = 64;
const JOB_RESULT_ATTESTATION_SIG_OFFSET_FROM_ANCHOR: usize = 96;
const JOB_RESULT_SUBMITTED_SLOT_OFFSET_FROM_ANCHOR: usize = 160;

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

#[derive(Debug, Clone)]
struct OnchainJobResultView {
    job_id: [u8; 32],
    worker: Pubkey,
    output_hash: [u8; 32],
    attestation_sig: [u8; 64],
    submitted_slot: u64,
}

fn posted_job_rpc_filters() -> Vec<RpcFilterType> {
    vec![
        RpcFilterType::DataSize(JOB_ACCOUNT_MIN_LEN as u64),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            anchor_account_discriminator("Job").to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR,
            vec![0_u8], // Posted
        )),
    ]
}

fn job_result_rpc_filters() -> Vec<RpcFilterType> {
    vec![
        RpcFilterType::DataSize(JOB_RESULT_ACCOUNT_MIN_LEN as u64),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            anchor_account_discriminator("JobResult").to_vec(),
        )),
    ]
}

fn discover_posted_jobs_from_chain(state: &AppState) -> Result<()> {
    let Some(chain) = state.chain.as_ref() else {
        return Ok(());
    };
    let filters = posted_job_rpc_filters();
    let accounts = chain
        .rpc
        .get_program_accounts_with_config(
            &chain.program_id,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                ..RpcProgramAccountsConfig::default()
            },
        )
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

fn sync_onchain_job_results(state: &AppState) -> Result<()> {
    let Some(chain) = state.chain.as_ref() else {
        return Ok(());
    };
    let accounts = chain
        .rpc
        .get_program_accounts_with_config(
            &chain.program_id,
            RpcProgramAccountsConfig {
                filters: Some(job_result_rpc_filters()),
                ..RpcProgramAccountsConfig::default()
            },
        )
        .context("failed to fetch program accounts for job-result sync")?;
    if accounts.is_empty() {
        return Ok(());
    }

    let mut changed_jobs: HashSet<String> = HashSet::new();
    for (_addr, account) in accounts {
        let Some(view) = parse_onchain_job_result_view(&account.data) else {
            continue;
        };
        if let Some(job_id_hex) = ingest_onchain_job_result_view(state, &view) {
            tracing::info!(
                job_id = %job_id_hex,
                worker = %view.worker,
                submitted_slot = view.submitted_slot,
                "synced on-chain JobResult into scheduler report state"
            );
            changed_jobs.insert(job_id_hex);
        }
    }

    if changed_jobs.is_empty() {
        return Ok(());
    }

    for job_id in changed_jobs {
        let _ = recompute_job_quorum(state, &job_id)?;
    }
    enforce_history_retention(state);
    write_state_snapshot(state)?;
    Ok(())
}

fn ingest_onchain_job_result_view(state: &AppState, view: &OnchainJobResultView) -> Option<String> {
    let job_id_hex = hex::encode(view.job_id);
    let (expected_bundle_hash, expected_runtime_id, is_assigned) = {
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        let entry = job_quorum.get(&job_id_hex)?;
        let worker = view.worker.to_string();
        let assigned = entry.committee_workers.iter().any(|w| w == &worker);
        (
            entry.expected_bundle_hash.clone(),
            entry.expected_runtime_id.clone(),
            assigned,
        )
    };
    if !is_assigned {
        return None;
    }
    if !verify_onchain_job_result_attestation(view, &expected_bundle_hash, &expected_runtime_id) {
        return None;
    }

    let worker_pubkey = view.worker.to_string();
    let output_hash_hex = hex::encode(view.output_hash);
    let idem = scheduler_idempotency_key(
        "onchain_result",
        &worker_pubkey,
        &job_id_hex,
        "job_result_sync",
        &output_hash_hex,
        &expected_bundle_hash,
    );
    let mut results = state.results.lock().expect("lock poisoned");
    let entries = results.entry(job_id_hex.clone()).or_default();
    if entries.iter().any(|e| {
        (e.idempotency_key == idem)
            || (e.worker_pubkey == worker_pubkey && e.output_hash == output_hash_hex)
    }) {
        return None;
    }
    entries.push(WorkerResultReport {
        idempotency_key: idem,
        worker_pubkey,
        job_id: job_id_hex.clone(),
        bundle_hash: expected_bundle_hash,
        output_hash: output_hash_hex,
        output_len: 0,
        attestation_sig: Some(
            base64::engine::general_purpose::STANDARD.encode(view.attestation_sig),
        ),
        attestation_claim: None,
        signature: None,
    });
    drop(results);
    touch_job_last_update(state, &job_id_hex);
    Some(job_id_hex)
}

fn verify_onchain_job_result_attestation(
    view: &OnchainJobResultView,
    expected_bundle_hash_hex: &str,
    expected_runtime_id_hex: &str,
) -> bool {
    let worker_bytes = view.worker.to_bytes();
    let Ok(worker_vk) = VerifyingKey::from_bytes(&worker_bytes) else {
        return false;
    };
    let signature = Signature::from_bytes(&view.attestation_sig);
    let Ok(bundle_hash) = parse_hex32(expected_bundle_hash_hex) else {
        return false;
    };
    let Ok(runtime_id) = parse_hex32(expected_runtime_id_hex) else {
        return false;
    };
    let message =
        build_worker_result_digest(&view.job_id, &bundle_hash, &view.output_hash, &runtime_id);
    edgerun_crypto::verify(&worker_vk, &message, &signature)
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
                slash_artifacts: Vec::new(),
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

fn parse_config_allowed_runtime_root(data: &[u8]) -> Option<[u8; 32]> {
    if !has_anchor_account_discriminator(data, "GlobalConfig") {
        return None;
    }
    read_fixed_32(data, GLOBAL_CONFIG_ALLOWED_RUNTIME_ROOT_OFFSET_FROM_ANCHOR)
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

fn parse_onchain_job_result_view(data: &[u8]) -> Option<OnchainJobResultView> {
    if data.len() < JOB_RESULT_ACCOUNT_MIN_LEN {
        return None;
    }
    if !has_anchor_account_discriminator(data, "JobResult") {
        return None;
    }
    let job_id = read_fixed_32(data, JOB_RESULT_JOB_ID_OFFSET_FROM_ANCHOR)?;
    let worker_bytes = read_fixed_32(data, JOB_RESULT_WORKER_OFFSET_FROM_ANCHOR)?;
    let worker = Pubkey::new_from_array(worker_bytes);
    let output_hash = read_fixed_32(data, JOB_RESULT_OUTPUT_HASH_OFFSET_FROM_ANCHOR)?;
    let attestation_sig = read_fixed_64(data, JOB_RESULT_ATTESTATION_SIG_OFFSET_FROM_ANCHOR)?;
    let submitted_slot = read_u64_from_anchor(data, JOB_RESULT_SUBMITTED_SLOT_OFFSET_FROM_ANCHOR)?;
    Some(OnchainJobResultView {
        job_id,
        worker,
        output_hash,
        attestation_sig,
        submitted_slot,
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

fn read_fixed_64(data: &[u8], offset_from_anchor: usize) -> Option<[u8; 64]> {
    let start = ANCHOR_DISCRIMINATOR_LEN + offset_from_anchor;
    let slice = data.get(start..start + 64)?;
    let mut out = [0_u8; 64];
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

fn client_job_create_signing_message(
    client_pubkey: &str,
    runtime_id: &str,
    max_memory_bytes: u32,
    max_instructions: u64,
    escrow_lamports: u64,
    wasm: &[u8],
    input: &[u8],
    signed_at_unix_s: u64,
) -> String {
    let wasm_hash_hex = hex::encode(hash(wasm).to_bytes());
    let input_hash_hex = hex::encode(hash(input).to_bytes());
    format!(
        "edgerun:job_create:v1|{}|{}|{}|{}|{}|{}|{}|{}",
        client_pubkey,
        runtime_id,
        max_memory_bytes,
        max_instructions,
        escrow_lamports,
        wasm_hash_hex,
        input_hash_hex,
        signed_at_unix_s
    )
}

fn verify_client_message_signature(
    client_pubkey: Pubkey,
    signature_b64: &str,
    message: &str,
) -> Result<bool, (StatusCode, String)> {
    let client_pk_bytes: [u8; 32] = client_pubkey.to_bytes();
    let client_vk = VerifyingKey::from_bytes(&client_pk_bytes).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid client pubkey bytes".to_string(),
        )
    })?;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64.as_bytes())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "client_signature must be base64".to_string(),
            )
        })?;
    let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "client_signature must decode to 64 bytes".to_string(),
        )
    })?;
    let signature = Signature::from_bytes(&sig_arr);
    Ok(edgerun_crypto::verify(
        &client_vk,
        message.as_bytes(),
        &signature,
    ))
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
    if !verify_attestation_claim_stub(state, payload.attestation_claim.as_ref()) {
        return Ok(false);
    }

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
    let bundle_hash = parse_hex32(&payload.bundle_hash).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "bundle_hash must be 32-byte hex".to_string(),
        )
    })?;
    let (expected_bundle_hash, expected_runtime_id) = {
        let jq = state.job_quorum.lock().expect("lock poisoned");
        let Some(entry) = jq.get(&payload.job_id) else {
            return Ok(false);
        };
        let expected_bundle_hash = parse_hex32(&entry.expected_bundle_hash).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "expected bundle_hash must be 32-byte hex".to_string(),
            )
        })?;
        let expected_runtime_id = parse_hex32(&entry.expected_runtime_id).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "expected runtime_id must be 32-byte hex".to_string(),
            )
        })?;
        (expected_bundle_hash, expected_runtime_id)
    };
    if bundle_hash != expected_bundle_hash {
        return Ok(false);
    }
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
    let message =
        build_worker_result_digest(&job_id, &bundle_hash, &output_hash, &expected_runtime_id);
    Ok(edgerun_crypto::verify(&worker_vk, &message, &signature))
}

fn build_worker_result_digest(
    job_id: &[u8; 32],
    bundle_hash: &[u8; 32],
    output_hash: &[u8; 32],
    runtime_id: &[u8; 32],
) -> [u8; 32] {
    let mut preimage = [0_u8; 128];
    preimage[0..32].copy_from_slice(job_id);
    preimage[32..64].copy_from_slice(bundle_hash);
    preimage[64..96].copy_from_slice(output_hash);
    preimage[96..128].copy_from_slice(runtime_id);
    edgerun_crypto::blake3_256(&preimage)
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
    let attestation_sig = payload.attestation_sig.clone().unwrap_or_default();
    let attestation_claim = payload
        .attestation_claim
        .as_ref()
        .and_then(|c| serde_json::to_string(c).ok())
        .unwrap_or_default();
    format!(
        "result|{}|{}|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.bundle_hash,
        payload.output_hash,
        payload.output_len,
        attestation_sig,
        attestation_claim
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

fn load_trust_policy(path: &std::path::Path) -> Result<edgerun_types::SyncTrustPolicy> {
    if !path.exists() {
        return Ok(edgerun_types::SyncTrustPolicy::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read trust policy file {}", path.display()))?;
    let parsed: edgerun_types::SyncTrustPolicy =
        serde_json::from_str(&raw).context("invalid trust policy json")?;
    Ok(parsed)
}

fn load_attestation_policy(path: &std::path::Path) -> Result<edgerun_types::AttestationPolicy> {
    if !path.exists() {
        return Ok(edgerun_types::AttestationPolicy::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read attestation policy file {}", path.display()))?;
    let parsed: edgerun_types::AttestationPolicy =
        serde_json::from_str(&raw).context("invalid attestation policy json")?;
    Ok(parsed)
}

fn save_trust_policy(
    path: &std::path::Path,
    policy: &edgerun_types::SyncTrustPolicy,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create trust policy parent directory {}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(policy).context("serialize trust policy")?;
    std::fs::write(path, bytes)
        .with_context(|| format!("failed to write trust policy file {}", path.display()))?;
    Ok(())
}

fn save_attestation_policy(
    path: &std::path::Path,
    policy: &edgerun_types::AttestationPolicy,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create attestation policy parent directory {}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(policy).context("serialize attestation policy")?;
    std::fs::write(path, bytes)
        .with_context(|| format!("failed to write attestation policy file {}", path.display()))?;
    Ok(())
}

fn load_policy_session_state(path: &std::path::Path) -> Result<PersistedPolicySessionState> {
    if !path.exists() {
        return Ok(PersistedPolicySessionState::default());
    }
    let raw = std::fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read policy session state file {}",
            path.display()
        )
    })?;
    let parsed: PersistedPolicySessionState =
        serde_json::from_str(&raw).context("invalid policy session state json")?;
    Ok(parsed)
}

fn save_policy_session_state(
    path: &std::path::Path,
    state: &PersistedPolicySessionState,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create policy session state parent directory {}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(state).context("serialize policy session state")?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).with_context(|| {
        format!(
            "failed to write temp policy session state file {}",
            tmp.display()
        )
    })?;
    std::fs::rename(&tmp, path).with_context(|| {
        format!(
            "failed to rename policy session state temp file {} to {}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn persist_policy_session_state(state: &AppState) -> Result<()> {
    let sessions = state.sessions.lock().expect("lock poisoned").clone();
    let nonces = state.policy_nonces.lock().expect("lock poisoned").clone();
    let snapshot = PersistedPolicySessionState { sessions, nonces };
    save_policy_session_state(&state.policy_session_state_path, &snapshot)
}

async fn housekeeping_loop(state: AppState) {
    let interval_secs = read_env_u64("EDGERUN_SCHEDULER_HOUSEKEEPING_INTERVAL_SECS", 5).max(1);
    loop {
        if let Err(err) = discover_posted_jobs_from_chain(&state) {
            tracing::warn!(error = %err, "housekeeping posted-job discovery failed");
        }
        if let Err(err) = sync_onchain_job_results(&state) {
            tracing::warn!(error = %err, "housekeeping on-chain result sync failed");
        }
        if let Err(err) = evaluate_expired_jobs(&state) {
            tracing::warn!(error = %err, "housekeeping evaluate_expired_jobs failed");
        }
        if let Err(err) = reconcile_onchain_job_statuses(&state) {
            tracing::warn!(error = %err, "housekeeping on-chain reconciliation failed");
        }
        let now = now_unix_seconds();
        if let Err((_, err)) = with_policy_session_store_mut(&state, |sessions, nonces| {
            edgerun_hwvault_primitives::session::cleanup_expired(sessions, nonces, now);
            Ok(())
        }) {
            tracing::warn!(error = %err, "failed to cleanup policy session state");
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

fn require_policy_session_headers(
    state: &AppState,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<(), (StatusCode, String)> {
    if !state.require_policy_session {
        return Ok(());
    }
    let now = now_unix_seconds();
    let cfg = edgerun_hwvault_primitives::session::SessionConfig {
        ttl_secs: state.session_ttl_secs.max(1),
        ..edgerun_hwvault_primitives::session::SessionConfig::default()
    };

    let auth_header = header_value(headers, "authorization");
    let origin_header = header_value(headers, "origin");
    let ts_header = header_value(headers, "x-hwv-ts");
    let nonce_header = header_value(headers, "x-hwv-nonce");
    let sig_header = header_value(headers, "x-hwv-sig");

    with_policy_session_store_mut(state, |sessions, nonces| {
        edgerun_hwvault_primitives::session::verify_session_request(
            sessions,
            nonces,
            edgerun_hwvault_primitives::session::SessionAuthInput {
                auth_header,
                origin_header,
                ts_header,
                nonce_header,
                sig_header,
                method,
                path,
                body,
            },
            now,
            &cfg,
        )
        .map(|_| ())
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
    })?;
    Ok(())
}

fn with_policy_session_store_mut<T>(
    state: &AppState,
    mutator: impl FnOnce(
        &mut HashMap<String, edgerun_hwvault_primitives::session::SessionState>,
        &mut HashMap<String, u64>,
    ) -> Result<T, (StatusCode, String)>,
) -> Result<T, (StatusCode, String)> {
    if state.policy_session_shared {
        if let Some(parent) = state.policy_session_lock_path.parent() {
            std::fs::create_dir_all(parent).map_err(internal_err)?;
        }
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&state.policy_session_lock_path)
            .map_err(internal_err)?;
        lock_file.lock_exclusive().map_err(internal_err)?;

        let loaded = load_policy_session_state(&state.policy_session_state_path)
            .unwrap_or_else(|_| PersistedPolicySessionState::default());
        let op_result = {
            let mut sessions = state.sessions.lock().expect("lock poisoned");
            let mut nonces = state.policy_nonces.lock().expect("lock poisoned");
            *sessions = loaded.sessions;
            *nonces = loaded.nonces;
            let result = mutator(&mut sessions, &mut nonces);
            let snapshot = PersistedPolicySessionState {
                sessions: sessions.clone(),
                nonces: nonces.clone(),
            };
            if let Err(err) = save_policy_session_state(&state.policy_session_state_path, &snapshot)
            {
                tracing::warn!(error = %err, "failed to persist shared policy session state");
            }
            result
        };
        drop(lock_file);
        return op_result;
    }

    let op_result = {
        let mut sessions = state.sessions.lock().expect("lock poisoned");
        let mut nonces = state.policy_nonces.lock().expect("lock poisoned");
        mutator(&mut sessions, &mut nonces)
    };
    if let Err(err) = persist_policy_session_state(state) {
        tracing::warn!(error = %err, "failed to persist policy session state");
    }
    op_result
}

fn invalidate_session_token(
    sessions: &mut HashMap<String, edgerun_hwvault_primitives::session::SessionState>,
    nonces: &mut HashMap<String, u64>,
    token: &str,
) {
    sessions.remove(token);
    let prefix = format!("{token}:");
    nonces.retain(|k, _| !k.starts_with(&prefix));
}

fn verify_attestation_claim_stub(
    state: &AppState,
    claim: Option<&edgerun_types::AttestationClaim>,
) -> bool {
    let now = now_unix_seconds();
    let policy = state
        .attestation_policy
        .lock()
        .expect("lock poisoned")
        .clone();
    let Some(claim) = claim else {
        return !policy.required;
    };

    if claim.issued_at_unix_s > now {
        return false;
    }
    if claim.expires_at_unix_s <= now {
        return false;
    }
    if now.saturating_sub(claim.issued_at_unix_s) > policy.max_age_secs {
        return false;
    }
    if !policy.allowed_measurements.is_empty() {
        let measurement = claim.measurement.trim().to_ascii_lowercase();
        if !policy
            .allowed_measurements
            .iter()
            .any(|m| m == &measurement)
        {
            return false;
        }
    }
    true
}

fn header_value<'a>(headers: &'a HeaderMap, key: &str) -> Option<&'a str> {
    headers.get(key).and_then(|v| v.to_str().ok())
}

fn touch_job_last_update(state: &AppState, job_id: &str) {
    let mut map = state.job_last_update.lock().expect("lock poisoned");
    map.insert(job_id.to_string(), now_unix_seconds());
}

fn persist_job_activity(state: &AppState, job_id: &str) -> Result<()> {
    touch_job_last_update(state, job_id);
    enforce_history_retention(state);
    evaluate_expired_jobs(state)?;
    write_state_snapshot(state)
}

fn is_duplicate_idempotency(existing: &str, incoming: &str) -> bool {
    !incoming.is_empty() && existing == incoming
}

fn scheduler_idempotency_key(
    kind: &str,
    worker_pubkey: &str,
    job_id: &str,
    phase: &str,
    discriminator: &str,
    bundle_hash: &str,
) -> String {
    let raw = format!("{kind}|{worker_pubkey}|{job_id}|{phase}|{discriminator}|{bundle_hash}");
    hex::encode(edgerun_crypto::blake3_256(raw.as_bytes()))
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
    use axum::http::{HeaderMap, HeaderValue};
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use ed25519_dalek::Signer;
    use hmac::{Hmac, Mac};
    use serde::Deserialize;
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_state() -> AppState {
        let data_dir =
            std::env::temp_dir().join(format!("edgerun-scheduler-tests-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(data_dir.join("bundles"));
        AppState {
            data_dir: data_dir.clone(),
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
            enable_slash_artifacts: true,
            require_client_signatures: false,
            client_signature_max_age_secs: 300,
            chain_auto_submit: false,
            job_timeout_secs: 60,
            session_ttl_secs: 900,
            require_policy_session: false,
            policy_session_bootstrap_token: None,
            policy_session_shared: false,
            policy_session_lock_path: data_dir.join("policy-session.lock"),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            policy_nonces: Arc::new(Mutex::new(HashMap::new())),
            policy_session_state_path: data_dir.join("policy-session-state.json"),
            policy_audit_path: data_dir.join("policy-audit.jsonl"),
            trust_policy: Arc::new(Mutex::new(edgerun_types::SyncTrustPolicy::default())),
            trust_policy_path: data_dir.join("trust-policy.json"),
            attestation_policy: Arc::new(Mutex::new(edgerun_types::AttestationPolicy::default())),
            attestation_policy_path: data_dir.join("attestation-policy.json"),
            route_shared_state_path: None,
            device_routes: Arc::new(Mutex::new(HashMap::new())),
            route_challenges: Arc::new(Mutex::new(HashMap::new())),
            route_heartbeat_tokens: Arc::new(Mutex::new(HashMap::new())),
            signal_peers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
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
                attestation_claim: None,
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
                attestation_claim: None,
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
                attestation_claim: None,
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
                attestation_claim: None,
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
                attestation_claim: None,
                signature: None,
            },
        ];

        assert!(find_winning_output_hash(&reports, 2).is_none());
    }

    #[test]
    fn tie_at_quorum_returns_no_winner() {
        let reports = vec![
            WorkerResultReport {
                idempotency_key: "a".to_string(),
                worker_pubkey: "w1".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-a".to_string(),
                output_len: 10,
                attestation_sig: None,
                attestation_claim: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "b".to_string(),
                worker_pubkey: "w2".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-a".to_string(),
                output_len: 10,
                attestation_sig: None,
                attestation_claim: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "c".to_string(),
                worker_pubkey: "w3".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-b".to_string(),
                output_len: 10,
                attestation_sig: None,
                attestation_claim: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "d".to_string(),
                worker_pubkey: "w4".to_string(),
                job_id: "j1".to_string(),
                bundle_hash: "b1".to_string(),
                output_hash: "out-b".to_string(),
                output_len: 10,
                attestation_sig: None,
                attestation_claim: None,
                signature: None,
            },
        ];

        assert!(find_winning_output_hash(&reports, 2).is_none());
    }

    #[test]
    fn randomized_ties_never_select_winner() {
        for seed in 0..64_u32 {
            let quorum = 2_usize + (seed as usize % 3);
            let mut reports = Vec::new();
            for i in 0..quorum {
                reports.push(WorkerResultReport {
                    idempotency_key: format!("a-{seed}-{i}"),
                    worker_pubkey: format!("wa-{seed}-{i}"),
                    job_id: "j1".to_string(),
                    bundle_hash: "b1".to_string(),
                    output_hash: "out-a".to_string(),
                    output_len: 10,
                    attestation_sig: None,
                    attestation_claim: None,
                    signature: None,
                });
                reports.push(WorkerResultReport {
                    idempotency_key: format!("b-{seed}-{i}"),
                    worker_pubkey: format!("wb-{seed}-{i}"),
                    job_id: "j1".to_string(),
                    bundle_hash: "b1".to_string(),
                    output_hash: "out-b".to_string(),
                    output_len: 10,
                    attestation_sig: None,
                    attestation_claim: None,
                    signature: None,
                });
            }
            assert!(find_winning_output_hash(&reports, quorum).is_none());
        }
    }

    #[test]
    fn randomized_unique_max_selects_winner() {
        for seed in 0..64_u32 {
            let quorum = 2_usize;
            let a_count = 3_usize + (seed as usize % 3);
            let b_count = 2_usize;
            let mut reports = Vec::new();
            for i in 0..a_count {
                reports.push(WorkerResultReport {
                    idempotency_key: format!("a-{seed}-{i}"),
                    worker_pubkey: format!("wa-{seed}-{i}"),
                    job_id: "j1".to_string(),
                    bundle_hash: "b1".to_string(),
                    output_hash: "out-a".to_string(),
                    output_len: 10,
                    attestation_sig: None,
                    attestation_claim: None,
                    signature: None,
                });
            }
            for i in 0..b_count {
                reports.push(WorkerResultReport {
                    idempotency_key: format!("b-{seed}-{i}"),
                    worker_pubkey: format!("wb-{seed}-{i}"),
                    job_id: "j1".to_string(),
                    bundle_hash: "b1".to_string(),
                    output_hash: "out-b".to_string(),
                    output_len: 10,
                    attestation_sig: None,
                    attestation_claim: None,
                    signature: None,
                });
            }
            let winning = find_winning_output_hash(&reports, quorum).expect("winner expected");
            assert_eq!(winning.0, "out-a");
            assert_eq!(winning.1.len(), a_count);
        }
    }

    #[test]
    fn winner_worker_set_is_unique_and_from_reports() {
        for seed in 0..64_u32 {
            let quorum = 2_usize;
            let mut reports = Vec::new();
            for i in 0..6_usize {
                reports.push(WorkerResultReport {
                    idempotency_key: format!("a-{seed}-{i}"),
                    worker_pubkey: format!("wa-{seed}-{}", i % 3),
                    job_id: "j1".to_string(),
                    bundle_hash: "b1".to_string(),
                    output_hash: if i < 4 {
                        "out-a".to_string()
                    } else {
                        "out-b".to_string()
                    },
                    output_len: 10,
                    attestation_sig: None,
                    attestation_claim: None,
                    signature: None,
                });
            }
            let winning = find_winning_output_hash(&reports, quorum).expect("winner expected");
            let winner_set: HashSet<String> = winning.1.iter().cloned().collect();
            assert_eq!(winner_set.len(), winning.1.len());
            for worker in &winning.1 {
                let exists = reports
                    .iter()
                    .any(|r| r.worker_pubkey == *worker && r.output_hash == winning.0);
                assert!(exists);
            }
        }
    }

    #[test]
    fn validate_worker_result_payload_rejects_bad_hex() {
        let payload = WorkerResultReport {
            idempotency_key: "k".to_string(),
            worker_pubkey: "w".to_string(),
            job_id: "not-hex".to_string(),
            bundle_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_string(),
            output_hash: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                .to_string(),
            output_len: 1,
            attestation_sig: None,
            attestation_claim: None,
            signature: None,
        };
        let err = validate_worker_result_payload(&payload).expect_err("must reject");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1, "job_id must be 32-byte hex");
    }

    #[test]
    fn merkle_root_and_proof_roundtrip() {
        let leaves = vec![[1_u8; 32], [2_u8; 32], [3_u8; 32], [4_u8; 32]];
        let target = [3_u8; 32];
        let (root, proof) = build_merkle_root_and_proof(&leaves, &target).expect("proof");
        let mut acc = target;
        for sibling in proof {
            acc = merkle_parent_sorted(&acc, &sibling);
        }
        assert_eq!(acc, root);
    }

    #[test]
    fn merkle_root_and_proof_missing_leaf_returns_none() {
        let leaves = vec![[1_u8; 32], [2_u8; 32]];
        let target = [9_u8; 32];
        assert!(build_merkle_root_and_proof(&leaves, &target).is_none());
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
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
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
                    slash_artifacts: Vec::new(),
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
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
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
                    slash_artifacts: Vec::new(),
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
    fn bundle_cas_rejects_overwrite_with_different_bytes() {
        let state = test_state();
        let path = state
            .data_dir
            .join("bundles")
            .join("cas-overwrite-test.cbor");

        let first = b"bundle-v1";
        let first_hash = hex::encode(edgerun_crypto::compute_bundle_hash(first));
        write_bundle_cas(&path, &first_hash, first).expect("initial write");

        let second = b"bundle-v2";
        let second_hash = hex::encode(edgerun_crypto::compute_bundle_hash(second));
        let err = write_bundle_cas(&path, &second_hash, second).expect_err("must reject drift");
        assert!(err
            .to_string()
            .contains("bundle path already exists with different bytes"));
    }

    #[tokio::test]
    async fn get_bundle_rejects_hash_mismatch_for_tampered_bytes() {
        let state = test_state();
        let canonical = b"bundle-canonical";
        let bundle_hash = hex::encode(edgerun_crypto::compute_bundle_hash(canonical));
        let path = bundle_path(&state, &bundle_hash);

        // Simulate local-disk tamper/drift at the expected bundle path.
        std::fs::write(path, b"bundle-tampered").expect("write tampered bundle");

        match get_bundle(State(state), Path(bundle_hash)).await {
            Ok(_) => panic!("tampered bytes should be rejected"),
            Err(err) => {
                assert_eq!(err.0, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(err.1, "bundle content hash mismatch");
            }
        }
    }

    #[test]
    fn posted_job_rpc_filters_include_discriminator_and_status() {
        let filters = posted_job_rpc_filters();
        assert_eq!(filters.len(), 3);
        match &filters[0] {
            RpcFilterType::DataSize(v) => assert_eq!(*v, JOB_ACCOUNT_MIN_LEN as u64),
            _ => panic!("expected data size filter"),
        }
        match &filters[2] {
            RpcFilterType::Memcmp(memcmp) => assert_eq!(
                memcmp.offset(),
                ANCHOR_DISCRIMINATOR_LEN + JOB_STATUS_OFFSET_FROM_ANCHOR
            ),
            _ => panic!("expected memcmp status filter"),
        }
    }

    #[test]
    fn job_result_rpc_filters_include_discriminator() {
        let filters = job_result_rpc_filters();
        assert_eq!(filters.len(), 2);
        match &filters[0] {
            RpcFilterType::DataSize(v) => assert_eq!(*v, JOB_RESULT_ACCOUNT_MIN_LEN as u64),
            _ => panic!("expected data size filter"),
        }
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
    fn parses_onchain_job_result_view_fields() {
        let mut data = vec![0_u8; JOB_RESULT_ACCOUNT_MIN_LEN];
        data[..ANCHOR_DISCRIMINATOR_LEN]
            .copy_from_slice(&anchor_account_discriminator("JobResult"));
        let job_id = [0x11_u8; 32];
        let worker = Pubkey::new_unique();
        let output_hash = [0x22_u8; 32];
        let sig = [0x33_u8; 64];
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_JOB_ID_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_JOB_ID_OFFSET_FROM_ANCHOR + 32]
            .copy_from_slice(&job_id);
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_WORKER_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_WORKER_OFFSET_FROM_ANCHOR + 32]
            .copy_from_slice(worker.as_ref());
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_OUTPUT_HASH_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_OUTPUT_HASH_OFFSET_FROM_ANCHOR + 32]
            .copy_from_slice(&output_hash);
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_ATTESTATION_SIG_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_ATTESTATION_SIG_OFFSET_FROM_ANCHOR + 64]
            .copy_from_slice(&sig);
        data[ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_SUBMITTED_SLOT_OFFSET_FROM_ANCHOR
            ..ANCHOR_DISCRIMINATOR_LEN + JOB_RESULT_SUBMITTED_SLOT_OFFSET_FROM_ANCHOR + 8]
            .copy_from_slice(&77_u64.to_le_bytes());

        let parsed = parse_onchain_job_result_view(&data).expect("parse");
        assert_eq!(parsed.job_id, job_id);
        assert_eq!(parsed.worker, worker);
        assert_eq!(parsed.output_hash, output_hash);
        assert_eq!(parsed.attestation_sig, sig);
        assert_eq!(parsed.submitted_slot, 77);
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
                attestation_claim: None,
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
                attestation_claim: None,
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
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
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
                    slash_artifacts: Vec::new(),
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
                attestation_claim: None,
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
                attestation_claim: None,
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
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
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
                    slash_artifacts: Vec::new(),
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
    fn onchain_result_ingest_can_drive_quorum() {
        let mut state = test_state();
        state.quorum_requires_attestation = false;
        let state = state;

        let worker1_sk = SigningKey::from_bytes(&[31_u8; 32]);
        let worker2_sk = SigningKey::from_bytes(&[32_u8; 32]);
        let worker3_sk = SigningKey::from_bytes(&[33_u8; 32]);
        let worker1 = Pubkey::new_from_array(*worker1_sk.verifying_key().as_bytes());
        let worker2 = Pubkey::new_from_array(*worker2_sk.verifying_key().as_bytes());
        let worker3 = Pubkey::new_from_array(*worker3_sk.verifying_key().as_bytes());
        let job_id = [0x90_u8; 32];
        let job_id_hex = hex::encode(job_id);
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                job_id_hex.clone(),
                JobQuorumState {
                    expected_bundle_hash:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    committee_workers: vec![
                        worker1.to_string(),
                        worker2.to_string(),
                        worker3.to_string(),
                    ],
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
                    slash_artifacts: Vec::new(),
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }

        let output_hash = [0xAB_u8; 32];
        let bundle_hash =
            parse_hex32("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .expect("hex");
        let runtime_id =
            parse_hex32("1111111111111111111111111111111111111111111111111111111111111111")
                .expect("hex");
        let msg1 = build_worker_result_digest(&job_id, &bundle_hash, &output_hash, &runtime_id);
        let msg2 = build_worker_result_digest(&job_id, &bundle_hash, &output_hash, &runtime_id);
        let sig1 = edgerun_crypto::sign(&worker1_sk, &msg1).to_bytes();
        let sig2 = edgerun_crypto::sign(&worker2_sk, &msg2).to_bytes();
        let r1 = OnchainJobResultView {
            job_id,
            worker: worker1,
            output_hash,
            attestation_sig: sig1,
            submitted_slot: 10,
        };
        let r2 = OnchainJobResultView {
            job_id,
            worker: worker2,
            output_hash,
            attestation_sig: sig2,
            submitted_slot: 11,
        };
        assert!(ingest_onchain_job_result_view(&state, &r1).is_some());
        assert!(ingest_onchain_job_result_view(&state, &r2).is_some());

        let reached = recompute_job_quorum(&state, &job_id_hex).expect("recompute");
        assert!(reached);
        let jq = state.job_quorum.lock().expect("lock poisoned");
        assert!(jq
            .get(&job_id_hex)
            .and_then(|v| v.winning_output_hash.as_ref())
            .is_some());
    }

    #[test]
    fn onchain_result_ingest_rejects_invalid_attestation() {
        let state = test_state();
        let worker_sk = SigningKey::from_bytes(&[41_u8; 32]);
        let worker = Pubkey::new_from_array(*worker_sk.verifying_key().as_bytes());
        let job_id = [0x91_u8; 32];
        let job_id_hex = hex::encode(job_id);
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                job_id_hex.clone(),
                JobQuorumState {
                    expected_bundle_hash:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    committee_workers: vec![worker.to_string()],
                    committee_size: 1,
                    quorum: 1,
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
                    slash_artifacts: Vec::new(),
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }
        let bad = OnchainJobResultView {
            job_id,
            worker,
            output_hash: [0xAB_u8; 32],
            attestation_sig: [0_u8; 64],
            submitted_slot: 99,
        };
        assert!(ingest_onchain_job_result_view(&state, &bad).is_none());
    }

    #[test]
    fn slash_artifacts_include_losing_workers() {
        let state = test_state();
        let worker_win_sk = SigningKey::from_bytes(&[51_u8; 32]);
        let worker_lose_sk = SigningKey::from_bytes(&[52_u8; 32]);
        let worker_other_sk = SigningKey::from_bytes(&[53_u8; 32]);
        let worker_win = Pubkey::new_from_array(*worker_win_sk.verifying_key().as_bytes());
        let worker_lose = Pubkey::new_from_array(*worker_lose_sk.verifying_key().as_bytes());
        let worker_other = Pubkey::new_from_array(*worker_other_sk.verifying_key().as_bytes());
        let job_id = [0xA1_u8; 32];
        let job_id_hex = hex::encode(job_id);
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                job_id_hex.clone(),
                JobQuorumState {
                    expected_bundle_hash:
                        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                            .to_string(),
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    committee_workers: vec![
                        worker_win.to_string(),
                        worker_lose.to_string(),
                        worker_other.to_string(),
                    ],
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
                    slash_artifacts: Vec::new(),
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }

        let winning_hash =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
        let losing_hash =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();

        let runtime_id =
            parse_hex32("1111111111111111111111111111111111111111111111111111111111111111")
                .expect("hex");
        let bundle_hash =
            parse_hex32("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
                .expect("hex");
        let msg_win = build_worker_result_digest(
            &job_id,
            &bundle_hash,
            &parse_hex32(&winning_hash).expect("hex"),
            &runtime_id,
        );
        let msg_lose = build_worker_result_digest(
            &job_id,
            &bundle_hash,
            &parse_hex32(&losing_hash).expect("hex"),
            &runtime_id,
        );
        let sig_win = base64::engine::general_purpose::STANDARD
            .encode(edgerun_crypto::sign(&worker_win_sk, &msg_win).to_bytes());
        let sig_lose = base64::engine::general_purpose::STANDARD
            .encode(edgerun_crypto::sign(&worker_lose_sk, &msg_lose).to_bytes());

        let reports = vec![
            WorkerResultReport {
                idempotency_key: "s1".to_string(),
                worker_pubkey: worker_win.to_string(),
                job_id: job_id_hex.clone(),
                bundle_hash: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_string(),
                output_hash: winning_hash.clone(),
                output_len: 1,
                attestation_sig: Some(sig_win),
                attestation_claim: None,
                signature: None,
            },
            WorkerResultReport {
                idempotency_key: "s2".to_string(),
                worker_pubkey: worker_lose.to_string(),
                job_id: job_id_hex.clone(),
                bundle_hash: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_string(),
                output_hash: losing_hash,
                output_len: 1,
                attestation_sig: Some(sig_lose),
                attestation_claim: None,
                signature: None,
            },
        ];

        let artifacts = build_slash_worker_artifacts(
            &state,
            &job_id_hex,
            &winning_hash,
            &[
                worker_win.to_string(),
                worker_lose.to_string(),
                worker_other.to_string(),
            ],
            &reports,
        );
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].worker_pubkey, worker_lose.to_string());
        assert_eq!(artifacts[0].tx, "UNAVAILABLE_NO_CHAIN_CONTEXT");
    }

    #[test]
    fn verifies_result_attestation() {
        let state = test_state();
        let signing_key = SigningKey::from_bytes(&[9_u8; 32]);
        let worker = Pubkey::new_from_array(*signing_key.verifying_key().as_bytes());
        let job_id = [1_u8; 32];
        let output_hash = [2_u8; 32];
        let bundle_hash =
            parse_hex32("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                .expect("hex");
        let runtime_id =
            parse_hex32("1111111111111111111111111111111111111111111111111111111111111111")
                .expect("hex");
        let message = build_worker_result_digest(&job_id, &bundle_hash, &output_hash, &runtime_id);
        let sig = edgerun_crypto::sign(&signing_key, &message);
        let attestation_sig = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let payload = WorkerResultReport {
            idempotency_key: "ik".to_string(),
            worker_pubkey: worker.to_string(),
            job_id: hex::encode(job_id),
            bundle_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_string(),
            output_hash: hex::encode(output_hash),
            output_len: 7,
            attestation_sig: Some(attestation_sig),
            attestation_claim: None,
            signature: None,
        };
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                payload.job_id.clone(),
                JobQuorumState {
                    expected_bundle_hash:
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    committee_workers: vec![worker.to_string()],
                    committee_size: 1,
                    quorum: 1,
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
                    slash_artifacts: Vec::new(),
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }

        let ok = verify_result_attestation(&state, &payload).expect("attestation verify");
        assert!(ok);
    }

    #[test]
    fn rejects_result_attestation_when_bundle_hash_mismatch() {
        let state = test_state();
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let worker = Pubkey::new_from_array(*signing_key.verifying_key().as_bytes());
        let job_id = [3_u8; 32];
        let expected_bundle_hash =
            parse_hex32("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                .expect("hex");
        let wrong_bundle_hash =
            parse_hex32("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
                .expect("hex");
        let output_hash = [4_u8; 32];
        let runtime_id =
            parse_hex32("1111111111111111111111111111111111111111111111111111111111111111")
                .expect("hex");
        let message =
            build_worker_result_digest(&job_id, &wrong_bundle_hash, &output_hash, &runtime_id);
        let sig = edgerun_crypto::sign(&signing_key, &message);
        let attestation_sig = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

        let payload = WorkerResultReport {
            idempotency_key: "ik-bundle-mismatch".to_string(),
            worker_pubkey: worker.to_string(),
            job_id: hex::encode(job_id),
            bundle_hash: hex::encode(wrong_bundle_hash),
            output_hash: hex::encode(output_hash),
            output_len: 7,
            attestation_sig: Some(attestation_sig),
            attestation_claim: None,
            signature: None,
        };
        {
            let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
            job_quorum.insert(
                payload.job_id.clone(),
                JobQuorumState {
                    expected_bundle_hash: hex::encode(expected_bundle_hash),
                    expected_runtime_id:
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    committee_workers: vec![worker.to_string()],
                    committee_size: 1,
                    quorum: 1,
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
                    slash_artifacts: Vec::new(),
                    created_at_unix_s: now_unix_seconds(),
                    quorum_reached_at_unix_s: None,
                },
            );
        }

        let ok = verify_result_attestation(&state, &payload).expect("attestation verify");
        assert!(!ok);
    }

    #[test]
    fn trust_policy_file_roundtrip() {
        let state = test_state();
        let path = state.data_dir.join("trust-policy-roundtrip.json");
        let expected = edgerun_types::SyncTrustPolicy::strict(true);
        save_trust_policy(&path, &expected).expect("save");
        let loaded = load_trust_policy(&path).expect("load");
        assert_eq!(loaded, expected);
    }

    #[test]
    fn policy_session_state_file_roundtrip() {
        let state = test_state();
        let path = state.data_dir.join("policy-session-roundtrip.json");

        let mut sessions = HashMap::new();
        sessions.insert(
            "tok-1".to_string(),
            edgerun_hwvault_primitives::session::SessionState {
                expires_at: 123,
                signing_key: "sk-1".to_string(),
                bound_origin: Some("https://app.example".to_string()),
            },
        );
        let mut nonces = HashMap::new();
        nonces.insert("tok-1:n-1".to_string(), 999);
        let snapshot = PersistedPolicySessionState { sessions, nonces };

        save_policy_session_state(&path, &snapshot).expect("save");
        let loaded = load_policy_session_state(&path).expect("load");
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.nonces.len(), 1);
        assert_eq!(
            loaded.sessions.get("tok-1").expect("session").signing_key,
            "sk-1"
        );
    }

    #[tokio::test]
    async fn strict_trust_policy_requires_client_signature() {
        let state = test_state();
        {
            let mut policy = state.trust_policy.lock().expect("lock poisoned");
            *policy = edgerun_types::SyncTrustPolicy::strict(true);
        }

        let payload = JobCreateRequest {
            runtime_id: "00".repeat(32),
            wasm_base64: base64::engine::general_purpose::STANDARD.encode([0x00_u8]),
            input_base64: base64::engine::general_purpose::STANDARD.encode([0x01_u8]),
            abi_version: Some(edgerun_types::BUNDLE_ABI_CURRENT),
            limits: edgerun_types::Limits {
                max_memory_bytes: 1024,
                max_instructions: 10_000,
            },
            escrow_lamports: 1,
            assignment_worker_pubkey: None,
            client_pubkey: None,
            client_signed_at_unix_s: None,
            client_signature: None,
        };

        let err = job_create(State(state), Json(payload))
            .await
            .expect_err("strict policy must require auth");
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
        assert!(err.1.contains("strict trust policy"));
    }

    #[tokio::test]
    async fn trust_policy_endpoints_require_valid_session() {
        let mut state = test_state();
        state.require_policy_session = true;

        let err = trust_policy_get(State(state.clone()), HeaderMap::new())
            .await
            .expect_err("missing session must fail");
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn trust_policy_set_and_get_with_signed_session_headers() {
        let mut state = test_state();
        state.require_policy_session = true;

        let issued = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("session create must succeed")
        .0;

        let set_body = serde_json::to_vec(&TrustPolicySetRequest {
            profile: "strict".to_string(),
        })
        .expect("serialize trust policy payload");
        let headers_set = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "POST",
            "/v1/trust/policy/set",
            "nonce-set-1",
            &set_body,
        );
        let set_resp = trust_policy_set(State(state.clone()), headers_set, Bytes::from(set_body))
            .await
            .expect("set must succeed")
            .0;
        assert!(matches!(
            set_resp.policy.profile,
            edgerun_types::SyncTrustProfile::Strict
        ));

        let headers_get = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "GET",
            "/v1/trust/policy/get",
            "nonce-get-1",
            &[],
        );
        let get_resp = trust_policy_get(State(state.clone()), headers_get)
            .await
            .expect("get must succeed")
            .0;
        assert!(matches!(
            get_resp.policy.profile,
            edgerun_types::SyncTrustProfile::Strict
        ));

        let persisted = load_trust_policy(&state.trust_policy_path).expect("load trust policy");
        assert!(matches!(
            persisted.profile,
            edgerun_types::SyncTrustProfile::Strict
        ));
        assert!(persisted.configured);
    }

    #[tokio::test]
    async fn trust_policy_endpoints_reject_replayed_nonce() {
        let mut state = test_state();
        state.require_policy_session = true;

        let issued = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("session create must succeed")
        .0;

        let headers = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "GET",
            "/v1/trust/policy/get",
            "nonce-replay-1",
            &[],
        );

        let first = trust_policy_get(State(state.clone()), headers.clone())
            .await
            .expect("first request should pass")
            .0;
        assert!(matches!(
            first.policy.profile,
            edgerun_types::SyncTrustProfile::Balanced
        ));

        let second = trust_policy_get(State(state.clone()), headers)
            .await
            .expect_err("replayed nonce should fail");
        assert_eq!(second.0, StatusCode::UNAUTHORIZED);
        assert!(second.1.contains("replay"));
    }

    #[tokio::test]
    async fn trust_policy_set_rejects_tampered_body_with_same_signature() {
        let mut state = test_state();
        state.require_policy_session = true;

        let issued = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("session create must succeed")
        .0;

        let signed_body = serde_json::to_vec(&TrustPolicySetRequest {
            profile: "strict".to_string(),
        })
        .expect("serialize signed payload");
        let tampered_body = serde_json::to_vec(&TrustPolicySetRequest {
            profile: "monitor".to_string(),
        })
        .expect("serialize tampered payload");
        let headers = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "POST",
            "/v1/trust/policy/set",
            "nonce-tamper-1",
            &signed_body,
        );

        let err = trust_policy_set(State(state.clone()), headers, Bytes::from(tampered_body))
            .await
            .expect_err("tampered body should be rejected");
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
        assert!(err.1.contains("invalid signature"));
    }

    #[tokio::test]
    async fn session_create_requires_bootstrap_token_when_configured() {
        let mut state = test_state();
        state.policy_session_bootstrap_token = Some("bootstrap-secret".to_string());

        let err = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect_err("missing bootstrap token must fail");
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-edgerun-bootstrap-token",
            HeaderValue::from_static("bootstrap-secret"),
        );
        let issued = session_create(
            State(state.clone()),
            headers,
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("bootstrap token should allow session")
        .0;
        assert!(!issued.token.is_empty());
        assert!(!issued.session_key.is_empty());
    }

    #[tokio::test]
    async fn session_rotate_replaces_and_revokes_old_token() {
        let mut state = test_state();
        state.require_policy_session = true;

        let issued = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("session create must succeed")
        .0;

        let headers_rotate = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "POST",
            "/v1/session/rotate",
            "nonce-rotate-1",
            &[],
        );
        let rotated = session_rotate(
            State(state.clone()),
            headers_rotate,
            Json(SessionRotateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("rotate must succeed")
        .0;
        assert_ne!(issued.token, rotated.token);
        assert_ne!(issued.session_key, rotated.session_key);

        let old_headers = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "GET",
            "/v1/trust/policy/get",
            "nonce-old-1",
            &[],
        );
        let old_err = trust_policy_get(State(state.clone()), old_headers)
            .await
            .expect_err("old session must be revoked");
        assert_eq!(old_err.0, StatusCode::UNAUTHORIZED);

        let new_headers = signed_policy_headers(
            &rotated.token,
            &rotated.session_key,
            "https://app.example",
            "GET",
            "/v1/trust/policy/get",
            "nonce-new-1",
            &[],
        );
        let _ = trust_policy_get(State(state.clone()), new_headers)
            .await
            .expect("new session must work");
    }

    #[tokio::test]
    async fn session_invalidate_revokes_token() {
        let mut state = test_state();
        state.require_policy_session = true;

        let issued = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("session create must succeed")
        .0;

        let headers_invalidate = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "POST",
            "/v1/session/invalidate",
            "nonce-invalidate-1",
            &[],
        );
        let resp = session_invalidate(
            State(state.clone()),
            headers_invalidate,
            Json(SessionInvalidateRequest { token: None }),
        )
        .await
        .expect("invalidate must succeed")
        .0;
        assert!(resp.ok);

        let old_headers = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "GET",
            "/v1/trust/policy/get",
            "nonce-after-invalidate",
            &[],
        );
        let old_err = trust_policy_get(State(state.clone()), old_headers)
            .await
            .expect_err("invalidated session must fail");
        assert_eq!(old_err.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn attestation_policy_set_and_get_with_signed_session_headers() {
        let mut state = test_state();
        state.require_policy_session = true;

        let issued = session_create(
            State(state.clone()),
            HeaderMap::new(),
            Json(SessionCreateRequest {
                bound_origin: Some("https://app.example".to_string()),
            }),
        )
        .await
        .expect("session create must succeed")
        .0;

        let set_body = serde_json::to_vec(&AttestationPolicySetRequest {
            required: true,
            max_age_secs: 120,
            allowed_measurements: vec!["M1".to_string(), "m1".to_string()],
        })
        .expect("serialize attestation policy payload");
        let headers_set = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "POST",
            "/v1/attestation/policy/set",
            "nonce-att-set-1",
            &set_body,
        );
        let set_resp =
            attestation_policy_set(State(state.clone()), headers_set, Bytes::from(set_body))
                .await
                .expect("set must succeed")
                .0;
        assert!(set_resp.policy.required);
        assert_eq!(set_resp.policy.max_age_secs, 120);
        assert_eq!(set_resp.policy.allowed_measurements, vec!["m1".to_string()]);

        let headers_get = signed_policy_headers(
            &issued.token,
            &issued.session_key,
            "https://app.example",
            "GET",
            "/v1/attestation/policy/get",
            "nonce-att-get-1",
            &[],
        );
        let get_resp = attestation_policy_get(State(state.clone()), headers_get)
            .await
            .expect("get must succeed")
            .0;
        assert!(get_resp.policy.required);
        assert_eq!(get_resp.policy.max_age_secs, 120);
        assert_eq!(get_resp.policy.allowed_measurements, vec!["m1".to_string()]);
    }

    #[test]
    fn result_attestation_claim_policy_is_enforced() {
        let state = test_state();
        let now = now_unix_seconds();

        let payload = WorkerResultReport {
            idempotency_key: "claim-1".to_string(),
            worker_pubkey: "11111111111111111111111111111111".to_string(),
            job_id: "00".repeat(32),
            bundle_hash: "11".repeat(32),
            output_hash: "22".repeat(32),
            output_len: 1,
            attestation_sig: None,
            attestation_claim: None,
            signature: None,
        };

        {
            let mut p = state.attestation_policy.lock().expect("lock poisoned");
            *p = edgerun_types::AttestationPolicy {
                required: true,
                max_age_secs: 300,
                allowed_measurements: vec!["tee-good".to_string()],
            };
        }
        let missing_claim = verify_result_attestation(&state, &payload).expect("verify");
        assert!(!missing_claim);

        let mut bad_measurement = payload.clone();
        bad_measurement.attestation_claim = Some(edgerun_types::AttestationClaim {
            measurement: "tee-bad".to_string(),
            issued_at_unix_s: now.saturating_sub(5),
            expires_at_unix_s: now.saturating_add(30),
            nonce: None,
            format: Some("stub".to_string()),
            evidence: None,
        });
        let bad = verify_result_attestation(&state, &bad_measurement).expect("verify");
        assert!(!bad);

        let mut good = payload.clone();
        good.attestation_claim = Some(edgerun_types::AttestationClaim {
            measurement: "tee-good".to_string(),
            issued_at_unix_s: now.saturating_sub(5),
            expires_at_unix_s: now.saturating_add(30),
            nonce: None,
            format: Some("stub".to_string()),
            evidence: None,
        });
        let ok = verify_result_attestation(&state, &good).expect("verify");
        assert!(ok);
    }

    fn signed_policy_headers(
        token: &str,
        session_key: &str,
        origin: &str,
        method: &str,
        path: &str,
        nonce: &str,
        body: &[u8],
    ) -> HeaderMap {
        type HmacSha256 = Hmac<Sha256>;

        let ts = now_unix_seconds().to_string();
        let body_hash = {
            let mut hasher = Sha256::new();
            hasher.update(body);
            URL_SAFE_NO_PAD.encode(hasher.finalize())
        };
        let canonical = format!("{method}|{path}|{ts}|{nonce}|{body_hash}");
        let sig = {
            let mut mac = HmacSha256::new_from_slice(session_key.as_bytes()).expect("hmac key");
            mac.update(canonical.as_bytes());
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {token}")).expect("auth header"),
        );
        headers.insert(
            "origin",
            HeaderValue::from_str(origin).expect("origin header"),
        );
        headers.insert("x-hwv-ts", HeaderValue::from_str(&ts).expect("ts header"));
        headers.insert(
            "x-hwv-nonce",
            HeaderValue::from_str(nonce).expect("nonce header"),
        );
        headers.insert(
            "x-hwv-sig",
            HeaderValue::from_str(&sig).expect("sig header"),
        );
        headers
    }

    #[tokio::test]
    async fn route_registration_flow_challenge_register_resolve_owner_heartbeat() {
        let state = test_state();
        let device_id = "device-abc".to_string();
        let owner_signing = SigningKey::from_bytes(&[7_u8; 32]);
        let owner_pubkey = URL_SAFE_NO_PAD.encode(owner_signing.verifying_key().to_bytes());

        let Ok(Json(challenge)) = route_challenge(
            State(state.clone()),
            Json(RouteChallengeRequest {
                device_id: device_id.clone(),
            }),
        )
        .await
        else {
            panic!("route challenge should succeed");
        };
        assert!(!challenge.nonce.is_empty());
        assert!(challenge.expires_at_unix_s > now_unix_seconds());

        let reachable_urls = vec!["http://127.0.0.1:8091".to_string()];
        let signed_at = now_unix_seconds();
        let message = route_register_signing_message(
            &owner_pubkey,
            &device_id,
            &reachable_urls,
            &challenge.nonce,
            signed_at,
        );
        let signature = URL_SAFE_NO_PAD.encode(owner_signing.sign(message.as_bytes()).to_bytes());

        let Ok(Json(registered)) = route_register(
            State(state.clone()),
            Json(RouteRegisterRequest {
                device_id: device_id.clone(),
                owner_pubkey: owner_pubkey.clone(),
                reachable_urls: reachable_urls.clone(),
                capabilities: vec!["terminal-ws".to_string()],
                relay_session_id: None,
                ttl_secs: Some(60),
                challenge_nonce: challenge.nonce.clone(),
                signed_at_unix_s: signed_at,
                signature,
            }),
        )
        .await
        else {
            panic!("route registration should succeed");
        };
        assert!(registered.ok);
        assert_eq!(registered.device_id, device_id);
        assert!(!registered.heartbeat_token.is_empty());

        let Json(resolved) = route_resolve(State(state.clone()), Path(device_id.clone())).await;
        assert!(resolved.ok);
        assert!(resolved.found);
        let resolved_entry = resolved.route.expect("route entry");
        assert_eq!(resolved_entry.device_id, device_id);
        assert_eq!(resolved_entry.owner_pubkey, owner_pubkey);
        assert_eq!(resolved_entry.reachable_urls, reachable_urls);

        let Json(owner_routes) =
            route_owner_resolve(State(state.clone()), Path(owner_pubkey.clone())).await;
        assert!(owner_routes.ok);
        assert_eq!(owner_routes.owner_pubkey, owner_pubkey);
        assert_eq!(owner_routes.devices.len(), 1);
        assert_eq!(owner_routes.devices[0].device_id, device_id);

        let Ok(Json(heartbeat)) = route_heartbeat(
            State(state.clone()),
            Json(RouteHeartbeatRequest {
                device_id: device_id.clone(),
                token: registered.heartbeat_token,
                ttl_secs: Some(90),
            }),
        )
        .await
        else {
            panic!("route heartbeat should succeed");
        };
        assert!(heartbeat.ok);
        assert_eq!(heartbeat.device_id, device_id);
        assert!(heartbeat.expires_at_unix_s > now_unix_seconds());
    }

    #[tokio::test]
    async fn route_registration_shared_state_works_across_scheduler_instances() {
        let shared_path = std::env::temp_dir().join(format!(
            "edgerun-route-shared-{}-{}.json",
            std::process::id(),
            now_unix_seconds()
        ));
        let _ = std::fs::remove_file(&shared_path);

        let mut state_a = test_state();
        state_a.route_shared_state_path = Some(shared_path.clone());
        let mut state_b = test_state();
        state_b.route_shared_state_path = Some(shared_path.clone());

        let device_id = "device-cross-instance".to_string();
        let owner_signing = SigningKey::from_bytes(&[9_u8; 32]);
        let owner_pubkey = URL_SAFE_NO_PAD.encode(owner_signing.verifying_key().to_bytes());

        // Challenge on A
        let Ok(Json(challenge)) = route_challenge(
            State(state_a.clone()),
            Json(RouteChallengeRequest {
                device_id: device_id.clone(),
            }),
        )
        .await
        else {
            panic!("route challenge should succeed");
        };

        // Register on B using challenge from A
        let reachable_urls = vec!["http://127.0.0.1:9001".to_string()];
        let signed_at = now_unix_seconds();
        let message = route_register_signing_message(
            &owner_pubkey,
            &device_id,
            &reachable_urls,
            &challenge.nonce,
            signed_at,
        );
        let signature = URL_SAFE_NO_PAD.encode(owner_signing.sign(message.as_bytes()).to_bytes());
        let Ok(Json(registered)) = route_register(
            State(state_b.clone()),
            Json(RouteRegisterRequest {
                device_id: device_id.clone(),
                owner_pubkey: owner_pubkey.clone(),
                reachable_urls: reachable_urls.clone(),
                capabilities: vec!["terminal-ws".to_string()],
                relay_session_id: None,
                ttl_secs: Some(60),
                challenge_nonce: challenge.nonce,
                signed_at_unix_s: signed_at,
                signature,
            }),
        )
        .await
        else {
            panic!("route register should succeed");
        };

        // Resolve on A should see route created on B.
        let Json(resolved) = route_resolve(State(state_a.clone()), Path(device_id.clone())).await;
        assert!(resolved.ok && resolved.found);
        assert_eq!(
            resolved
                .route
                .as_ref()
                .map(|r| r.reachable_urls.clone())
                .unwrap_or_default(),
            reachable_urls
        );

        // Heartbeat on A should accept token issued on B.
        let Ok(Json(hb)) = route_heartbeat(
            State(state_a.clone()),
            Json(RouteHeartbeatRequest {
                device_id: device_id.clone(),
                token: registered.heartbeat_token,
                ttl_secs: Some(90),
            }),
        )
        .await
        else {
            panic!("cross-instance heartbeat should succeed");
        };
        assert!(hb.ok);

        // Owner lookup on B should include route.
        let Json(owner_routes) = route_owner_resolve(State(state_b), Path(owner_pubkey)).await;
        assert_eq!(owner_routes.devices.len(), 1);
        assert_eq!(owner_routes.devices[0].device_id, device_id);

        let _ = std::fs::remove_file(shared_path);
    }

    #[derive(Debug, Deserialize)]
    struct RouteChallengeHttpResponse {
        nonce: String,
    }

    #[derive(Debug, Deserialize)]
    struct RouteResolveHttpResponse {
        found: bool,
        route: Option<RouteResolveHttpEntry>,
    }

    #[derive(Debug, Deserialize)]
    struct RouteResolveHttpEntry {
        reachable_urls: Vec<String>,
    }

    #[tokio::test]
    async fn route_resolution_enables_real_service_to_service_connectivity() {
        let state = test_state();
        let scheduler_app = Router::new()
            .route("/v1/route/challenge", post(route_challenge))
            .route("/v1/route/register", post(route_register))
            .route("/v1/route/resolve/{device_id}", get(route_resolve))
            .with_state(state);
        let scheduler_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind scheduler listener");
        let scheduler_addr = scheduler_listener
            .local_addr()
            .expect("scheduler local addr");
        let scheduler_task = tokio::spawn(async move {
            axum::serve(scheduler_listener, scheduler_app)
                .await
                .expect("scheduler serve");
        });

        let service_b_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind service B listener");
        let service_b_addr = service_b_listener.local_addr().expect("service B local addr");
        let service_b_task = tokio::spawn(async move {
            let (mut socket, _) = service_b_listener.accept().await.expect("accept service B");
            let mut buf = [0_u8; 32];
            let n = socket.read(&mut buf).await.expect("read from service A");
            assert_eq!(&buf[..n], b"ping-from-service-a");
            socket
                .write_all(b"pong-from-service-b")
                .await
                .expect("write to service A");
        });

        let scheduler_base = format!("http://{scheduler_addr}");
        let client = reqwest::Client::new();
        let owner_signing = SigningKey::from_bytes(&[11_u8; 32]);
        let owner_pubkey = URL_SAFE_NO_PAD.encode(owner_signing.verifying_key().to_bytes());
        let service_b_device_id = "service-b".to_string();
        let service_b_reachable_url = format!("http://{}", service_b_addr);

        let challenge = client
            .post(format!("{scheduler_base}/v1/route/challenge"))
            .json(&serde_json::json!({ "device_id": service_b_device_id }))
            .send()
            .await
            .expect("request challenge")
            .error_for_status()
            .expect("challenge status")
            .json::<RouteChallengeHttpResponse>()
            .await
            .expect("parse challenge response");

        let signed_at = now_unix_seconds();
        let message = route_register_signing_message(
            &owner_pubkey,
            &service_b_device_id,
            std::slice::from_ref(&service_b_reachable_url),
            &challenge.nonce,
            signed_at,
        );
        let signature = URL_SAFE_NO_PAD.encode(owner_signing.sign(message.as_bytes()).to_bytes());

        client
            .post(format!("{scheduler_base}/v1/route/register"))
            .json(&serde_json::json!({
                "device_id": service_b_device_id,
                "owner_pubkey": owner_pubkey,
                "reachable_urls": [service_b_reachable_url],
                "challenge_nonce": challenge.nonce,
                "signed_at_unix_s": signed_at,
                "signature": signature,
                "ttl_secs": 90
            }))
            .send()
            .await
            .expect("request register")
            .error_for_status()
            .expect("register status");

        let resolved = client
            .get(format!(
                "{scheduler_base}/v1/route/resolve/{service_b_device_id}"
            ))
            .send()
            .await
            .expect("request resolve")
            .error_for_status()
            .expect("resolve status")
            .json::<RouteResolveHttpResponse>()
            .await
            .expect("parse resolve response");

        assert!(resolved.found);
        let target_url = resolved
            .route
            .and_then(|entry| entry.reachable_urls.into_iter().next())
            .expect("resolved reachable URL");
        let target = reqwest::Url::parse(&target_url).expect("valid reachable URL");
        let host = target.host_str().expect("target host");
        let port = target.port_or_known_default().expect("target port");

        let mut stream = tokio::net::TcpStream::connect((host, port))
            .await
            .expect("service A connects to service B via resolved route");
        stream
            .write_all(b"ping-from-service-a")
            .await
            .expect("service A writes ping");
        let mut reply = [0_u8; 32];
        let n = stream
            .read(&mut reply)
            .await
            .expect("service A reads response");
        assert_eq!(&reply[..n], b"pong-from-service-b");

        service_b_task.await.expect("service B task");
        scheduler_task.abort();
        let _ = scheduler_task.await;
    }
}
