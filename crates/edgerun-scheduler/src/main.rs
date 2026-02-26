// SPDX-License-Identifier: LicenseRef-Edgerun-Proprietary
#![allow(deprecated)]

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::OpenOptions;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Query,
    extract::State,
    http::{header, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer as DalekSigner, SigningKey, VerifyingKey};
use edgerun_hwvault_primitives::hardware::random_token_b64url;
use edgerun_storage::durability::DurabilityLevel;
use edgerun_storage::event::{
    ActorId as StorageActorId, Event as StorageEvent, StreamId as StorageStreamId,
};
use edgerun_storage::event_bus::{
    BusPhaseV1, BusQueryFilter, EventBus, EventBusPolicyV1, PolicyRuleV1, PolicyUpdateRequestV1,
    StorageBackedEventBus,
};
use edgerun_storage::StorageEngine;
use edgerun_transport_core::{
    assignments_signing_message, failure_signing_message, heartbeat_signing_message,
    replay_signing_message, result_signing_message, route_register_signing_message,
};
use edgerun_types::control_plane::{
    assignment_policy_message, default_policy_key_id, default_policy_version, AssignmentsResponse,
    BundleGetResponse, ControlWsClientMessage, ControlWsRequestPayload, ControlWsResponsePayload,
    ControlWsServerMessage, HeartbeatRequest, HeartbeatResponse, JobCreateRequest,
    JobCreateResponse, JobQuorumState, JobStatusRequest, JobStatusResponse, QueuedAssignment,
    RouteCandidate as CpRouteCandidate, RouteChallengeRequest, RouteChallengeResponse,
    RouteHeartbeatRequest, RouteHeartbeatResponse, RouteOwnerRequest, RouteRegisterRequest,
    RouteRegisterResponse, RouteResolveEntry as CpRouteEntry, RouteResolveRequest,
    RouteResolveResponse as CpRouteResolveResponse, SlashWorkerArtifact, SubmissionAck,
    WorkerAssignmentsRequest, WorkerFailureReport, WorkerReplayArtifactReport, WorkerResultReport,
};
use fs2::FileExt;
use prost::Message as ProstCodec;
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
use tower_http::cors::{Any, CorsLayer};

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
    snapshot_flush_state: Arc<Mutex<SnapshotFlushState>>,
    state_snapshot_min_interval_secs: u64,
    worker_registry: Arc<Mutex<HashMap<String, WorkerRegistryEntry>>>,
    job_quorum: Arc<Mutex<HashMap<String, JobQuorumState>>>,
    policy_signing_key: SigningKey,
    policy_key_id: String,
    policy_version: u32,
    policy_ttl_secs: u64,
    pricing_lamports_per_billion_instructions: u64,
    pricing_flat_lamports: u64,
    committee_size: usize,
    quorum: usize,
    heartbeat_ttl_secs: u64,
    assignments_signature_max_age_secs: u64,
    require_assignments_signatures: bool,
    max_assignments_per_worker: usize,
    max_assignments_total: usize,
    require_worker_signatures: bool,
    require_result_attestation: bool,
    quorum_requires_attestation: bool,
    enable_slash_artifacts: bool,
    require_client_signatures: bool,
    client_signature_max_age_secs: u64,
    chain_auto_submit: bool,
    job_timeout_secs: u64,
    policy_audit_path: PathBuf,
    trust_policy: Arc<Mutex<edgerun_types::SyncTrustPolicy>>,
    attestation_policy: Arc<Mutex<edgerun_types::AttestationPolicy>>,
    route_shared_state_path: PathBuf,
    route_flush_state: Arc<Mutex<SnapshotFlushState>>,
    route_sync_min_interval_secs: u64,
    housekeeping_interval_secs: u64,
    route_signature_max_age_secs: u64,
    route_challenge_ttl_secs: u64,
    device_routes: Arc<Mutex<HashMap<String, DeviceRouteEntry>>>,
    route_challenges: Arc<Mutex<HashMap<String, u64>>>,
    route_heartbeat_tokens: Arc<Mutex<HashMap<String, RouteHeartbeatToken>>>,
    signal_peers: Arc<tokio::sync::Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
    chain_progress_sink: Arc<Mutex<Option<ChainProgressSink>>>,
    chain_event_bus_sink: Arc<Mutex<Option<ChainEventBusSink>>>,
    latest_chain_progress_event_id: Arc<Mutex<Option<String>>>,
    require_chain_progress_signature_verification: bool,
    enforce_event_ingestion: bool,
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

struct ChainProgressSink {
    session: edgerun_storage::EngineAppendSession,
    stream_id: StorageStreamId,
    actor_id: StorageActorId,
    signer_pubkey: String,
    seq: u64,
}

