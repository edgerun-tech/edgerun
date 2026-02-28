// SPDX-License-Identifier: LicenseRef-Edgerun-Proprietary
#![allow(deprecated)]

mod workflow_domain;

use std::collections::{HashMap, HashSet};
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
use base64::Engine;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use edgerun_event_bus::edge_internal::{spawn_edge_internal_broker, EdgeInternalBrokerHandle};
use edgerun_event_bus::{EventTopic, OverlayNetwork, RuntimeEventBus};
use edgerun_transport_core::{
    assignments_signing_message, failure_signing_message, heartbeat_signing_message,
    replay_signing_message, result_signing_message,
};
use edgerun_types::control_plane::{
    assignment_policy_message, default_policy_key_id, default_policy_version, AssignmentsResponse,
    BundleGetResponse, ControlWsClientMessage, ControlWsRequestPayload, ControlWsResponsePayload,
    ControlWsServerMessage, HeartbeatRequest, HeartbeatResponse, JobCreateRequest,
    JobCreateResponse, JobQuorumState, JobStatusRequest, JobStatusResponse, QueuedAssignment,
    SlashWorkerArtifact, SubmissionAck, WorkerAssignmentsRequest, WorkerFailureReport,
    WorkerReplayArtifactReport, WorkerResultReport,
};
use edgerun_types::intent_pipeline::{
    WorkerAssignmentsPollIngestEvent, WorkerFailureIngestEvent, WorkerHeartbeatIngestEvent,
    WorkerReplayIngestEvent, WorkerResultIngestEvent, EVENT_PAYLOAD_TYPE_WORKER_ASSIGNMENTS_POLL,
    EVENT_PAYLOAD_TYPE_WORKER_FAILURE, EVENT_PAYLOAD_TYPE_WORKER_HEARTBEAT,
    EVENT_PAYLOAD_TYPE_WORKER_REPLAY, EVENT_PAYLOAD_TYPE_WORKER_RESULT, EVENT_SCHEMA_VERSION_V1,
    EVENT_TOPIC_SCHEDULER_WORKER_ASSIGNMENTS_POLL, EVENT_TOPIC_SCHEDULER_WORKER_FAILURE,
    EVENT_TOPIC_SCHEDULER_WORKER_HEARTBEAT, EVENT_TOPIC_SCHEDULER_WORKER_REPLAY,
    EVENT_TOPIC_SCHEDULER_WORKER_RESULT,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct HashDigest([u8; 32]);

impl HashDigest {
    fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

fn hash(bytes: &[u8]) -> HashDigest {
    HashDigest(edgerun_crypto::blake3_256(bytes))
}

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
    job_timeout_secs: u64,
    policy_audit_path: PathBuf,
    trust_policy: Arc<Mutex<edgerun_types::SyncTrustPolicy>>,
    attestation_policy: Arc<Mutex<edgerun_types::AttestationPolicy>>,
    housekeeping_interval_secs: u64,
    scheduler_event_bus: RuntimeEventBus,
    enforce_event_ingestion: bool,
}

#[derive(Debug, Clone, Copy)]
struct RetentionConfig {
    max_reports_per_job: usize,
    max_failures_per_job: usize,
    max_replays_per_job: usize,
    max_jobs_tracked: usize,
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

#[derive(Debug, Deserialize)]
struct ControlWsConnectQuery {
    #[serde(default)]
    client_id: String,
}

#[derive(Debug, Deserialize)]
struct JsonControlWsClientMessage {
    request_id: String,
    op: String,
    payload: sonic_rs::Value,
}

#[derive(Debug, Serialize)]
struct JsonControlWsServerMessage {
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<sonic_rs::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
}

const INSTRUCTION_PRICE_QUANTUM: u128 = 1_000_000_000;
static STATE_SNAPSHOT_TMP_SEQ: AtomicU64 = AtomicU64::new(1);

#[tokio::main]
async fn main() -> Result<()> {
    edgerun_observability::init_service("edgerun-scheduler")?;
    tracing::info!(planner_version = workflow_domain::planner_version(), "workflow domain loaded");

    let data_dir = std::env::var("EDGERUN_SCHEDULER_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".edgerun-scheduler-data"));
    std::fs::create_dir_all(data_dir.join("bundles"))?;

    let persisted = load_state(&data_dir)?;

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
    let policy_signing_key = load_policy_signing_key()?;
    let scheduler_event_bus = init_scheduler_event_bus().context("init scheduler event bus")?;
    let _edge_internal_broker =
        maybe_start_edge_internal_broker(&data_dir, scheduler_event_bus.clone())
            .await
            .context("start edge-internal event bus broker")?;

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
        job_timeout_secs: read_env_u64("EDGERUN_SCHEDULER_JOB_TIMEOUT_SECS", 60),
        policy_audit_path: data_dir.join("policy-audit.jsonl"),
        trust_policy: Arc::new(Mutex::new(trust_policy)),
        attestation_policy: Arc::new(Mutex::new(attestation_policy)),
        housekeeping_interval_secs: read_env_u64("EDGERUN_SCHEDULER_HOUSEKEEPING_INTERVAL_SECS", 5)
            .max(1),
        scheduler_event_bus,
        enforce_event_ingestion: read_env_bool("EDGERUN_SCHEDULER_ENFORCE_EVENT_INGESTION", true),
    };
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
                let payload = sonic_rs::to_string(&response).unwrap_or_else(|_| {
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
    let parsed = sonic_rs::from_str::<JsonControlWsClientMessage>(text).map_err(|_| {
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
            ControlWsRequestPayload::JobCreate(sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for job.create".to_string(),
                )
            })?)
        }
        "job.status" => {
            ControlWsRequestPayload::JobStatus(sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for job.status".to_string(),
                )
            })?)
        }
        "worker.heartbeat" => ControlWsRequestPayload::WorkerHeartbeat(
            sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.heartbeat".to_string(),
                )
            })?,
        ),
        "worker.assignments" => ControlWsRequestPayload::WorkerAssignments(
            sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.assignments".to_string(),
                )
            })?,
        ),
        "worker.result" => {
            ControlWsRequestPayload::WorkerResult(sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.result".to_string(),
                )
            })?)
        }
        "worker.failure" => ControlWsRequestPayload::WorkerFailure(
            sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.failure".to_string(),
                )
            })?,
        ),
        "worker.replay" => {
            ControlWsRequestPayload::WorkerReplay(sonic_rs::from_value(&payload).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid payload for worker.replay".to_string(),
                )
            })?)
        }
        "bundle.get" => {
            ControlWsRequestPayload::BundleGet(sonic_rs::from_value(&payload).map_err(|_| {
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
        .and_then(|payload| sonic_rs::to_value(&payload).ok());
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
        ControlWsRequestPayload::RouteChallenge(_)
        | ControlWsRequestPayload::RouteRegister(_)
        | ControlWsRequestPayload::RouteHeartbeat(_)
        | ControlWsRequestPayload::RouteResolve(_)
        | ControlWsRequestPayload::RouteOwner(_) => control_error_response(
            request_id,
            StatusCode::GONE,
            "route control operations removed; use libp2p cluster networking".to_string(),
        ),
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

const EVENT_PUBLISHER_SCHEDULER: &str = "scheduler";

fn topic_for_payload_type(payload_type: &str) -> Result<EventTopic, (StatusCode, String)> {
    let (overlay, name) = match payload_type {
        EVENT_PAYLOAD_TYPE_WORKER_HEARTBEAT => (
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_HEARTBEAT,
        ),
        EVENT_PAYLOAD_TYPE_WORKER_ASSIGNMENTS_POLL => (
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_ASSIGNMENTS_POLL,
        ),
        EVENT_PAYLOAD_TYPE_WORKER_RESULT => (
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_RESULT,
        ),
        EVENT_PAYLOAD_TYPE_WORKER_FAILURE => (
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_FAILURE,
        ),
        EVENT_PAYLOAD_TYPE_WORKER_REPLAY => (
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_REPLAY,
        ),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown event payload_type '{payload_type}'"),
            ));
        }
    };
    EventTopic::new(overlay, name).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("invalid configured topic '{name}': {err}"),
        )
    })
}