struct ChainEventBusSink {
    bus: StorageBackedEventBus,
    next_nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchedulerSignedChainProgressEvent {
    progress_event_id: String,
    slot: u64,
    epoch: u64,
    observed_at_unix_ms: u64,
    signer: String,
    signature: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct SnapshotFlushState {
    dirty: bool,
    last_write_unix_s: u64,
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
    latest_chain_progress_event_id: Option<String>,
    latest_chain_progress_bus_nonce: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceRouteEntry {
    device_id: String,
    owner_pubkey: String,
    #[serde(default)]
    candidates: Vec<CpRouteCandidate>,
    #[serde(default)]
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

#[derive(Debug, Serialize, Deserialize)]
struct RouteResolveResponse {
    ok: bool,
    found: bool,
    route: Option<DeviceRouteEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OwnerRoutesResponse {
    ok: bool,
    owner_pubkey: String,
    devices: Vec<DeviceRouteEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteHeartbeatToken {
    device_id: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerHeartbeatIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: HeartbeatRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerAssignmentsPollIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: WorkerAssignmentsRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerResultIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: WorkerResultReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerFailureIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: WorkerFailureReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerReplayIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: WorkerReplayArtifactReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteChallengeIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    device_id: String,
    nonce: String,
    expires_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteRegisterIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: RouteRegisterRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteHeartbeatIngestEvent {
    schema_version: u32,
    observed_at_unix_ms: u64,
    payload: RouteHeartbeatRequest,
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
struct ControlWsConnectQuery {
    #[serde(default)]
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct JsonControlWsClientMessage {
    request_id: String,
    op: String,
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonControlWsServerMessage {
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct WebRtcSignalClientMessage {
    to_device_id: String,
    to_owner_pubkey: String,
    kind: String,
    sdp: Option<String>,
    candidate: Option<String>,
    sdp_mid: Option<String>,
    sdp_mline_index: Option<u16>,
    metadata: Option<String>,
}

#[derive(Debug)]
struct WebRtcSignalServerMessage {
    from_device_id: String,
    kind: String,
    sdp: Option<String>,
    candidate: Option<String>,
    sdp_mid: Option<String>,
    sdp_mline_index: Option<u16>,
    metadata: Option<String>,
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

const INSTRUCTION_PRICE_QUANTUM: u128 = 1_000_000_000;
static STATE_SNAPSHOT_TMP_SEQ: AtomicU64 = AtomicU64::new(1);

#[tokio::main]
async fn main() -> Result<()> {
    edgerun_observability::init_service("edgerun-scheduler")?;

    let data_dir = std::env::var("EDGERUN_SCHEDULER_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".edgerun-scheduler-data"));
    std::fs::create_dir_all(data_dir.join("bundles"))?;
    let route_shared_state_path = std::env::var("EDGERUN_SCHEDULER_ROUTE_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir.join("route-state.bin"));
    if let Some(parent) = route_shared_state_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    tracing::info!(
        path = %route_shared_state_path.display(),
        "route-state persistence enabled"
    );

    let persisted = load_state(&data_dir)?;
    let require_chain = read_env_bool("EDGERUN_SCHEDULER_REQUIRE_CHAIN_CONTEXT", true);

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
                anyhow::bail!("chain context required but unavailable: {err}");
            } else {
                tracing::warn!(
                    error = %err,
                    "chain context unavailable; chain-backed operations will fail"
                );
                None
            }
        }
    };

    let addr: SocketAddr = std::env::var("EDGERUN_SCHEDULER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:5566".to_string())
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
    let trust_policy_path = data_dir.join("trust-policy.bin");
    let trust_policy = load_trust_policy(&trust_policy_path).unwrap_or_default();
    let attestation_policy_path = data_dir.join("attestation-policy.bin");
    let attestation_policy = load_attestation_policy(&attestation_policy_path).unwrap_or_default();
    let loaded_route_state = load_route_state(&route_shared_state_path).unwrap_or_default();
    let require_chain_progress_signature_verification = read_env_bool(
        "EDGERUN_SCHEDULER_REQUIRE_CHAIN_PROGRESS_SIGNATURE_VERIFICATION",
        false,
    );
    if require_chain_progress_signature_verification {
        anyhow::bail!(
            "EDGERUN_SCHEDULER_REQUIRE_CHAIN_PROGRESS_SIGNATURE_VERIFICATION=true is not supported yet: \
implement cryptographic verification for scheduler signed chain progress events before enabling"
        );
    }
    let policy_signing_key = load_policy_signing_key()?;
    let chain_progress_sink = init_chain_progress_sink(
        &data_dir,
        hex::encode(policy_signing_key.verifying_key().to_bytes()),
    );
    let chain_event_bus_sink =
        init_chain_event_bus_sink(&data_dir, persisted.latest_chain_progress_bus_nonce)?;

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
        snapshot_flush_state: Arc::new(Mutex::new(SnapshotFlushState::default())),
        state_snapshot_min_interval_secs: read_env_u64(
            "EDGERUN_SCHEDULER_STATE_SNAPSHOT_MIN_INTERVAL_SECS",
            2,
        )
        .max(1),
        worker_registry: Arc::new(Mutex::new(persisted.worker_registry)),
        job_quorum: Arc::new(Mutex::new(persisted.job_quorum)),
        policy_signing_key,
        policy_key_id: std::env::var("EDGERUN_SCHEDULER_POLICY_KEY_ID")
            .unwrap_or_else(|_| default_policy_key_id()),
        policy_version: read_env_u32("EDGERUN_SCHEDULER_POLICY_VERSION", default_policy_version()),
        policy_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_POLICY_TTL_SECS", 300),
        pricing_lamports_per_billion_instructions: read_env_u64(
            "EDGERUN_SCHEDULER_LAMPORTS_PER_BILLION_INSTRUCTIONS",
            10_000_000,
        ),
        pricing_flat_lamports: read_env_u64_allow_zero("EDGERUN_SCHEDULER_FLAT_FEE_LAMPORTS", 0),
        committee_size: 3,
        quorum: 2,
        heartbeat_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_HEARTBEAT_TTL_SECS", 15),
        assignments_signature_max_age_secs: read_env_u64(
            "EDGERUN_SCHEDULER_ASSIGNMENTS_SIGNATURE_MAX_AGE_SECS",
            60,
        ),
        require_assignments_signatures: read_env_bool(
            "EDGERUN_SCHEDULER_REQUIRE_ASSIGNMENTS_SIGNATURES",
            true,
        ),
        max_assignments_per_worker: read_env_usize(
            "EDGERUN_SCHEDULER_MAX_ASSIGNMENTS_PER_WORKER",
            256,
        ),
        max_assignments_total: read_env_usize("EDGERUN_SCHEDULER_MAX_ASSIGNMENTS_TOTAL", 20000),
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
        policy_audit_path: data_dir.join("policy-audit.jsonl"),
        trust_policy: Arc::new(Mutex::new(trust_policy)),
        attestation_policy: Arc::new(Mutex::new(attestation_policy)),
        route_shared_state_path,
        route_flush_state: Arc::new(Mutex::new(SnapshotFlushState::default())),
        route_sync_min_interval_secs: read_env_u64(
            "EDGERUN_SCHEDULER_ROUTE_SYNC_MIN_INTERVAL_SECS",
            2,
        )
        .max(1),
        housekeeping_interval_secs: read_env_u64("EDGERUN_SCHEDULER_HOUSEKEEPING_INTERVAL_SECS", 5)
            .max(1),
        route_signature_max_age_secs: read_env_u64(
            "EDGERUN_SCHEDULER_ROUTE_SIGNATURE_MAX_AGE_SECS",
            300,
        ),
        route_challenge_ttl_secs: read_env_u64("EDGERUN_SCHEDULER_ROUTE_CHALLENGE_TTL_SECS", 120)
            .max(30),
        device_routes: Arc::new(Mutex::new(loaded_route_state.device_routes)),
        route_challenges: Arc::new(Mutex::new(loaded_route_state.route_challenges)),
        route_heartbeat_tokens: Arc::new(Mutex::new(loaded_route_state.route_heartbeat_tokens)),
        signal_peers: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        chain_progress_sink: Arc::new(Mutex::new(chain_progress_sink)),
        chain_event_bus_sink: Arc::new(Mutex::new(chain_event_bus_sink)),
        latest_chain_progress_event_id: Arc::new(Mutex::new(
            persisted.latest_chain_progress_event_id.clone(),
        )),
        require_chain_progress_signature_verification,
        enforce_event_ingestion: read_env_bool("EDGERUN_SCHEDULER_ENFORCE_EVENT_INGESTION", true),
        chain,
    };
    if state.enforce_event_ingestion
        && state
            .chain_event_bus_sink
            .lock()
            .expect("lock poisoned")
            .is_none()
    {
        anyhow::bail!(
            "EDGERUN_SCHEDULER_ENFORCE_EVENT_INGESTION=true but scheduler event bus sink is unavailable"
        );
    }
    enforce_history_retention(&state);

    let housekeeping_state = state.clone();
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("https://www.edgerun.tech"),
            HeaderValue::from_static("https://edgerun.tech"),
        ])
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers(Any)
        .expose_headers([header::CONTENT_TYPE]);
    let app = Router::new()
        .route("/v1/webrtc/ws", get(webrtc_signal_ws))
        .route("/v1/control/ws", get(control_ws))
        .layer(cors)
        .with_state(state);
    tokio::spawn(async move {
        housekeeping_loop(housekeeping_state).await;
    });
    tracing::info!(%addr, "scheduler listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn with_route_maps_mut<R>(
    state: &AppState,
    f: impl FnOnce(
        &mut HashMap<String, DeviceRouteEntry>,
        &mut HashMap<String, u64>,
        &mut HashMap<String, RouteHeartbeatToken>,
    ) -> Result<R, (StatusCode, String)>,
) -> Result<R, (StatusCode, String)> {
    let out = {
        let mut routes = state.device_routes.lock().expect("lock poisoned");
        let mut challenges = state.route_challenges.lock().expect("lock poisoned");
        let mut tokens = state.route_heartbeat_tokens.lock().expect("lock poisoned");
        f(&mut routes, &mut challenges, &mut tokens)?
    };
    schedule_route_state_flush(state).map_err(internal_err)?;
    Ok(out)
}

fn resolve_route_for_device(state: &AppState, device_id: &str) -> RouteResolveResponse {
    let now = now_unix_seconds();
    let route = with_route_maps_mut(state, |routes, _, _| {
        prune_expired_routes(routes, now);
        Ok(routes.get(device_id.trim()).cloned())
    })
    .ok()
    .flatten();
    RouteResolveResponse {
        ok: true,
        found: route.is_some(),
        route,
    }
}

fn resolve_owner_routes_for_owner(state: &AppState, owner_pubkey: &str) -> OwnerRoutesResponse {
    let owner_pubkey = owner_pubkey.trim().to_string();
    let now = now_unix_seconds();
    let devices = with_route_maps_mut(state, |routes, _, _| {
        prune_expired_routes(routes, now);
        Ok(routes
            .values()
            .filter(|route| route.owner_pubkey == owner_pubkey)
            .cloned()
            .collect::<Vec<_>>())
    })
    .unwrap_or_default();
    OwnerRoutesResponse {
        ok: true,
        owner_pubkey,
        devices,
    }
}

async fn webrtc_signal_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WebRtcSignalConnectQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| webrtc_signal_socket(state, query.device_id, socket))
}

async fn control_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<ControlWsConnectQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| control_ws_socket(state, query.client_id, socket))
}

async fn control_ws_socket(state: AppState, client_id: String, mut socket: WebSocket) {
    let client_id = client_id.trim().to_string();
    if client_id.is_empty() {
        let response = ControlWsServerMessage {
            request_id: String::new(),
            ok: false,
            data: None,
            error: Some("client_id is required".to_string()),
            status: Some(400),
        };
        let _ = socket
            .send(Message::Binary(encode_control_message(&response).into()))
            .await;
        return;
    }

    loop {
        match socket.recv().await {
            Some(Ok(Message::Binary(bytes))) => {
                let response = handle_control_client_binary_message(&state, &bytes).await;
                if socket
                    .send(Message::Binary(encode_control_message(&response).into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Some(Ok(Message::Text(text))) => {
                let response = handle_control_client_text_message(&state, &text).await;
                let payload = serde_json::to_string(&response).unwrap_or_else(|_| {
                    "{\"request_id\":\"\",\"ok\":false,\"error\":\"encode error\",\"status\":500}"
                        .to_string()
                });
                if socket.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
            Some(Ok(Message::Close(_))) | None => break,
            Some(Ok(_)) => {}
            Some(Err(_)) => break,
        }
    }
}

fn encode_control_message(msg: &ControlWsServerMessage) -> Vec<u8> {
    bincode::serialize(msg).unwrap_or_default()
}

fn control_error_response(
    request_id: String,
    status: StatusCode,
    error: String,
) -> ControlWsServerMessage {
    ControlWsServerMessage {
        request_id,
        ok: false,
        data: None,
        error: Some(error),
        status: Some(status.as_u16()),
    }
}

fn control_ok_response(
    request_id: String,
    data: ControlWsResponsePayload,
) -> ControlWsServerMessage {
    ControlWsServerMessage {
        request_id,
        ok: true,
        data: Some(data),
        error: None,
        status: None,
    }
}

async fn handle_control_client_binary_message(
    state: &AppState,
    bytes: &[u8],
) -> ControlWsServerMessage {
    let parsed = bincode::deserialize::<ControlWsClientMessage>(bytes);
    let msg = match parsed {
        Ok(message) if !message.request_id.trim().is_empty() => message,
        _ => {
            return control_error_response(
                String::new(),
                StatusCode::BAD_REQUEST,
                "invalid control message".to_string(),
            );
        }
    };

    handle_control_request_payload(state, msg).await
}

async fn handle_control_client_text_message(
    state: &AppState,
    text: &str,
) -> JsonControlWsServerMessage {
    let msg = match decode_json_control_request(text) {
        Ok(msg) => msg,
        Err((status, error)) => {
            return JsonControlWsServerMessage {
                request_id: String::new(),
                ok: false,
                data: None,
                error: Some(error),
                status: Some(status.as_u16()),
            }
        }
    };
    let response = handle_control_request_payload(state, msg).await;
    control_response_to_json(response)
}

fn decode_json_control_request(text: &str) -> Result<ControlWsClientMessage, (StatusCode, String)> {
    let parsed = serde_json::from_str::<JsonControlWsClientMessage>(text).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid json control message".to_string(),
        )
    })?;
    let request_id = parsed.request_id.trim().to_string();
    if request_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "request_id is required".to_string(),
        ));
    }
    let op = parsed.op.trim().to_ascii_lowercase();
    let payload = parsed.payload;
    let mapped = match op.as_str() {
        "job.create" => {
            ControlWsRequestPayload::JobCreate(serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for job.create".to_string(),
                )
            })?)
        }
        "job.status" => {
            ControlWsRequestPayload::JobStatus(serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for job.status".to_string(),
                )
            })?)
        }
        "route.challenge" => ControlWsRequestPayload::RouteChallenge(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for route.challenge".to_string(),
                )
            })?,
        ),
        "route.register" => ControlWsRequestPayload::RouteRegister(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for route.register".to_string(),
                )
            })?,
        ),
        "route.heartbeat" => ControlWsRequestPayload::RouteHeartbeat(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for route.heartbeat".to_string(),
                )
            })?,
        ),
        "route.resolve" => ControlWsRequestPayload::RouteResolve(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for route.resolve".to_string(),
                )
            })?,
        ),
        "route.owner" => {
            ControlWsRequestPayload::RouteOwner(serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for route.owner".to_string(),
                )
            })?)
        }
        "worker.heartbeat" => ControlWsRequestPayload::WorkerHeartbeat(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.heartbeat".to_string(),
                )
            })?,
        ),
        "worker.assignments" => ControlWsRequestPayload::WorkerAssignments(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.assignments".to_string(),
                )
            })?,
        ),
        "worker.result" => ControlWsRequestPayload::WorkerResult(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.result".to_string(),
                )
            })?,
        ),
        "worker.failure" => ControlWsRequestPayload::WorkerFailure(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.failure".to_string(),
                )
            })?,
        ),
        "worker.replay" => ControlWsRequestPayload::WorkerReplay(
            serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.replay".to_string(),
                )
            })?,
        ),
        "bundle.get" => {
            ControlWsRequestPayload::BundleGet(serde_json::from_value(payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for bundle.get".to_string(),
                )
            })?)
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unsupported op '{}'", parsed.op.trim()),
            ))
        }
    };
    Ok(ControlWsClientMessage {
        request_id,
        payload: mapped,
    })
}

fn control_response_to_json(response: ControlWsServerMessage) -> JsonControlWsServerMessage {
    let data = response
        .data
        .and_then(|payload| serde_json::to_value(payload).ok());
    JsonControlWsServerMessage {
        request_id: response.request_id,
        ok: response.ok,
        data,
        error: response.error,
        status: response.status,
    }
}

async fn handle_control_request_payload(
    state: &AppState,
    msg: ControlWsClientMessage,
) -> ControlWsServerMessage {
    let request_id = msg.request_id.trim().to_string();
    match msg.payload {
        ControlWsRequestPayload::JobCreate(payload) => match job_create_inner(state, payload) {
            Ok(ok) => control_ok_response(request_id, ControlWsResponsePayload::JobCreate(ok)),
            Err((status, err)) => control_error_response(request_id, status, err),
        },
        ControlWsRequestPayload::JobStatus(JobStatusRequest { job_id }) => {
            let job_id = job_id.trim().to_string();
            if job_id.is_empty() {
                control_error_response(
                    request_id,
                    StatusCode::BAD_REQUEST,
                    "job_id is required".to_string(),
                )
            } else {
                let status = get_job_status_inner(state, job_id);
                control_ok_response(
                    request_id,
                    ControlWsResponsePayload::JobStatus(Box::new(status)),
                )
            }
        }
        ControlWsRequestPayload::RouteChallenge(payload) => {
            match route_challenge_inner(state, payload) {
                Ok(ok) => {
                    control_ok_response(request_id, ControlWsResponsePayload::RouteChallenge(ok))
                }
                Err((status, err)) => control_error_response(request_id, status, err),
            }
        }
        ControlWsRequestPayload::RouteRegister(payload) => {
            match route_register_inner(state, payload) {
                Ok(ok) => {
                    control_ok_response(request_id, ControlWsResponsePayload::RouteRegister(ok))
                }
                Err((status, err)) => control_error_response(request_id, status, err),
            }
        }
        ControlWsRequestPayload::RouteHeartbeat(payload) => {
            match route_heartbeat_inner(state, payload) {
                Ok(ok) => {
                    control_ok_response(request_id, ControlWsResponsePayload::RouteHeartbeat(ok))
                }
                Err((status, err)) => control_error_response(request_id, status, err),
            }
        }
        ControlWsRequestPayload::RouteResolve(RouteResolveRequest { device_id }) => {
            let device_id = device_id.trim().to_string();
            if device_id.is_empty() {
                control_error_response(
                    request_id,
                    StatusCode::BAD_REQUEST,
                    "device_id is required".to_string(),
                )
            } else {
                let resolved = resolve_route_for_device(state, &device_id);
                let mapped = CpRouteResolveResponse {
                    ok: resolved.ok,
                    found: resolved.found,
                    route: resolved.route.map(|entry| CpRouteEntry {
                        device_id: entry.device_id,
                        owner_pubkey: entry.owner_pubkey,
                        candidates: entry.candidates,
                        reachable_urls: entry.reachable_urls,
                        capabilities: entry.capabilities,
                        relay_session_id: entry.relay_session_id,
                        online: entry.online,
                        last_seen_unix_s: entry.last_seen_unix_s,
                        expires_at_unix_s: entry.expires_at_unix_s,
                        updated_at_unix_s: entry.updated_at_unix_s,
                    }),
                };
                control_ok_response(request_id, ControlWsResponsePayload::RouteResolve(mapped))
            }
        }
        ControlWsRequestPayload::RouteOwner(RouteOwnerRequest { owner_pubkey }) => {
            let owner_pubkey = owner_pubkey.trim().to_string();
            if owner_pubkey.is_empty() {
                control_error_response(
                    request_id,
                    StatusCode::BAD_REQUEST,
                    "owner_pubkey is required".to_string(),
                )
            } else {
                let resolved = resolve_owner_routes_for_owner(state, &owner_pubkey);
                let mapped = edgerun_types::control_plane::OwnerRoutesResponse {
                    ok: resolved.ok,
                    owner_pubkey: resolved.owner_pubkey,
                    devices: resolved
                        .devices
                        .into_iter()
                        .map(|entry| CpRouteEntry {
                            device_id: entry.device_id,
                            owner_pubkey: entry.owner_pubkey,
                            candidates: entry.candidates,
                            reachable_urls: entry.reachable_urls,
                            capabilities: entry.capabilities,
                            relay_session_id: entry.relay_session_id,
                            online: entry.online,
                            last_seen_unix_s: entry.last_seen_unix_s,
                            expires_at_unix_s: entry.expires_at_unix_s,
                            updated_at_unix_s: entry.updated_at_unix_s,
                        })
                        .collect(),
                };
                control_ok_response(
                    request_id,
                    ControlWsResponsePayload::RouteOwner(Box::new(mapped)),
                )
            }
        }
        ControlWsRequestPayload::WorkerHeartbeat(payload) => {
            match worker_heartbeat_inner(state, payload) {
                Ok(ok) => {
                    control_ok_response(request_id, ControlWsResponsePayload::WorkerHeartbeat(ok))
                }
                Err((status, err)) => control_error_response(request_id, status, err),
            }
        }
        ControlWsRequestPayload::WorkerAssignments(payload) => {
            let worker_pubkey = payload.worker_pubkey.trim().to_string();
            if worker_pubkey.is_empty() {
                control_error_response(
                    request_id,
                    StatusCode::BAD_REQUEST,
                    "worker_pubkey is required".to_string(),
                )
            } else {
                match worker_assignments_inner(state, payload) {
                    Ok(ok) => control_ok_response(
                        request_id,
                        ControlWsResponsePayload::WorkerAssignments(Box::new(ok)),
                    ),
                    Err((status, err)) => control_error_response(request_id, status, err),
                }
            }
        }
        ControlWsRequestPayload::WorkerResult(payload) => match worker_result_inner(state, payload)
        {
            Ok(ok) => control_ok_response(request_id, ControlWsResponsePayload::WorkerResult(ok)),
            Err((status, err)) => control_error_response(request_id, status, err),
        },
        ControlWsRequestPayload::WorkerFailure(payload) => {
            match worker_failure_inner(state, payload) {
                Ok(ok) => {
                    control_ok_response(request_id, ControlWsResponsePayload::WorkerFailure(ok))
                }
                Err((status, err)) => control_error_response(request_id, status, err),
            }
        }
        ControlWsRequestPayload::WorkerReplay(payload) => {
            match worker_replay_artifact_inner(state, payload) {
                Ok(ok) => {
                    control_ok_response(request_id, ControlWsResponsePayload::WorkerReplay(ok))
                }
                Err((status, err)) => control_error_response(request_id, status, err),
            }
        }
        ControlWsRequestPayload::BundleGet(edgerun_types::control_plane::BundleGetRequest {
            bundle_hash,
        }) => {
            let bundle_hash = bundle_hash.trim().to_string();
            if bundle_hash.is_empty() {
                control_error_response(
                    request_id,
                    StatusCode::BAD_REQUEST,
                    "bundle_hash is required".to_string(),
                )
            } else {
                let path = bundle_path(state, &bundle_hash);
                match std::fs::read(path) {
                    Ok(bytes) => control_ok_response(
                        request_id,
                        ControlWsResponsePayload::BundleGet(BundleGetResponse {
                            ok: true,
                            bundle_hash,
                            payload_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
                        }),
                    ),
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                        control_error_response(
                            request_id,
                            StatusCode::NOT_FOUND,
                            "bundle not found".to_string(),
                        )
                    }
                    Err(err) => control_error_response(
                        request_id,
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("bundle read failed: {err}"),
                    ),
                }
            }
        }
    }
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
                        if let Err(error) = handle_signal_client_message(&state, &device_id, &text).await {
                            let encoded = encode_signal_error(&device_id, &error);
                            let _ = socket.send(Message::Text(encoded.into())).await;
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
) -> Result<(), String> {
    let msg = decode_signal_client_message(text)?;
    let to_device_id = msg.to_device_id.trim().to_string();
    let to_owner_pubkey = msg.to_owner_pubkey.trim().to_string();
    if to_device_id.is_empty() && to_owner_pubkey.is_empty() {
        return Err("missing signal target: set to_device_id or to_owner_pubkey".to_string());
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
    let encoded = encode_signal_server_message(&outbound);

    if !to_device_id.is_empty() {
        let sender = {
            let peers = state.signal_peers.lock().await;
            peers.get(to_device_id.as_str()).cloned()
        };
        let Some(sender) = sender else {
            return Err(format!(
                "target device not connected to signaling ws: {}",
                to_device_id
            ));
        };
        sender.send(encoded).map_err(|_| {
            format!(
                "failed to deliver signaling message to target device: {}",
                to_device_id
            )
        })?;
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
        Err(_) => return Err("failed to resolve route targets for owner".to_string()),
    };
    if target_device_ids.is_empty() {
        return Err(format!(
            "no routed devices found for owner: {}",
            to_owner_pubkey
        ));
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
        return Err(format!(
            "no online signaling peers available for owner: {}",
            to_owner_pubkey
        ));
    }
    Ok(())
}

fn b64_encode_text(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(value.as_bytes())
}

fn b64_decode_text(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .map_err(|_| "invalid signaling payload encoding".to_string())?;
    String::from_utf8(bytes).map_err(|_| "invalid signaling utf8 payload".to_string())
}

fn encode_optional_text(value: Option<&str>) -> String {
    value.map(b64_encode_text).unwrap_or_default()
}

fn encode_signal_server_message(message: &WebRtcSignalServerMessage) -> String {
    let sdp_mline_index = message
        .sdp_mline_index
        .map(|v| v.to_string())
        .unwrap_or_default();
    [
        b64_encode_text(&message.from_device_id),
        b64_encode_text(&message.kind),
        encode_optional_text(message.sdp.as_deref()),
        encode_optional_text(message.candidate.as_deref()),
        encode_optional_text(message.sdp_mid.as_deref()),
        b64_encode_text(&sdp_mline_index),
        encode_optional_text(message.metadata.as_deref()),
        String::new(),
    ]
    .join("|")
}

fn encode_signal_error(from_device_id: &str, error: &str) -> String {
    [
        b64_encode_text(from_device_id),
        b64_encode_text("error"),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        b64_encode_text(error),
    ]
    .join("|")
}

fn decode_signal_client_message(text: &str) -> Result<WebRtcSignalClientMessage, String> {
    let mut parts = text.split('|');
    let fields: [Cow<'_, str>; 8] =
        std::array::from_fn(|_| Cow::Borrowed(parts.next().unwrap_or("")));
    if parts.next().is_some() {
        return Err("invalid signaling frame format".to_string());
    }

    let to_device_id = b64_decode_text(&fields[0])?;
    let to_owner_pubkey = b64_decode_text(&fields[1])?;
    let kind = b64_decode_text(&fields[2])?;
    let sdp = b64_decode_text(&fields[3])?;
    let candidate = b64_decode_text(&fields[4])?;
    let sdp_mid = b64_decode_text(&fields[5])?;
    let sdp_mline_index = b64_decode_text(&fields[6])?;
    let metadata = b64_decode_text(&fields[7])?;

    let parsed_mline = if sdp_mline_index.trim().is_empty() {
        None
    } else {
        Some(
            sdp_mline_index
                .trim()
                .parse::<u16>()
                .map_err(|_| "invalid sdp_mline_index".to_string())?,
        )
    };

    Ok(WebRtcSignalClientMessage {
        to_device_id,
        to_owner_pubkey,
        kind,
        sdp: if sdp.is_empty() { None } else { Some(sdp) },
        candidate: if candidate.is_empty() {
            None
        } else {
            Some(candidate)
        },
        sdp_mid: if sdp_mid.is_empty() {
            None
        } else {
            Some(sdp_mid)
        },
        sdp_mline_index: parsed_mline,
        metadata: if metadata.is_empty() {
            None
        } else {
            Some(metadata)
        },
    })
}

fn prune_expired_routes(routes: &mut HashMap<String, DeviceRouteEntry>, now: u64) {
    routes.retain(|_, route| route.expires_at_unix_s > now);
}

const INGEST_SCHEMA_VERSION: u32 = 1;
const EVENT_PUBLISHER_SCHEDULER: &str = "scheduler";
const EVENT_SIGNATURE_SCHEDULER_INGEST: &str = "scheduler-control-plane";
const EVENT_POLICY_ID_SCHEDULER_CONTROL_PLANE: &str = "scheduler-control-plane-v1";
const EVENT_TYPE_WORKER_HEARTBEAT: &str = "worker_heartbeat";
const EVENT_TYPE_WORKER_ASSIGNMENTS_POLL: &str = "worker_assignments_poll";
const EVENT_TYPE_WORKER_RESULT: &str = "worker_result";
const EVENT_TYPE_WORKER_FAILURE: &str = "worker_failure";
const EVENT_TYPE_WORKER_REPLAY: &str = "worker_replay";
const EVENT_TYPE_ROUTE_CHALLENGE: &str = "route_challenge";
const EVENT_TYPE_ROUTE_REGISTER: &str = "route_register";
const EVENT_TYPE_ROUTE_HEARTBEAT: &str = "route_heartbeat";

fn ingest_scheduler_event<T: Serialize>(
    state: &AppState,
    payload_type: &str,
    payload: &T,
) -> Result<(), (StatusCode, String)> {
    let encoded = bincode::serialize(payload).map_err(internal_err)?;
    let mut guard = state.chain_event_bus_sink.lock().expect("lock poisoned");
    let Some(bus_sink) = guard.as_mut() else {
        if state.enforce_event_ingestion {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "scheduler event ingestion sink is unavailable".to_string(),
            ));
        }
        tracing::warn!(
            payload_type,
            "skipping control-plane event ingestion because event bus sink is unavailable"
        );
        return Ok(());
    };
    let envelope = StorageBackedEventBus::build_envelope(
        bus_sink.next_nonce,
        EVENT_PUBLISHER_SCHEDULER.to_string(),
        EVENT_SIGNATURE_SCHEDULER_INGEST.to_string(),
        EVENT_POLICY_ID_SCHEDULER_CONTROL_PLANE.to_string(),
        vec!["*".to_string()],
        payload_type.to_string(),
        encoded,
    );
    match bus_sink.bus.publish(&envelope) {
        Ok(_) => {
            bus_sink.next_nonce = bus_sink.next_nonce.saturating_add(1);
            Ok(())
        }
        Err(err) => {
            if state.enforce_event_ingestion {
                Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("failed to ingest event '{payload_type}': {err}"),
                ))
            } else {
                tracing::warn!(
                    payload_type,
                    error = %err,
                    "control-plane event ingestion failed (continuing because enforcement is disabled)"
                );
                Ok(())
            }
        }
    }
}