fn ingest_scheduler_event<T: Serialize>(
    state: &AppState,
    payload_type: &str,
    payload: &T,
) -> Result<(), (StatusCode, String)> {
    let encoded = bincode::serialize(payload).map_err(internal_err)?;
    let topic = topic_for_payload_type(payload_type)?;
    match state.scheduler_event_bus.publish(
        &topic,
        EVENT_PUBLISHER_SCHEDULER.to_string(),
        payload_type.to_string(),
        encoded,
    ) {
        Ok(_) => Ok(()),
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
                    "control-plane event publish failed (continuing because enforcement is disabled)"
                );
                Ok(())
            }
        }
    }
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
        schema_version: EVENT_SCHEMA_VERSION_V1,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_PAYLOAD_TYPE_WORKER_HEARTBEAT, &ingest_event)?;
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
        schema_version: EVENT_SCHEMA_VERSION_V1,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(
        state,
        EVENT_PAYLOAD_TYPE_WORKER_ASSIGNMENTS_POLL,
        &ingest_event,
    )?;
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
        schema_version: EVENT_SCHEMA_VERSION_V1,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_PAYLOAD_TYPE_WORKER_RESULT, &ingest_event)?;

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
        schema_version: EVENT_SCHEMA_VERSION_V1,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_PAYLOAD_TYPE_WORKER_FAILURE, &ingest_event)?;

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
        schema_version: EVENT_SCHEMA_VERSION_V1,
        observed_at_unix_ms: now_unix_millis(),
        payload: payload.clone(),
    };
    ingest_scheduler_event(state, EVENT_PAYLOAD_TYPE_WORKER_REPLAY, &ingest_event)?;

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
            let client = parse_pubkey_bytes(client_pubkey).map_err(|_| {
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

    let _ = (parsed_client, bundle_hash, runtime_id);
    let (post_job_tx, post_job_sig) = (String::new(), None);

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

fn init_scheduler_event_bus() -> Result<RuntimeEventBus> {
    let capacity = read_env_usize("EDGERUN_EVENT_BUS_TOPIC_CAPACITY", 1024).max(8);
    let topics = vec![
        EventTopic::new(
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_HEARTBEAT,
        )?,
        EventTopic::new(
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_ASSIGNMENTS_POLL,
        )?,
        EventTopic::new(
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_RESULT,
        )?,
        EventTopic::new(
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_FAILURE,
        )?,
        EventTopic::new(
            OverlayNetwork::EdgeInternal,
            EVENT_TOPIC_SCHEDULER_WORKER_REPLAY,
        )?,
    ];
    RuntimeEventBus::with_topics(capacity, &topics).map_err(Into::into)
}

async fn maybe_start_edge_internal_broker(
    data_dir: &FsPath,
    bus: RuntimeEventBus,
) -> Result<Option<EdgeInternalBrokerHandle>> {
    if !read_env_bool("EDGERUN_EVENT_BUS_EDGE_INTERNAL_ENABLED", true) {
        tracing::info!("edge-internal event bus broker disabled");
        return Ok(None);
    }
    let socket_path = std::env::var("EDGERUN_EVENT_BUS_EDGE_INTERNAL_SOCK")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir.join("event-bus/edge-internal.sock"));
    let handle = spawn_edge_internal_broker(&socket_path, bus).await?;
    tracing::info!(
        socket = %handle.socket_path.display(),
        "edge-internal event bus broker enabled"
    );
    Ok(Some(handle))
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
        latest_chain_progress_event_id: None,
        latest_chain_progress_bus_nonce: None,
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
    tracing::warn!(
        job_id = %job_id_hex,
        count = candidate_workers.len(),
        "slash artifacts skipped: chain transaction support disabled in this build"
    );
    Vec::new()
}

fn build_finalize_trigger_payload_inner(
    state: &AppState,
    job_id_hex: &str,
    committee_workers: &[String],
    winning_workers: &[String],
) -> Result<(String, Option<String>, bool)> {
    let _ = (state, job_id_hex, committee_workers, winning_workers);
    anyhow::bail!("finalize transaction generation disabled: chain support removed")
}

fn build_assign_workers_artifact(
    state: &AppState,
    job_id_hex: &str,
) -> Result<(Option<String>, Option<String>, bool)> {
    let _ = (state, job_id_hex);
    Ok((None, None, false))
}

fn evaluate_expired_jobs(state: &AppState) -> Result<()> {
    let now = now_unix_seconds();
    let timeout = state.job_timeout_secs.max(1);
    let candidates = {
        let job_quorum = state.job_quorum.lock().expect("lock poisoned");
        job_quorum
            .iter()
            .filter_map(|(job_id, quorum_state)| {
                let expired = if let Some(deadline_slot) = quorum_state.onchain_deadline_slot {
                    let _ = deadline_slot;
                    now.saturating_sub(quorum_state.created_at_unix_s) >= timeout
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
    let _ = state;
    tracing::warn!(
        job_id = %job_id_hex,
        "cancel_expired transaction generation skipped: chain support removed"
    );
    Ok((None, None, false))
}

fn build_finalize_trigger_payload(
    state: &AppState,
    job_id_hex: &str,
    committee_workers: &[String],
    winning_workers: &[String],
) -> Result<(String, Option<String>, bool)> {
    build_finalize_trigger_payload_inner(state, job_id_hex, committee_workers, winning_workers)
}

fn parse_pubkey_bytes(value: &str) -> Result<[u8; 32], ()> {
    let decoded = bs58::decode(value.trim()).into_vec().map_err(|_| ())?;
    decoded.as_slice().try_into().map_err(|_| ())
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
    let wasm_hash_hex = hex::encode(edgerun_crypto::blake3_256(wasm));
    let input_hash_hex = hex::encode(edgerun_crypto::blake3_256(input));
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
    client_pubkey: [u8; 32],
    signature_b64: &str,
    message: &str,
) -> Result<bool, (StatusCode, String)> {
    let client_vk = VerifyingKey::from_bytes(&client_pubkey).map_err(|_| {
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

    let worker_pk_bytes = parse_pubkey_bytes(worker_pubkey).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "worker_pubkey must be base58 pubkey".to_string(),
        )
    })?;
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
    let worker_bytes = parse_pubkey_bytes(&payload.worker_pubkey).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "worker_pubkey must be base58 pubkey".to_string(),
        )
    })?;
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

async fn housekeeping_loop(state: AppState) {
    let interval_secs = state.housekeeping_interval_secs;
    loop {
        if let Err(err) = evaluate_expired_jobs(&state) {
            tracing::warn!(error = %err, "housekeeping evaluate_expired_jobs failed");
        }
        if let Err(err) = flush_state_snapshot_if_dirty(&state) {
            tracing::warn!(error = %err, "housekeeping state snapshot flush failed");
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