fn parse_route_owner_verifying_key(
    owner_pubkey: &str,
) -> Result<VerifyingKey, (StatusCode, String)> {
    let trimmed = owner_pubkey.trim();
    if trimmed.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "owner_pubkey is required".to_string(),
        ));
    }
    if let Ok(pubkey) = trimmed.parse::<Pubkey>() {
        return VerifyingKey::from_bytes(&pubkey.to_bytes()).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "invalid owner_pubkey bytes".to_string(),
            )
        });
    }
    let bytes = URL_SAFE_NO_PAD.decode(trimmed.as_bytes()).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "owner_pubkey must be base58 or base64url".to_string(),
        )
    })?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "owner_pubkey must decode to 32 bytes".to_string(),
        )
    })?;
    VerifyingKey::from_bytes(&arr).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid owner_pubkey bytes".to_string(),
        )
    })
}

fn parse_signature_b64url(signature: &str) -> Result<Signature, (StatusCode, String)> {
    let bytes = URL_SAFE_NO_PAD
        .decode(signature.trim().as_bytes())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "invalid signature encoding".to_string(),
            )
        })?;
    let arr: [u8; 64] = bytes.try_into().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "signature must decode to 64 bytes".to_string(),
        )
    })?;
    Ok(Signature::from_bytes(&arr))
}

fn route_challenge_inner(
    state: &AppState,
    payload: RouteChallengeRequest,
) -> Result<RouteChallengeResponse, (StatusCode, String)> {
    let device_id = payload.device_id.trim().to_string();
    if device_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "device_id is required".to_string()));
    }
    let now = now_unix_seconds();
    let nonce = random_token_b64url(24);
    let expires_at_unix_s = now.saturating_add(state.route_challenge_ttl_secs);
    let ingest_event = RouteChallengeIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        device_id,
        nonce: nonce.clone(),
        expires_at_unix_s,
    };
    ingest_scheduler_event(state, EVENT_TYPE_ROUTE_CHALLENGE, &ingest_event)?;
    with_route_maps_mut(state, |_, challenges, tokens| {
        challenges.retain(|_, exp| *exp > now);
        tokens.retain(|_, token| token.expires_at_unix_s > now);
        challenges.insert(nonce.clone(), expires_at_unix_s);
        Ok(())
    })?;
    Ok(RouteChallengeResponse {
        nonce,
        expires_at_unix_s,
    })
}

fn route_register_inner(
    state: &AppState,
    payload: RouteRegisterRequest,
) -> Result<RouteRegisterResponse, (StatusCode, String)> {
    let device_id = payload.device_id.trim().to_string();
    if device_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "device_id is required".to_string()));
    }
    let owner_pubkey = payload.owner_pubkey.trim().to_string();
    if owner_pubkey.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "owner_pubkey is required".to_string(),
        ));
    }
    let challenge_nonce = payload.challenge_nonce.trim().to_string();
    if challenge_nonce.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "challenge_nonce is required".to_string(),
        ));
    }
    let signature = payload.signature.trim().to_string();
    if signature.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "signature is required".to_string()));
    }
    let (candidates, reachable_urls) =
        normalize_route_candidates(&payload.candidates, &payload.reachable_urls);
    if candidates.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "at least one valid transport candidate is required".to_string(),
        ));
    }
    let capabilities = payload
        .capabilities
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let relay_session_id = payload
        .relay_session_id
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let now = now_unix_seconds();
    if payload.signed_at_unix_s == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "signed_at_unix_s is required".to_string(),
        ));
    }
    if payload.signed_at_unix_s > now.saturating_add(state.route_signature_max_age_secs) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "route signature timestamp is too far in the future".to_string(),
        ));
    }
    let age = now.saturating_sub(payload.signed_at_unix_s);
    if age > state.route_signature_max_age_secs {
        return Err((
            StatusCode::UNAUTHORIZED,
            "route signature is too old".to_string(),
        ));
    }

    let challenge_valid = {
        let mut challenges = state.route_challenges.lock().expect("lock poisoned");
        challenges.retain(|_, exp| *exp > now);
        challenges
            .get(&challenge_nonce)
            .copied()
            .map(|exp| exp > now)
            .unwrap_or(false)
    };
    if !challenge_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            "unknown or expired challenge_nonce".to_string(),
        ));
    }

    let verifier = parse_route_owner_verifying_key(&owner_pubkey)?;
    let parsed_signature = parse_signature_b64url(&signature)?;
    let signing_message = route_register_signing_message(
        &owner_pubkey,
        &device_id,
        &reachable_urls,
        &challenge_nonce,
        payload.signed_at_unix_s,
    );
    let signing_digest = hash(signing_message.as_bytes());
    if !edgerun_crypto::verify(&verifier, signing_message.as_bytes(), &parsed_signature) {
        tracing::warn!(
            owner_pubkey = %owner_pubkey,
            device_id = %device_id,
            signed_at_unix_s = payload.signed_at_unix_s,
            signing_digest = %hex::encode(signing_digest.to_bytes()),
            reachable_urls = ?reachable_urls,
            "route register signature verification failed"
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid route register signature".to_string(),
        ));
    }

    let ttl_secs = payload.ttl_secs.clamp(30, 300);
    let mut normalized_payload = payload.clone();
    normalized_payload.device_id = device_id.clone();
    normalized_payload.owner_pubkey = owner_pubkey.clone();
    normalized_payload.challenge_nonce = challenge_nonce.clone();
    normalized_payload.candidates = candidates.clone();
    normalized_payload.reachable_urls = reachable_urls.clone();
    normalized_payload.capabilities = capabilities.clone();
    normalized_payload.relay_session_id = relay_session_id.clone();
    normalized_payload.ttl_secs = ttl_secs;
    normalized_payload.signature = signature.clone();
    let ingest_event = RouteRegisterIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: normalized_payload,
    };
    ingest_scheduler_event(state, EVENT_TYPE_ROUTE_REGISTER, &ingest_event)?;

    let heartbeat_token = random_token_b64url(32);
    with_route_maps_mut(state, |routes, challenges, tokens| {
        challenges.retain(|_, exp| *exp > now);
        let Some(expiry) = challenges.remove(&challenge_nonce) else {
            return Err((
                StatusCode::BAD_REQUEST,
                "unknown or expired challenge_nonce".to_string(),
            ));
        };
        if expiry <= now {
            return Err((
                StatusCode::BAD_REQUEST,
                "unknown or expired challenge_nonce".to_string(),
            ));
        }
        let expires_at = now.saturating_add(ttl_secs);
        routes.insert(
            device_id.clone(),
            DeviceRouteEntry {
                device_id: device_id.clone(),
                owner_pubkey,
                candidates,
                reachable_urls,
                capabilities,
                relay_session_id,
                online: true,
                last_seen_unix_s: now,
                expires_at_unix_s: expires_at,
                updated_at_unix_s: now,
            },
        );
        tokens.insert(
            heartbeat_token.clone(),
            RouteHeartbeatToken {
                device_id,
                expires_at_unix_s: expires_at,
            },
        );
        Ok(())
    })?;
    Ok(RouteRegisterResponse {
        ok: true,
        heartbeat_token,
    })
}

fn route_heartbeat_inner(
    state: &AppState,
    payload: RouteHeartbeatRequest,
) -> Result<RouteHeartbeatResponse, (StatusCode, String)> {
    let device_id = payload.device_id.trim().to_string();
    if device_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "device_id is required".to_string()));
    }
    let token = payload.token.trim().to_string();
    if token.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "token is required".to_string()));
    }
    let ttl_secs = payload.ttl_secs.clamp(30, 300);
    let now = now_unix_seconds();
    {
        let mut tokens = state.route_heartbeat_tokens.lock().expect("lock poisoned");
        tokens.retain(|_, item| item.expires_at_unix_s > now);
        let Some(stored) = tokens.get(&token) else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "unknown or expired heartbeat token".to_string(),
            ));
        };
        if stored.device_id != device_id {
            return Err((
                StatusCode::FORBIDDEN,
                "heartbeat token does not match device_id".to_string(),
            ));
        }
    }

    let ingest_event = RouteHeartbeatIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: RouteHeartbeatRequest {
            device_id: device_id.clone(),
            token: token.clone(),
            ttl_secs,
        },
    };
    ingest_scheduler_event(state, EVENT_TYPE_ROUTE_HEARTBEAT, &ingest_event)?;

    with_route_maps_mut(state, |routes, _, tokens| {
        tokens.retain(|_, item| item.expires_at_unix_s > now);
        let Some(stored) = tokens.get_mut(&token) else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "unknown or expired heartbeat token".to_string(),
            ));
        };
        if stored.device_id != device_id {
            return Err((
                StatusCode::FORBIDDEN,
                "heartbeat token does not match device_id".to_string(),
            ));
        }
        let expires_at = now.saturating_add(ttl_secs);
        stored.expires_at_unix_s = expires_at;
        let route = routes.get_mut(&device_id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "route not found for device".to_string(),
            )
        })?;
        route.online = true;
        route.last_seen_unix_s = now;
        route.expires_at_unix_s = expires_at;
        route.updated_at_unix_s = now;
        Ok(())
    })?;

    Ok(RouteHeartbeatResponse { ok: true })
}

fn normalize_route_candidates(
    candidates: &[CpRouteCandidate],
    reachable_urls: &[String],
) -> (Vec<CpRouteCandidate>, Vec<String>) {
    let mut out = Vec::<CpRouteCandidate>::new();
    let mut seen = HashSet::<String>::new();

    for candidate in candidates {
        let uri = candidate.uri.trim();
        if uri.is_empty() {
            continue;
        }
        let Some(kind) = normalize_candidate_kind(candidate.kind.as_str(), uri) else {
            continue;
        };
        if !seen.insert(uri.to_string()) {
            continue;
        }
        let metadata = candidate
            .metadata
            .iter()
            .filter_map(|(k, v)| {
                let key = k.trim();
                let value = v.trim();
                if key.is_empty() || value.is_empty() {
                    None
                } else {
                    Some((key.to_string(), value.to_string()))
                }
            })
            .collect::<BTreeMap<_, _>>();
        out.push(CpRouteCandidate {
            kind: kind.to_string(),
            uri: uri.to_string(),
            priority: candidate.priority,
            metadata,
        });
    }

    if out.is_empty() {
        for raw in reachable_urls {
            let uri = raw.trim();
            if uri.is_empty() {
                continue;
            }
            let Some(kind) = normalize_candidate_kind("", uri) else {
                continue;
            };
            if !seen.insert(uri.to_string()) {
                continue;
            }
            out.push(CpRouteCandidate {
                kind: kind.to_string(),
                uri: uri.to_string(),
                priority: 100,
                metadata: BTreeMap::new(),
            });
        }
    }

    let signable_urls = out.iter().map(|candidate| candidate.uri.clone()).collect();
    (out, signable_urls)
}

fn normalize_candidate_kind<'a>(kind: &'a str, uri: &'a str) -> Option<&'static str> {
    let normalized = kind.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "quic" if uri.starts_with("quic://") => return Some("quic"),
        "websocket" if uri.starts_with("ws://") || uri.starts_with("wss://") => {
            return Some("websocket");
        }
        "wireguard" if uri.starts_with("wg://") || uri.starts_with("wireguard://") => {
            return Some("wireguard");
        }
        _ => {}
    }
    if uri.starts_with("quic://") {
        return Some("quic");
    }
    if uri.starts_with("ws://") || uri.starts_with("wss://") {
        return Some("websocket");
    }
    if uri.starts_with("wg://") || uri.starts_with("wireguard://") {
        return Some("wireguard");
    }
    None
}

fn worker_heartbeat_inner(
    state: &AppState,
    payload: HeartbeatRequest,
) -> Result<HeartbeatResponse, (StatusCode, String)> {
    if !verify_worker_message_signature(
        state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &heartbeat_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    prune_expired_workers(state);
    tracing::info!(
        worker = %payload.worker_pubkey,
        runtime_count = payload.runtime_ids.len(),
        version = %payload.version,
        "received worker heartbeat"
    );
    let ingest_event = WorkerHeartbeatIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_TYPE_WORKER_HEARTBEAT, &ingest_event)?;
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
    if let Err(err) = schedule_state_snapshot(state) {
        tracing::warn!(error = %err, "failed to persist state after worker heartbeat");
    }
    if let Err(err) = evaluate_expired_jobs(state) {
        tracing::warn!(error = %err, "failed to evaluate expired jobs");
    }

    Ok(HeartbeatResponse {
        ok: true,
        next_poll_ms: 2000,
        server_time_unix_s: Some(now),
    })
}

fn worker_assignments_inner(
    state: &AppState,
    payload: WorkerAssignmentsRequest,
) -> Result<AssignmentsResponse, (StatusCode, String)> {
    if !verify_worker_assignments_request(state, &payload)? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker assignment signature".to_string(),
        ));
    }
    let ingest_event = WorkerAssignmentsPollIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_TYPE_WORKER_ASSIGNMENTS_POLL, &ingest_event)?;
    let worker_pubkey = payload.worker_pubkey;
    tracing::info!(worker = %worker_pubkey, "assignment poll");
    let mut assignments = state.assignments.lock().expect("lock poisoned");
    let jobs = assignments.remove(&worker_pubkey).unwrap_or_default();
    drop(assignments);

    schedule_state_snapshot(state).map_err(internal_err)?;
    Ok(AssignmentsResponse { jobs })
}

fn worker_result_inner(
    state: &AppState,
    payload: WorkerResultReport,
) -> Result<SubmissionAck, (StatusCode, String)> {
    validate_worker_result_payload(&payload)?;
    if !verify_worker_message_signature(
        state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &result_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    if !is_assigned_worker(state, &payload.job_id, &payload.worker_pubkey) {
        return Err((
            StatusCode::FORBIDDEN,
            "worker is not assigned to this job".to_string(),
        ));
    }
    if !matches_expected_bundle_hash(state, &payload.job_id, &payload.bundle_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            "bundle_hash does not match job expectation".to_string(),
        ));
    }
    if !verify_result_attestation(state, &payload)? {
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
    let ingest_event = WorkerResultIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_TYPE_WORKER_RESULT, &ingest_event)?;

    let job_id = payload.job_id.clone();
    let mut results = state.results.lock().expect("lock poisoned");
    let entries = results.entry(payload.job_id.clone()).or_default();
    if entries.iter().any(|existing| {
        is_duplicate_idempotency(&existing.idempotency_key, &payload.idempotency_key)
    }) {
        drop(results);
        let quorum_reached = recompute_job_quorum(state, &job_id).map_err(internal_err)?;
        return Ok(SubmissionAck {
            ok: true,
            duplicate: true,
            quorum_reached: Some(quorum_reached),
        });
    }
    entries.push(payload);
    drop(results);
    let quorum_reached = recompute_job_quorum(state, &job_id).map_err(internal_err)?;
    persist_job_activity(state, &job_id).map_err(internal_err)?;
    Ok(SubmissionAck {
        ok: true,
        duplicate: false,
        quorum_reached: Some(quorum_reached),
    })
}

fn worker_failure_inner(
    state: &AppState,
    payload: WorkerFailureReport,
) -> Result<SubmissionAck, (StatusCode, String)> {
    if !verify_worker_message_signature(
        state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &failure_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    if !is_assigned_worker(state, &payload.job_id, &payload.worker_pubkey) {
        return Err((
            StatusCode::FORBIDDEN,
            "worker is not assigned to this job".to_string(),
        ));
    }
    if !matches_expected_bundle_hash(state, &payload.job_id, &payload.bundle_hash) {
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
    let ingest_event = WorkerFailureIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_TYPE_WORKER_FAILURE, &ingest_event)?;

    let job_id = payload.job_id.clone();
    let mut failures = state.failures.lock().expect("lock poisoned");
    let entries = failures.entry(payload.job_id.clone()).or_default();
    if entries.iter().any(|existing| {
        is_duplicate_idempotency(&existing.idempotency_key, &payload.idempotency_key)
    }) {
        drop(failures);
        return Ok(SubmissionAck {
            ok: true,
            duplicate: true,
            quorum_reached: None,
        });
    }
    entries.push(payload);
    drop(failures);
    persist_job_activity(state, &job_id).map_err(internal_err)?;
    Ok(SubmissionAck {
        ok: true,
        duplicate: false,
        quorum_reached: None,
    })
}

fn worker_replay_artifact_inner(
    state: &AppState,
    payload: WorkerReplayArtifactReport,
) -> Result<SubmissionAck, (StatusCode, String)> {
    if !verify_worker_message_signature(
        state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &replay_signing_message(&payload),
    )? {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid worker signature".to_string(),
        ));
    }
    if !is_assigned_worker(state, &payload.job_id, &payload.worker_pubkey) {
        return Err((
            StatusCode::FORBIDDEN,
            "worker is not assigned to this job".to_string(),
        ));
    }
    if !matches_expected_bundle_hash(state, &payload.job_id, &payload.artifact.bundle_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            "artifact.bundle_hash does not match job expectation".to_string(),
        ));
    }
    if let Some(runtime_id) = payload.artifact.runtime_id.as_deref() {
        if !matches_expected_runtime_id(state, &payload.job_id, runtime_id) {
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
    let ingest_event = WorkerReplayIngestEvent {
        schema_version: INGEST_SCHEMA_VERSION,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_TYPE_WORKER_REPLAY, &ingest_event)?;

    let job_id = payload.job_id.clone();
    let mut replay_artifacts = state.replay_artifacts.lock().expect("lock poisoned");
    let entries = replay_artifacts.entry(payload.job_id.clone()).or_default();
    if entries.iter().any(|existing| {
        is_duplicate_idempotency(&existing.idempotency_key, &payload.idempotency_key)
    }) {
        drop(replay_artifacts);
        return Ok(SubmissionAck {
            ok: true,
            duplicate: true,
            quorum_reached: None,
        });
    }
    entries.push(payload);
    drop(replay_artifacts);
    persist_job_activity(state, &job_id).map_err(internal_err)?;
    Ok(SubmissionAck {
        ok: true,
        duplicate: false,
        quorum_reached: None,
    })
}

fn job_create_inner(
    state: &AppState,
    payload: JobCreateRequest,
) -> Result<JobCreateResponse, (StatusCode, String)> {
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

    let bundle_path = bundle_path(state, &bundle_hash_hex);
    write_bundle_cas(&bundle_path, &bundle_hash_hex, &bundle_payload_bytes)
        .map_err(internal_err)?;

    prune_expired_workers(state);
    let committee_workers = if let Some(worker_pubkey) = payload.assignment_worker_pubkey.as_ref() {
        vec![worker_pubkey.clone()]
    } else {
        let selected = select_committee_workers(
            state,
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
    let redundancy_multiplier = committee_workers.len().max(1) as u64;
    let min_escrow_lamports = required_instruction_escrow_lamports(
        payload.limits.max_instructions,
        state.pricing_lamports_per_billion_instructions,
        redundancy_multiplier,
        state.pricing_flat_lamports,
    );
    if payload.escrow_lamports < min_escrow_lamports {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "escrow_lamports below deterministic minimum: required {}, got {}",
                min_escrow_lamports, payload.escrow_lamports
            ),
        ));
    }

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

    enqueue_assignments_with_limits(state, queued)?;
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
    touch_job_last_update(state, &bundle_hash_hex);
    let (assign_workers_tx, assign_workers_sig, assign_workers_submitted) =
        build_assign_workers_artifact(state, &bundle_hash_hex).map_err(internal_err)?;
    {
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        if let Some(quorum_state) = job_quorum.get_mut(&bundle_hash_hex) {
            quorum_state.assign_tx = assign_workers_tx.clone();
            quorum_state.assign_sig = assign_workers_sig.clone();
            quorum_state.assign_submitted = assign_workers_submitted;
        }
    }
    schedule_state_snapshot(state).map_err(internal_err)?;
    evaluate_expired_jobs(state).map_err(internal_err)?;
    let audit_event = edgerun_hwvault_primitives::audit::PolicyAuditEvent {
        ts: now,
        action: "job_create".to_string(),
        target: bundle_hash_hex.clone(),
        details: format!(
            "policy_key_id={} policy_version={} quorum={} committee={} escrow_lamports={} min_escrow_lamports={}",
            state.policy_key_id,
            state.policy_version,
            effective_quorum,
            state.committee_size,
            payload.escrow_lamports,
            min_escrow_lamports
        ),
    };
    if let Err(err) = edgerun_hwvault_primitives::audit::append_event_jsonl(
        &state.policy_audit_path,
        &audit_event,
    ) {
        tracing::warn!(error = %err, "failed to append policy audit event");
    }

    let chain = state.chain.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "chain context unavailable; cannot build post_job transaction".to_string(),
        )
    })?;
    let runtime_proof =
        build_runtime_allowlist_proof_for_chain(chain, runtime_id).map_err(|err| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("failed to build runtime allowlist proof: {err}"),
            )
        })?;
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
    let (post_job_tx, post_job_sig) = build_post_job_tx_base64(chain, tx_args).map_err(|err| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("failed to build chain post_job transaction: {err}"),
        )
    })?;

    Ok(JobCreateResponse {
        // Job identity is bundle-hash keyed at MVP scaffold level.
        job_id: bundle_hash_hex.clone(),
        bundle_hash: bundle_hash_hex.clone(),
        bundle_url: format!("{}/bundle/{bundle_hash_hex}", state.public_base_url),
        post_job_tx,
        post_job_sig,
        assign_workers_tx,
        assign_workers_sig,
    })
}

fn get_job_status_inner(state: &AppState, job_id: String) -> JobStatusResponse {
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
    JobStatusResponse {
        job_id,
        reports,
        failures,
        replay_artifacts,
        quorum,
    }
}

fn init_chain_context() -> Result<ChainContext> {
    let rpc_url = std::env::var("EDGERUN_CHAIN_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let program_id_str = std::env::var("EDGERUN_CHAIN_PROGRAM_ID")
        .unwrap_or_else(|_| "A2ac8yDnTXKfZCHWqcJVYFfR2jv65kezW95XTgrrdbtG".to_string());
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

fn init_chain_progress_sink(data_dir: &FsPath, signer_pubkey: String) -> Option<ChainProgressSink> {
    let storage_dir = data_dir.join("storage").join("scheduler-chain-progress");
    let stream_id = fixed_storage_stream_id("edgerun-scheduler-chain-progress-stream");
    let actor_id = fixed_storage_actor_id("edgerun-scheduler-chain-progress-actor");
    match StorageEngine::new(storage_dir)
        .and_then(|engine| engine.create_append_session("chain-progress.seg", 128 * 1024 * 1024))
    {
        Ok(session) => Some(ChainProgressSink {
            session,
            stream_id,
            actor_id,
            signer_pubkey,
            seq: 0,
        }),
        Err(err) => {
            tracing::warn!(error = %err, "chain progress storage sink unavailable");
            None
        }
    }
}

fn fixed_storage_stream_id(seed: &str) -> StorageStreamId {
    let digest = hash(seed.as_bytes()).to_bytes();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    StorageStreamId::from_bytes(out)
}

fn fixed_storage_actor_id(seed: &str) -> StorageActorId {
    let digest = hash(seed.as_bytes()).to_bytes();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[16..32]);
    StorageActorId::from_bytes(out)
}

fn query_max_nonce_for_publisher(bus: &mut StorageBackedEventBus, publisher: &str) -> Result<u64> {
    let mut cursor = 0_u64;
    let mut max_nonce = 0_u64;
    loop {
        let page = bus.query(
            1024,
            cursor,
            BusQueryFilter {
                publisher: Some(publisher.to_string()),
                payload_type: None,
            },
        )?;
        for row in page.events {
            if row.envelope.nonce > max_nonce {
                max_nonce = row.envelope.nonce;
            }
        }
        let Some(next) = page.next_cursor_offset else {
            break;
        };
        cursor = next;
    }
    Ok(max_nonce)
}

const SCHEDULER_EVENT_BUS_POLICY_VERSION: u32 = 2;

fn scheduler_event_bus_rules() -> Vec<PolicyRuleV1> {
    vec![
        PolicyRuleV1 {
            publisher: "*".to_string(),
            payload_type: "policy_update_request".to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: "chain_progress".to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_WORKER_HEARTBEAT.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_WORKER_ASSIGNMENTS_POLL.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_WORKER_RESULT.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_WORKER_FAILURE.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_WORKER_REPLAY.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_ROUTE_CHALLENGE.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_ROUTE_REGISTER.to_string(),
        },
        PolicyRuleV1 {
            publisher: EVENT_PUBLISHER_SCHEDULER.to_string(),
            payload_type: EVENT_TYPE_ROUTE_HEARTBEAT.to_string(),
        },
    ]
}

fn ensure_chain_progress_policy(bus: &mut StorageBackedEventBus, next_nonce: u64) -> Result<u64> {
    let status = bus.status()?;
    if status.phase == BusPhaseV1::Running as i32
        && status.policy_version >= SCHEDULER_EVENT_BUS_POLICY_VERSION
    {
        return Ok(next_nonce);
    }
    let policy_req = PolicyUpdateRequestV1 {
        schema_version: 1,
        policy: Some(EventBusPolicyV1 {
            version: SCHEDULER_EVENT_BUS_POLICY_VERSION,
            rules: scheduler_event_bus_rules(),
        }),
    };
    let envelope = StorageBackedEventBus::build_envelope(
        next_nonce,
        "scheduler".to_string(),
        "scheduler-internal".to_string(),
        "scheduler-chain-progress-v1".to_string(),
        vec!["*".to_string()],
        "policy_update_request".to_string(),
        ProstCodec::encode_to_vec(&policy_req),
    );
    let _ = bus.publish(&envelope)?;
    Ok(next_nonce.saturating_add(1))
}

fn init_chain_event_bus_sink(
    data_dir: &FsPath,
    persisted_nonce: Option<u64>,
) -> Result<Option<ChainEventBusSink>> {
    let bus_data_dir = data_dir.join("event-bus");
    std::fs::create_dir_all(&bus_data_dir)
        .with_context(|| format!("create event bus dir: {}", bus_data_dir.display()))?;
    let mut bus = StorageBackedEventBus::open_writer(bus_data_dir, "events.seg")
        .context("open scheduler event bus sink")?;
    let discovered_max = query_max_nonce_for_publisher(&mut bus, "scheduler")
        .context("query scheduler nonce in event bus")?;
    let mut next_nonce = discovered_max.saturating_add(1);
    if let Some(saved) = persisted_nonce {
        next_nonce = next_nonce.max(saved.saturating_add(1));
    }
    next_nonce = ensure_chain_progress_policy(&mut bus, next_nonce)
        .context("ensure chain progress policy in event bus")?;
    Ok(Some(ChainEventBusSink { bus, next_nonce }))
}

fn read_and_record_chain_progress(state: &AppState) -> Option<SchedulerSignedChainProgressEvent> {
    let chain = state.chain.as_ref()?;
    let epoch_info = match chain.rpc.get_epoch_info() {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(error = %err, "failed to read current chain epoch info");
            return None;
        }
    };
    match append_signed_chain_progress_event(state, epoch_info.absolute_slot, epoch_info.epoch) {
        Ok(ev) => Some(ev),
        Err(err) => {
            tracing::warn!(error = %err, "failed to append signed chain progress event");
            None
        }
    }
}

fn append_signed_chain_progress_event(
    state: &AppState,
    slot: u64,
    epoch: u64,
) -> Result<SchedulerSignedChainProgressEvent> {
    if state.require_chain_progress_signature_verification {
        anyhow::bail!(
            "chain progress signature verification required but not implemented; \
implement verification before enabling this mode"
        );
    }
    let mut sink_guard = state.chain_progress_sink.lock().expect("lock poisoned");
    let sink = sink_guard
        .as_mut()
        .context("chain progress sink not initialized")?;
    sink.seq = sink.seq.saturating_add(1);
    let observed_at_unix_ms = now_unix_millis();
    let signing_message = format!(
        "edgerun:chain_progress:v1:{}:{}:{}:{}:{}",
        slot, epoch, observed_at_unix_ms, sink.signer_pubkey, sink.seq
    );
    let signature = state.policy_signing_key.sign(signing_message.as_bytes());
    let signature_hex = hex::encode(signature.to_bytes());
    let progress_event_id = hex::encode(hash(signing_message.as_bytes()).to_bytes());
    let event_payload = SchedulerSignedChainProgressEvent {
        progress_event_id: progress_event_id.clone(),
        slot,
        epoch,
        observed_at_unix_ms,
        signer: sink.signer_pubkey.clone(),
        signature: signature_hex,
    };
    let payload = bincode::serialize(&event_payload).context("serialize chain progress event")?;
    let event = StorageEvent::new(
        sink.stream_id.clone(),
        sink.actor_id.clone(),
        payload.clone(),
    );
    sink.session
        .append_with_durability(&event, DurabilityLevel::AckDurable)
        .context("append chain progress event to storage")?;
    drop(sink_guard);

    let mut latest_chain_event_id = progress_event_id;
    if let Some(bus_sink) = state
        .chain_event_bus_sink
        .lock()
        .expect("lock poisoned")
        .as_mut()
    {
        let envelope = StorageBackedEventBus::build_envelope(
            bus_sink.next_nonce,
            "scheduler".to_string(),
            "scheduler-internal".to_string(),
            "scheduler-chain-progress-v1".to_string(),
            vec!["*".to_string()],
            "chain_progress".to_string(),
            payload,
        );
        match bus_sink.bus.publish(&envelope) {
            Ok(_) => {
                bus_sink.next_nonce = bus_sink.next_nonce.saturating_add(1);
                latest_chain_event_id = envelope.event_id;
            }
            Err(err) => {
                tracing::warn!(error = %err, "failed to publish chain progress into event bus");
            }
        }
    }

    *state
        .latest_chain_progress_event_id
        .lock()
        .expect("lock poisoned") = Some(latest_chain_event_id);
    Ok(event_payload)
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
    let path = data_dir.join("state.bin");
    if !path.exists() {
        return Ok(PersistedState::default());
    }
    let bytes = std::fs::read(path)?;
    let state = bincode::deserialize::<PersistedState>(&bytes)?;
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
        latest_chain_progress_event_id: state
            .latest_chain_progress_event_id
            .lock()
            .expect("lock poisoned")
            .clone(),
        latest_chain_progress_bus_nonce: state
            .chain_event_bus_sink
            .lock()
            .expect("lock poisoned")
            .as_ref()
            .map(|sink| sink.next_nonce.saturating_sub(1)),
    };
    let bytes = bincode::serialize(&snapshot)?;
    let final_path = state.data_dir.join("state.bin");
    // Snapshot writes can run concurrently from heartbeat/poll handlers.
    // Use a unique temp name per write attempt to avoid rename races.
    let tmp_seq = STATE_SNAPSHOT_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp_path = state
        .data_dir
        .join(format!("state.bin.tmp-{}-{tmp_seq}", std::process::id()));
    std::fs::write(&tmp_path, &bytes)?;
    if let Err(err) = std::fs::rename(&tmp_path, &final_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err.into());
    }
    Ok(())
}

fn schedule_state_snapshot(state: &AppState) -> Result<()> {
    let now = now_unix_seconds();
    let min_interval = state.state_snapshot_min_interval_secs;
    let should_write = {
        let mut gate = state.snapshot_flush_state.lock().expect("lock poisoned");
        gate.dirty = true;
        gate.last_write_unix_s == 0 || now.saturating_sub(gate.last_write_unix_s) >= min_interval
    };
    if !should_write {
        return Ok(());
    }
    match write_state_snapshot(state) {
        Ok(()) => {
            let mut gate = state.snapshot_flush_state.lock().expect("lock poisoned");
            gate.dirty = false;
            gate.last_write_unix_s = now;
            Ok(())
        }
        Err(err) => {
            let mut gate = state.snapshot_flush_state.lock().expect("lock poisoned");
            gate.dirty = true;
            Err(err)
        }
    }
}

fn flush_state_snapshot_if_dirty(state: &AppState) -> Result<()> {
    let should_write = {
        let gate = state.snapshot_flush_state.lock().expect("lock poisoned");
        gate.dirty
    };
    if !should_write {
        return Ok(());
    }
    let now = now_unix_seconds();
    match write_state_snapshot(state) {
        Ok(()) => {
            let mut gate = state.snapshot_flush_state.lock().expect("lock poisoned");
            gate.dirty = false;
            gate.last_write_unix_s = now;
            Ok(())
        }
        Err(err) => {
            let mut gate = state.snapshot_flush_state.lock().expect("lock poisoned");
            gate.dirty = true;
            Err(err)
        }
    }
}

fn internal_err<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
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

fn enqueue_assignments_with_limits(
    state: &AppState,
    queued: Vec<(String, QueuedAssignment)>,
) -> Result<(), (StatusCode, String)> {
    let mut per_worker_incoming: HashMap<String, usize> = HashMap::new();
    for (worker_pubkey, _) in &queued {
        *per_worker_incoming
            .entry(worker_pubkey.clone())
            .or_insert(0) += 1;
    }
    let incoming_total = queued.len();

    let mut assignments = state.assignments.lock().expect("lock poisoned");
    let mut pending_total = 0usize;
    for (worker_pubkey, jobs) in assignments.iter() {
        pending_total = pending_total.saturating_add(jobs.len());
        let incoming = per_worker_incoming.get(worker_pubkey).copied().unwrap_or(0);
        if jobs.len().saturating_add(incoming) > state.max_assignments_per_worker {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!(
                    "worker backlog limit reached for {} (limit {})",
                    worker_pubkey, state.max_assignments_per_worker
                ),
            ));
        }
    }
    for (worker_pubkey, incoming) in &per_worker_incoming {
        if !assignments.contains_key(worker_pubkey) && *incoming > state.max_assignments_per_worker
        {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!(
                    "worker backlog limit reached for {} (limit {})",
                    worker_pubkey, state.max_assignments_per_worker
                ),
            ));
        }
    }
    if pending_total.saturating_add(incoming_total) > state.max_assignments_total {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "scheduler assignment backlog limit reached (pending {}, incoming {}, limit {})",
                pending_total, incoming_total, state.max_assignments_total
            ),
        ));
    }

    for (worker_pubkey, assignment) in queued {
        assignments
            .entry(worker_pubkey)
            .or_default()
            .push(assignment);
    }
    Ok(())
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
    let workers = registry
        .values()
        .filter(|entry| {
            entry.last_heartbeat_unix_s >= cutoff
                && entry
                    .runtime_ids
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(runtime_id))
        })
        .map(|entry| {
            (
                entry.worker_pubkey.clone(),
                entry.max_concurrent.unwrap_or(1).max(1),
            )
        })
        .collect::<Vec<_>>();
    drop(registry);
    let assignments = state.assignments.lock().expect("lock poisoned");

    let mut eligible = workers
        .into_iter()
        .map(|(worker_pubkey, max_concurrent)| {
            let pending = assignments
                .get(&worker_pubkey)
                .map(|jobs| jobs.len() as u128)
                .unwrap_or(0);
            let capacity = u128::from(max_concurrent.max(1));
            let load_ppm = pending.saturating_mul(1_000_000).saturating_div(capacity);
            let hash_score = hash(format!("{seed}|{runtime_id}|{worker_pubkey}").as_bytes());
            (worker_pubkey, load_ppm, hash_score)
        })
        .collect::<Vec<_>>();
    drop(assignments);

    eligible.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.2.to_bytes().cmp(&b.2.to_bytes()))
            .then_with(|| a.0.cmp(&b.0))
    });
    eligible.truncate(committee_size.max(1));
    eligible
        .into_iter()
        .map(|(worker_pubkey, _, _)| worker_pubkey)
        .collect()
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
        match build_finalize_trigger_payload(state, job_id, &committee_workers, &winners) {
            Ok((finalize_tx, finalize_sig, finalize_submitted)) => {
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
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    job_id = %job_id,
                    "quorum reached but finalize artifact unavailable"
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
        tracing::warn!(job_id = %job_id_hex, "skipping slash artifacts: no chain context");
        return Vec::new();
    };
    let Ok(job_id) = parse_hex32(job_id_hex) else {
        tracing::warn!(job_id = %job_id_hex, "skipping slash artifacts: invalid job id");
        return Vec::new();
    };

    candidate_workers
        .into_iter()
        .filter_map(|worker_pubkey| {
            let Some(worker) = worker_pubkey.parse::<Pubkey>().ok() else {
                tracing::warn!(
                    worker_pubkey = %worker_pubkey,
                    "skipping slash artifact: invalid worker pubkey"
                );
                return None;
            };
            match build_slash_worker_tx_base64(chain, job_id, worker, state.chain_auto_submit) {
                Ok((tx, sig, submitted)) => Some(SlashWorkerArtifact {
                    worker_pubkey,
                    tx,
                    sig,
                    submitted,
                }),
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        worker_pubkey = %worker_pubkey,
                        "skipping slash artifact: tx build failed"
                    );
                    None
                }
            }
        })
        .collect()
}

fn build_finalize_trigger_payload_inner(
    state: &AppState,
    job_id_hex: &str,
    committee_workers: &[String],
    winning_workers: &[String],
) -> Result<(String, Option<String>, bool)> {
    let chain = state
        .chain
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("chain context unavailable"))?;
    let job_id = parse_hex32(job_id_hex)?;
    let (committee, winners) = parse_finalize_accounts(committee_workers, winning_workers)
        .ok_or_else(|| anyhow::anyhow!("unable to derive finalize account metas"))?;
    build_finalize_job_tx_base64(chain, job_id, committee, winners, state.chain_auto_submit)
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
        anyhow::bail!("committee workers must contain exactly 3 entries");
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
    let chain_progress = read_and_record_chain_progress(state);
    let chain_slot = chain_progress.as_ref().map(|p| p.slot);
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

    if let Some(progress) = chain_progress.as_ref() {
        tracing::debug!(
            slot = progress.slot,
            epoch = progress.epoch,
            progress_event_id = %progress.progress_event_id,
            "using signed chain progress snapshot for expiry evaluation"
        );
    }

    if candidates.is_empty() {
        return Ok(());
    }

    for job_id in candidates {
        let (cancel_tx, cancel_sig, cancel_submitted) =
            build_cancel_expired_artifact(state, &job_id)?;
        let mut job_quorum = state.job_quorum.lock().expect("lock poisoned");
        if let Some(quorum_state) = job_quorum.get_mut(&job_id) {
            quorum_state.cancel_triggered = true;
            quorum_state.cancel_tx = cancel_tx;
            quorum_state.cancel_sig = cancel_sig;
            quorum_state.cancel_submitted = cancel_submitted;
        }
    }

    schedule_state_snapshot(state)?;
    Ok(())
}

fn build_cancel_expired_artifact(
    state: &AppState,
    job_id_hex: &str,
) -> Result<(Option<String>, Option<String>, bool)> {
    let Some(chain) = state.chain.as_ref() else {
        anyhow::bail!("chain context unavailable");
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
    Ok((Some(tx), sig, submitted))
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
        schedule_state_snapshot(state)?;
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
    schedule_state_snapshot(state)?;
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
    if let Err((_, err)) = enqueue_assignments_with_limits(state, queued) {
        tracing::warn!(job_id = %job_id_hex, error = %err, "skipping discovered posted job due to assignment backlog limits");
        return Ok(false);
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
        schedule_state_snapshot(state)?;
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
    committee_workers: &[String],
    winning_workers: &[String],
) -> Result<(String, Option<String>, bool)> {
    build_finalize_trigger_payload_inner(state, job_id_hex, committee_workers, winning_workers)
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

#[allow(clippy::too_many_arguments)]
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

fn verify_worker_assignments_request(
    state: &AppState,
    payload: &WorkerAssignmentsRequest,
) -> Result<bool, (StatusCode, String)> {
    if payload.worker_pubkey.trim().is_empty() {
        return Ok(false);
    }
    let now = now_unix_seconds();
    if payload.signed_at_unix_s == 0 {
        return Ok(!state.require_assignments_signatures);
    }
    if payload.signed_at_unix_s > now.saturating_add(5) {
        return Ok(false);
    }
    let age = now.saturating_sub(payload.signed_at_unix_s);
    if age > state.assignments_signature_max_age_secs {
        return Ok(false);
    }
    let require_sig = state.require_assignments_signatures;
    if !verify_worker_message_signature(
        state,
        &payload.worker_pubkey,
        payload.signature.as_deref(),
        &assignments_signing_message(payload),
    )? {
        return Ok(false);
    }
    if require_sig && payload.signature.as_deref().unwrap_or_default().is_empty() {
        return Ok(false);
    }
    Ok(true)
}

fn verify_result_attestation(
    state: &AppState,
    payload: &WorkerResultReport,
) -> Result<bool, (StatusCode, String)> {
    if !verify_attestation_claim_policy(state, payload.attestation_claim.as_ref()) {
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

fn read_env_u64_allow_zero(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_value)
}

fn required_instruction_escrow_lamports(
    max_instructions: u64,
    lamports_per_billion_instructions: u64,
    redundancy_multiplier: u64,
    flat_fee_lamports: u64,
) -> u64 {
    let instructions = u128::from(max_instructions);
    let price_per_billion = u128::from(lamports_per_billion_instructions);
    let redundancy = u128::from(redundancy_multiplier.max(1));
    let usage = instructions
        .saturating_mul(price_per_billion)
        .saturating_mul(redundancy);
    let variable = usage
        .saturating_add(INSTRUCTION_PRICE_QUANTUM.saturating_sub(1))
        .saturating_div(INSTRUCTION_PRICE_QUANTUM);
    let total = variable.saturating_add(u128::from(flat_fee_lamports));
    total.min(u128::from(u64::MAX)) as u64
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
    let raw = std::fs::read(path)
        .with_context(|| format!("failed to read trust policy file {}", path.display()))?;
    let parsed: edgerun_types::SyncTrustPolicy =
        bincode::deserialize(&raw).context("invalid trust policy binary")?;
    Ok(parsed)
}

fn load_attestation_policy(path: &std::path::Path) -> Result<edgerun_types::AttestationPolicy> {
    if !path.exists() {
        return Ok(edgerun_types::AttestationPolicy::default());
    }
    let raw = std::fs::read(path)
        .with_context(|| format!("failed to read attestation policy file {}", path.display()))?;
    let parsed: edgerun_types::AttestationPolicy =
        bincode::deserialize(&raw).context("invalid attestation policy binary")?;
    Ok(parsed)
}

fn load_route_state(path: &std::path::Path) -> Result<PersistedRouteState> {
    if !path.exists() {
        return Ok(PersistedRouteState::default());
    }
    let raw = std::fs::read(path)
        .with_context(|| format!("failed to read route shared state file {}", path.display()))?;
    if raw.is_empty() {
        return Ok(PersistedRouteState::default());
    }
    let parsed: PersistedRouteState =
        bincode::deserialize(&raw).context("invalid route shared state binary")?;
    Ok(parsed)
}

fn save_route_state(path: &std::path::Path, state: &PersistedRouteState) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create route shared state parent directory {}",
                parent.display()
            )
        })?;
    }
    let bytes = bincode::serialize(state).context("serialize route shared state")?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).with_context(|| {
        format!(
            "failed to write temp route shared state file {}",
            tmp.display()
        )
    })?;
    std::fs::rename(&tmp, path).with_context(|| {
        format!(
            "failed to rename route shared state temp file {} to {}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn schedule_route_state_flush(state: &AppState) -> Result<()> {
    let now = now_unix_seconds();
    let min_interval = state.route_sync_min_interval_secs;
    let should_write = {
        let mut gate = state.route_flush_state.lock().expect("lock poisoned");
        gate.dirty = true;
        gate.last_write_unix_s == 0 || now.saturating_sub(gate.last_write_unix_s) >= min_interval
    };
    if !should_write {
        return Ok(());
    }
    flush_route_state_if_dirty(state)
}

fn flush_route_state_if_dirty(state: &AppState) -> Result<()> {
    let should_write = {
        let gate = state.route_flush_state.lock().expect("lock poisoned");
        gate.dirty
    };
    if !should_write {
        return Ok(());
    }
    let snapshot = PersistedRouteState {
        device_routes: state.device_routes.lock().expect("lock poisoned").clone(),
        route_challenges: state
            .route_challenges
            .lock()
            .expect("lock poisoned")
            .clone(),
        route_heartbeat_tokens: state
            .route_heartbeat_tokens
            .lock()
            .expect("lock poisoned")
            .clone(),
    };
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&state.route_shared_state_path)
        .with_context(|| {
            format!(
                "failed to open route shared lock file {}",
                state.route_shared_state_path.display()
            )
        })?;
    lock_file.lock_exclusive()?;
    let save_result = save_route_state(&state.route_shared_state_path, &snapshot);
    let _ = lock_file.unlock();
    save_result?;
    let now = now_unix_seconds();
    let mut gate = state.route_flush_state.lock().expect("lock poisoned");
    gate.dirty = false;
    gate.last_write_unix_s = now;
    Ok(())
}

fn sync_route_state_from_shared_file(state: &AppState) -> Result<()> {
    {
        let gate = state.route_flush_state.lock().expect("lock poisoned");
        if gate.dirty {
            return Ok(());
        }
    }
    let loaded = load_route_state(&state.route_shared_state_path)?;
    {
        let mut routes = state.device_routes.lock().expect("lock poisoned");
        let mut challenges = state.route_challenges.lock().expect("lock poisoned");
        let mut tokens = state.route_heartbeat_tokens.lock().expect("lock poisoned");
        *routes = loaded.device_routes;
        *challenges = loaded.route_challenges;
        *tokens = loaded.route_heartbeat_tokens;
    }
    let mut gate = state.route_flush_state.lock().expect("lock poisoned");
    gate.last_write_unix_s = now_unix_seconds();
    Ok(())
}

async fn housekeeping_loop(state: AppState) {
    let interval_secs = state.housekeeping_interval_secs;
    loop {
        if let Err(err) = sync_route_state_from_shared_file(&state) {
            tracing::warn!(error = %err, "housekeeping route-state sync failed");
        }
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
        if let Err(err) = flush_state_snapshot_if_dirty(&state) {
            tracing::warn!(error = %err, "housekeeping state snapshot flush failed");
        }
        if let Err(err) = flush_route_state_if_dirty(&state) {
            tracing::warn!(error = %err, "housekeeping route-state flush failed");
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

fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn verify_attestation_claim_policy(
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

fn touch_job_last_update(state: &AppState, job_id: &str) {
    let mut map = state.job_last_update.lock().expect("lock poisoned");
    map.insert(job_id.to_string(), now_unix_seconds());
}

fn persist_job_activity(state: &AppState, job_id: &str) -> Result<()> {
    touch_job_last_update(state, job_id);
    enforce_history_retention(state);
    evaluate_expired_jobs(state)?;
    schedule_state_snapshot(state)
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
