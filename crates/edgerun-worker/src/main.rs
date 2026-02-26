// SPDX-License-Identifier: Apache-2.0
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use edgerun_control_signing::{
    assignments_signing_message, failure_signing_message, heartbeat_signing_message,
    replay_signing_message, result_signing_message,
};
use edgerun_types::control_plane::{
    assignment_policy_message, default_policy_key_id, default_policy_version, AssignmentsResponse,
    BundleGetRequest, ControlWsClientMessage, ControlWsRequestPayload, ControlWsResponsePayload,
    ControlWsServerMessage, HeartbeatRequest, HeartbeatResponse, PolicyInfoResponse,
    QueuedAssignment, SessionCreateRequest, SessionCreateResponse, WorkerAssignmentsRequest,
    WorkerCapacity, WorkerFailureReport, WorkerReplayArtifactReport, WorkerResultReport,
};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use reqwest::Url;
use serde::Serialize;
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    hash::hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    sysvar,
    transaction::Transaction,
};
use solana_system_interface::program as system_program;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug)]
struct WorkerConfig {
    worker_pubkey: String,
    scheduler_base_url: String,
    runtime_ids: Vec<String>,
    version: String,
    capacity: WorkerCapacity,
    worker_signing_key: Option<SigningKey>,
    chain_submit: Option<ChainSubmitConfig>,
    policy_verifiers: Arc<RwLock<Vec<PolicyVerifier>>>,
    policy_session: Arc<Mutex<PolicySessionState>>,
    policy_clock_skew_secs: u64,
    policy_refresh_interval: Duration,
    pending_queue_max: usize,
    retry_base_ms: u64,
    retry_max_ms: u64,
    retry_flush_batch: usize,
}

#[derive(Debug)]
struct ChainSubmitConfig {
    rpc_url: String,
    program_id: Pubkey,
    wallet: Keypair,
}

#[derive(Debug, Clone)]
struct PolicyVerifier {
    key_id: String,
    version: u32,
    verify_pubkey_hex: String,
}

#[derive(Debug)]
struct PolicySessionState {
    token: Option<String>,
    session_key: Option<String>,
    expires_at_unix_s: u64,
    bound_origin: Option<String>,
    bootstrap_token: Option<String>,
    nonce_counter: u64,
}

impl PolicySessionState {
    fn has_live_session(&self, now: u64) -> bool {
        self.token.is_some()
            && self.session_key.is_some()
            && self.expires_at_unix_s > now.saturating_add(5)
    }
}

type ReplayArtifactPayload = edgerun_types::control_plane::ReplayArtifactPayload;
const CONTROL_WS_TIMEOUT: Duration = Duration::from_secs(15);
static CONTROL_REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy)]
enum SubmissionKind {
    Result,
    Failure,
    Replay,
}

impl SubmissionKind {
    fn op(self) -> &'static str {
        match self {
            SubmissionKind::Result => "worker.result",
            SubmissionKind::Failure => "worker.failure",
            SubmissionKind::Replay => "worker.replay",
        }
    }
}

#[derive(Debug)]
struct PendingSubmission {
    kind: SubmissionKind,
    idempotency_key: String,
    body: ControlWsRequestPayload,
    attempts: u32,
    next_attempt_at: Instant,
}

#[derive(Debug)]
struct SubmissionQueue {
    items: VecDeque<PendingSubmission>,
    max_len: usize,
    base_backoff: Duration,
    max_backoff: Duration,
    flush_batch: usize,
}

const DEFAULT_POLICY_SIGNING_KEY_HEX: &str =
    "0101010101010101010101010101010101010101010101010101010101010101";

fn install_rustls_crypto_provider() {
    // rustls 0.23 panics when more than one provider backend is compiled unless
    // a process-level provider is explicitly installed.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[tokio::main]
async fn main() -> Result<()> {
    install_rustls_crypto_provider();
    edgerun_observability::init_service("edgerun-worker")?;

    let cfg = load_config();
    let p2p_runtime = edgerun_p2p::spawn_event_bus_from_env("worker")
        .await
        .context("failed to initialize worker p2p runtime")?;
    if let Some(runtime) = p2p_runtime {
        let edgerun_p2p::P2pEventBusHandle { inbound_rx, .. } = runtime;
        let mut inbound_rx = inbound_rx;
        tokio::spawn(async move {
            while let Some(payload) = inbound_rx.recv().await {
                tracing::info!(
                    bytes = payload.len(),
                    "worker received libp2p event bus message"
                );
            }
        });
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let mut submission_queue = SubmissionQueue {
        items: VecDeque::new(),
        max_len: cfg.pending_queue_max,
        base_backoff: Duration::from_millis(cfg.retry_base_ms.max(1)),
        max_backoff: Duration::from_millis(cfg.retry_max_ms.max(cfg.retry_base_ms.max(1))),
        flush_batch: cfg.retry_flush_batch.max(1),
    };
    let mut last_policy_refresh = Instant::now();

    tracing::info!(
        worker = %cfg.worker_pubkey,
        scheduler = %cfg.scheduler_base_url,
        "edgerun-worker loop starting"
    );

    let mut next_poll_ms = 2000;
    loop {
        if last_policy_refresh.elapsed() >= cfg.policy_refresh_interval {
            if let Err(err) = refresh_policy_verifiers(&client, &cfg).await {
                tracing::warn!(error = %err, "policy verifier refresh failed");
            }
            last_policy_refresh = Instant::now();
        }

        flush_submission_queue(&cfg, &mut submission_queue).await;

        if let Ok(resp) = send_heartbeat(&cfg).await {
            if resp.ok {
                next_poll_ms = resp.next_poll_ms.max(200);
            }
        }

        match poll_assignments(&cfg).await {
            Ok(assignments) => {
                for assignment in assignments.jobs {
                    if let Err(err) =
                        process_assignment(&cfg, &mut submission_queue, assignment).await
                    {
                        tracing::error!(error = %err, "assignment processing failed");
                    }
                }
            }
            Err(err) => tracing::error!(error = %err, "assignment poll failed"),
        }

        flush_submission_queue(&cfg, &mut submission_queue).await;

        tokio::time::sleep(Duration::from_millis(next_poll_ms)).await;
    }
}

fn load_config() -> WorkerConfig {
    let mut worker_pubkey =
        std::env::var("EDGERUN_WORKER_PUBKEY").unwrap_or_else(|_| "worker-demo-1".to_string());
    let scheduler_base_url = std::env::var("EDGERUN_SCHEDULER_URL")
        .unwrap_or_else(|_| "https://api.edgerun.tech".to_string());
    let runtime_ids = std::env::var("EDGERUN_WORKER_RUNTIME_IDS")
        .ok()
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            vec!["0000000000000000000000000000000000000000000000000000000000000000".to_string()]
        });
    let version = std::env::var("EDGERUN_WORKER_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    let capacity = WorkerCapacity {
        max_concurrent: std::env::var("EDGERUN_WORKER_MAX_CONCURRENT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(1),
        mem_bytes: std::env::var("EDGERUN_WORKER_MEM_BYTES")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(268_435_456),
    };
    let worker_signing_key = std::env::var("EDGERUN_WORKER_SIGNING_KEY_HEX")
        .ok()
        .map(|hex_key| parse_signing_key_hex(&hex_key))
        .transpose()
        .unwrap_or_else(|err| {
            tracing::warn!(error = %err, "invalid EDGERUN_WORKER_SIGNING_KEY_HEX; disabling worker request signatures");
            None
        });
    let chain_submit = load_chain_submit_config();
    if let Some(chain_cfg) = chain_submit.as_ref() {
        let chain_worker = chain_cfg.wallet.pubkey().to_string();
        if worker_pubkey != chain_worker {
            tracing::warn!(
                configured_worker_pubkey = %worker_pubkey,
                chain_wallet_pubkey = %chain_worker,
                "EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED requires worker_pubkey to match chain wallet; overriding"
            );
            worker_pubkey = chain_worker;
        }
    }
    let policy_verify_pubkey_hex = std::env::var("EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(default_policy_verify_pubkey_hex);
    let expected_policy_key_id = std::env::var("EDGERUN_WORKER_POLICY_KEY_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(default_policy_key_id);
    let expected_policy_version = std::env::var("EDGERUN_WORKER_POLICY_VERSION")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or_else(default_policy_version);
    let mut policy_verifiers = vec![PolicyVerifier {
        key_id: expected_policy_key_id,
        version: expected_policy_version,
        verify_pubkey_hex: policy_verify_pubkey_hex,
    }];
    let next_key_id = std::env::var("EDGERUN_WORKER_POLICY_KEY_ID_NEXT")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let next_pubkey = std::env::var("EDGERUN_WORKER_POLICY_VERIFY_KEY_HEX_NEXT")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let next_version = std::env::var("EDGERUN_WORKER_POLICY_VERSION_NEXT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0);
    if let (Some(key_id), Some(verify_pubkey_hex), Some(version)) =
        (next_key_id, next_pubkey, next_version)
    {
        policy_verifiers.push(PolicyVerifier {
            key_id,
            version,
            verify_pubkey_hex,
        });
    }
    let policy_clock_skew_secs = std::env::var("EDGERUN_WORKER_POLICY_CLOCK_SKEW_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    let policy_session_origin = std::env::var("EDGERUN_WORKER_POLICY_SESSION_ORIGIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let policy_session_bootstrap_token =
        std::env::var("EDGERUN_WORKER_POLICY_SESSION_BOOTSTRAP_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
    let policy_refresh_secs = std::env::var("EDGERUN_WORKER_POLICY_REFRESH_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(30);
    let pending_queue_max = std::env::var("EDGERUN_WORKER_PENDING_QUEUE_MAX")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(2048);
    let retry_base_ms = std::env::var("EDGERUN_WORKER_RETRY_BASE_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(1000);
    let retry_max_ms = std::env::var("EDGERUN_WORKER_RETRY_MAX_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(60_000);
    let retry_flush_batch = std::env::var("EDGERUN_WORKER_RETRY_FLUSH_BATCH")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(64);

    WorkerConfig {
        worker_pubkey,
        scheduler_base_url,
        runtime_ids,
        version,
        capacity,
        worker_signing_key,
        chain_submit,
        policy_verifiers: Arc::new(RwLock::new(policy_verifiers)),
        policy_session: Arc::new(Mutex::new(PolicySessionState {
            token: None,
            session_key: None,
            expires_at_unix_s: 0,
            bound_origin: policy_session_origin,
            bootstrap_token: policy_session_bootstrap_token,
            nonce_counter: 0,
        })),
        policy_clock_skew_secs,
        policy_refresh_interval: Duration::from_secs(policy_refresh_secs),
        pending_queue_max,
        retry_base_ms,
        retry_max_ms,
        retry_flush_batch,
    }
}

fn load_chain_submit_config() -> Option<ChainSubmitConfig> {
    if !read_env_bool("EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED", false) {
        return None;
    }

    let rpc_url = std::env::var("EDGERUN_CHAIN_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let program_id = std::env::var("EDGERUN_CHAIN_PROGRAM_ID")
        .ok()
        .and_then(|v| v.parse::<Pubkey>().ok());
    let wallet_path = std::env::var("EDGERUN_CHAIN_WALLET")
        .unwrap_or_else(|_| "program/.solana/id.json".to_string());

    let Some(program_id) = program_id else {
        tracing::warn!(
            "EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED=true but EDGERUN_CHAIN_PROGRAM_ID is missing/invalid"
        );
        return None;
    };
    let wallet = match read_keypair_file(&wallet_path) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "EDGERUN_WORKER_CHAIN_SUBMIT_ENABLED=true but failed to read EDGERUN_CHAIN_WALLET"
            );
            return None;
        }
    };

    Some(ChainSubmitConfig {
        rpc_url,
        program_id,
        wallet,
    })
}

async fn send_heartbeat(cfg: &WorkerConfig) -> Result<HeartbeatResponse> {
    let mut payload = HeartbeatRequest {
        worker_pubkey: cfg.worker_pubkey.clone(),
        runtime_ids: cfg.runtime_ids.clone(),
        version: cfg.version.clone(),
        capacity: Some(cfg.capacity.clone()),
        signature: None,
    };
    payload.signature = sign_worker_payload(cfg, heartbeat_signing_message(&payload));
    let response = control_ws_request(cfg, ControlWsRequestPayload::WorkerHeartbeat(payload))
        .await
        .context("heartbeat request failed")?;
    match response {
        ControlWsResponsePayload::WorkerHeartbeat(v) => Ok(v),
        other => anyhow::bail!(
            "heartbeat request returned unexpected payload variant: {:?}",
            other
        ),
    }
}

async fn poll_assignments(cfg: &WorkerConfig) -> Result<AssignmentsResponse> {
    let mut payload = WorkerAssignmentsRequest {
        worker_pubkey: cfg.worker_pubkey.clone(),
        signed_at_unix_s: now_unix_seconds(),
        signature: None,
    };
    payload.signature = sign_worker_payload(cfg, assignments_signing_message(&payload));
    let response = control_ws_request(
        cfg,
        ControlWsRequestPayload::WorkerAssignments(payload),
    )
    .await
    .context("assignments request failed")?;
    match response {
        ControlWsResponsePayload::WorkerAssignments(v) => Ok(*v),
        other => anyhow::bail!(
            "assignments request returned unexpected payload variant: {:?}",
            other
        ),
    }
}

async fn refresh_policy_verifiers(client: &reqwest::Client, cfg: &WorkerConfig) -> Result<()> {
    let info = fetch_policy_info(client, cfg).await?;

    let verifier = PolicyVerifier {
        key_id: info.key_id.clone(),
        version: info.version,
        verify_pubkey_hex: info.signer_pubkey.clone(),
    };
    let mut verifiers = cfg.policy_verifiers.write().expect("lock poisoned");
    if let Some(existing) = verifiers
        .iter_mut()
        .find(|v| v.key_id == verifier.key_id && v.version == verifier.version)
    {
        if existing.verify_pubkey_hex != verifier.verify_pubkey_hex {
            tracing::warn!(
                key_id = %verifier.key_id,
                version = verifier.version,
                "policy verifier key changed for existing tuple; updating"
            );
            existing.verify_pubkey_hex = verifier.verify_pubkey_hex;
        }
    } else {
        verifiers.push(verifier);
        tracing::info!(
            key_id = %info.key_id,
            version = info.version,
            ttl_secs = info.ttl_secs,
            "added policy verifier from scheduler policy info"
        );
    }
    Ok(())
}

async fn fetch_policy_info(
    client: &reqwest::Client,
    cfg: &WorkerConfig,
) -> Result<PolicyInfoResponse> {
    let path = "/v1/policy/info";
    let url = Url::parse(&format!("{}{}", cfg.scheduler_base_url, path))?;
    let mut attempted_session_create = false;
    loop {
        let mut req = client.get(url.clone());
        if let Some(headers) = build_policy_session_headers(cfg, "GET", path, b"")? {
            req = req
                .header("authorization", headers.authorization)
                .header("x-hwv-ts", headers.ts)
                .header("x-hwv-nonce", headers.nonce)
                .header("x-hwv-sig", headers.sig);
            if let Some(origin) = headers.origin {
                req = req.header("origin", origin);
            }
        }
        let resp = req.send().await.context("policy info request failed")?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<PolicyInfoResponse>()
                .await
                .context("invalid policy info response");
        }
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            if attempted_session_create {
                tracing::debug!(
                    status = %status,
                    "policy info endpoint denied access; keeping configured policy verifiers"
                );
                anyhow::bail!("policy info failed with status {status}");
            }
            attempted_session_create = true;
            establish_policy_session(client, cfg)
                .await
                .context("failed to establish policy session")?;
            continue;
        }
        anyhow::bail!("policy info failed with status {status}");
    }
}

#[derive(Debug)]
struct PolicySessionHeaders {
    authorization: String,
    origin: Option<String>,
    ts: String,
    nonce: String,
    sig: String,
}

type HmacSha256 = Hmac<Sha256>;

fn build_policy_session_headers(
    cfg: &WorkerConfig,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Option<PolicySessionHeaders>> {
    let now = now_unix_seconds();
    let mut session = cfg.policy_session.lock().expect("lock poisoned");
    if !session.has_live_session(now) {
        return Ok(None);
    }
    let token = session.token.clone().unwrap_or_default();
    let key = session.session_key.clone().unwrap_or_default();
    let origin = session.bound_origin.clone();
    session.nonce_counter = session.nonce_counter.saturating_add(1);
    let nonce = format!("worker-{}-{}", now, session.nonce_counter);
    drop(session);

    let ts = now.to_string();
    let mut hasher = Sha256::new();
    hasher.update(body);
    let body_hash = URL_SAFE_NO_PAD.encode(hasher.finalize());
    let canonical = format!("{method}|{path}|{ts}|{nonce}|{body_hash}");
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).context("hmac key")?;
    mac.update(canonical.as_bytes());
    let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(Some(PolicySessionHeaders {
        authorization: format!("Bearer {token}"),
        origin,
        ts,
        nonce,
        sig,
    }))
}

async fn establish_policy_session(client: &reqwest::Client, cfg: &WorkerConfig) -> Result<()> {
    let (bootstrap_token, bound_origin) = {
        let mut session = cfg.policy_session.lock().expect("lock poisoned");
        session.token = None;
        session.session_key = None;
        session.expires_at_unix_s = 0;
        session.nonce_counter = 0;
        (
            session.bootstrap_token.clone(),
            session.bound_origin.clone(),
        )
    };
    let path = "/v1/session/create";
    let url = Url::parse(&format!("{}{}", cfg.scheduler_base_url, path))?;
    let payload = SessionCreateRequest {
        bound_origin: bound_origin.clone(),
    };
    let mut req = client.post(url).json(&payload);
    if let Some(token) = bootstrap_token {
        req = req.header("x-edgerun-bootstrap-token", token);
    }

    let resp = req.send().await.context("session create request failed")?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("session create failed with status {status}");
    }
    let issued = resp
        .json::<SessionCreateResponse>()
        .await
        .context("invalid session create response")?;
    let now = now_unix_seconds();
    let mut session = cfg.policy_session.lock().expect("lock poisoned");
    session.token = Some(issued.token);
    session.session_key = Some(issued.session_key);
    session.expires_at_unix_s = now.saturating_add(issued.ttl_secs.max(1));
    session.nonce_counter = 0;
    Ok(())
}

async fn process_assignment(
    cfg: &WorkerConfig,
    queue: &mut SubmissionQueue,
    assignment: QueuedAssignment,
) -> Result<()> {
    tracing::info!(
        job_id = %assignment.job_id,
        bundle_hash = %assignment.bundle_hash,
        runtime_id = %assignment.runtime_id,
        max_memory = assignment.limits.max_memory_bytes,
        max_instructions = assignment.limits.max_instructions,
        escrow_lamports = assignment.escrow_lamports,
        "processing assignment"
    );

    if let Err(err) = verify_assignment_policy(cfg, &assignment) {
        let msg = format!("assignment policy verification failed: {err}");
        submit_failure_report(
            cfg,
            queue,
            WorkerFailureReport {
                idempotency_key: idempotency_key(
                    "failure",
                    &cfg.worker_pubkey,
                    &assignment.job_id,
                    "assignment_policy_verify",
                    "AssignmentPolicyInvalid",
                    &assignment.bundle_hash,
                ),
                worker_pubkey: cfg.worker_pubkey.clone(),
                job_id: assignment.job_id.clone(),
                bundle_hash: assignment.bundle_hash.clone(),
                phase: "assignment_policy_verify".to_string(),
                error_code: "AssignmentPolicyInvalid".to_string(),
                error_message: msg.clone(),
                signature: None,
            },
        )
        .await;
        anyhow::bail!(msg);
    }

    let bundle_response = match control_ws_request(
        cfg,
        ControlWsRequestPayload::BundleGet(BundleGetRequest {
            bundle_hash: assignment.bundle_hash.clone(),
        }),
    )
    .await
    .context("bundle fetch failed")?
    {
        ControlWsResponsePayload::BundleGet(v) => v,
        other => anyhow::bail!(
            "bundle.get request returned unexpected payload variant: {:?}",
            other
        ),
    };
    let bundle_bytes = base64::engine::general_purpose::STANDARD
        .decode(bundle_response.payload_b64.as_bytes())
        .context("bundle payload base64 decode failed")?;

    let expected_runtime_id = match parse_runtime_id_hex(&assignment.runtime_id) {
        Ok(value) => value,
        Err(err) => {
            let msg = err.to_string();
            submit_failure_report(
                cfg,
                queue,
                WorkerFailureReport {
                    idempotency_key: idempotency_key(
                        "failure",
                        &cfg.worker_pubkey,
                        &assignment.job_id,
                        "assignment_validation",
                        "InvalidAssignmentRuntimeId",
                        &assignment.bundle_hash,
                    ),
                    worker_pubkey: cfg.worker_pubkey.clone(),
                    job_id: assignment.job_id.clone(),
                    bundle_hash: assignment.bundle_hash.clone(),
                    phase: "assignment_validation".to_string(),
                    error_code: "InvalidAssignmentRuntimeId".to_string(),
                    error_message: msg.clone(),
                    signature: None,
                },
            )
            .await;
            return Err(anyhow::anyhow!(msg));
        }
    };

    let exec = edgerun_runtime::execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
        bundle_bytes.as_ref(),
        expected_runtime_id,
        assignment.abi_version,
    );
    let report = match exec {
        Ok(report) => report,
        Err(err) => {
            let err_code = format!("{:?}", err.code);
            let err_message = err.message.clone();
            submit_replay_artifact(
                cfg,
                queue,
                WorkerReplayArtifactReport {
                    idempotency_key: idempotency_key(
                        "replay",
                        &cfg.worker_pubkey,
                        &assignment.job_id,
                        "runtime_execute",
                        &err_code,
                        &assignment.bundle_hash,
                    ),
                    worker_pubkey: cfg.worker_pubkey.clone(),
                    job_id: assignment.job_id.clone(),
                    artifact: ReplayArtifactPayload {
                        bundle_hash: assignment.bundle_hash.clone(),
                        ok: false,
                        abi_version: None,
                        runtime_id: Some(assignment.runtime_id.clone()),
                        output_hash: None,
                        output_len: None,
                        input_len: None,
                        max_memory_bytes: None,
                        max_instructions: None,
                        fuel_limit: err.fuel_limit,
                        fuel_remaining: err.fuel_remaining,
                        error_code: Some(err_code.clone()),
                        error_message: Some(err_message.clone()),
                        trap_code: err.trap_code,
                    },
                    signature: None,
                },
            )
            .await;
            submit_failure_report(
                cfg,
                queue,
                WorkerFailureReport {
                    idempotency_key: idempotency_key(
                        "failure",
                        &cfg.worker_pubkey,
                        &assignment.job_id,
                        "runtime_execute",
                        &err_code,
                        &assignment.bundle_hash,
                    ),
                    worker_pubkey: cfg.worker_pubkey.clone(),
                    job_id: assignment.job_id.clone(),
                    bundle_hash: assignment.bundle_hash.clone(),
                    phase: "runtime_execute".to_string(),
                    error_code: err_code,
                    error_message: err_message.clone(),
                    signature: None,
                },
            )
            .await;
            return Err(anyhow::anyhow!(err_message));
        }
    };

    let computed_bundle_hash = hex::encode(report.bundle_hash);
    if computed_bundle_hash != assignment.bundle_hash.to_lowercase() {
        let msg = format!(
            "bundle hash mismatch: scheduler={} worker={}",
            assignment.bundle_hash, computed_bundle_hash
        );
        submit_replay_artifact(
            cfg,
            queue,
            WorkerReplayArtifactReport {
                idempotency_key: idempotency_key(
                    "replay",
                    &cfg.worker_pubkey,
                    &assignment.job_id,
                    "post_execution_verify",
                    "BundleHashMismatch",
                    &assignment.bundle_hash,
                ),
                worker_pubkey: cfg.worker_pubkey.clone(),
                job_id: assignment.job_id.clone(),
                artifact: ReplayArtifactPayload {
                    bundle_hash: assignment.bundle_hash.clone(),
                    ok: false,
                    abi_version: Some(report.abi_version),
                    runtime_id: Some(hex::encode(report.runtime_id)),
                    output_hash: None,
                    output_len: None,
                    input_len: Some(report.input_len),
                    max_memory_bytes: Some(report.max_memory_bytes),
                    max_instructions: Some(report.max_instructions),
                    fuel_limit: Some(report.fuel_limit),
                    fuel_remaining: Some(report.fuel_remaining),
                    error_code: Some("BundleHashMismatch".to_string()),
                    error_message: Some(msg.clone()),
                    trap_code: None,
                },
                signature: None,
            },
        )
        .await;
        submit_failure_report(
            cfg,
            queue,
            WorkerFailureReport {
                idempotency_key: idempotency_key(
                    "failure",
                    &cfg.worker_pubkey,
                    &assignment.job_id,
                    "post_execution_verify",
                    "BundleHashMismatch",
                    &assignment.bundle_hash,
                ),
                worker_pubkey: cfg.worker_pubkey.clone(),
                job_id: assignment.job_id.clone(),
                bundle_hash: assignment.bundle_hash.clone(),
                phase: "post_execution_verify".to_string(),
                error_code: "BundleHashMismatch".to_string(),
                error_message: msg.clone(),
                signature: None,
            },
        )
        .await;
        anyhow::bail!(msg);
    }

    let mut result = WorkerResultReport {
        idempotency_key: idempotency_key(
            "result",
            &cfg.worker_pubkey,
            &assignment.job_id,
            "runtime_execute",
            &hex::encode(report.output_hash),
            &computed_bundle_hash,
        ),
        worker_pubkey: cfg.worker_pubkey.clone(),
        job_id: assignment.job_id,
        bundle_hash: computed_bundle_hash,
        output_hash: hex::encode(report.output_hash),
        output_len: report.output_len,
        attestation_sig: None,
        attestation_claim: None,
        signature: None,
    };
    if let Some(chain_cfg) = cfg.chain_submit.as_ref() {
        match build_and_send_submit_result_tx(
            chain_cfg,
            &result.job_id,
            report.bundle_hash,
            report.output_hash,
            report.runtime_id,
        ) {
            Ok((tx_sig, attestation_sig_b64)) => {
                result.attestation_sig = Some(attestation_sig_b64);
                tracing::info!(job_id = %result.job_id, tx_sig = %tx_sig, "submitted on-chain result tx");
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    job_id = %result.job_id,
                    "failed to build/send on-chain submit_result tx"
                );
            }
        }
    }
    result.signature = sign_worker_payload(cfg, result_signing_message(&result));

    if !submit_with_retry(
        cfg,
        queue,
        SubmissionKind::Result,
        result.idempotency_key.clone(),
        &result,
    )
    .await
    {
        tracing::warn!(
            job_id = %result.job_id,
            idempotency_key = %result.idempotency_key,
            "result report queued for retry"
        );
    }

    let replay_payload = WorkerReplayArtifactReport {
        idempotency_key: idempotency_key(
            "replay",
            &cfg.worker_pubkey,
            &result.job_id,
            "runtime_execute",
            &result.output_hash,
            &result.bundle_hash,
        ),
        worker_pubkey: cfg.worker_pubkey.clone(),
        job_id: result.job_id.clone(),
        artifact: ReplayArtifactPayload {
            bundle_hash: result.bundle_hash.clone(),
            ok: true,
            abi_version: Some(report.abi_version),
            runtime_id: Some(hex::encode(report.runtime_id)),
            output_hash: Some(result.output_hash.clone()),
            output_len: Some(result.output_len),
            input_len: Some(report.input_len),
            max_memory_bytes: Some(report.max_memory_bytes),
            max_instructions: Some(report.max_instructions),
            fuel_limit: Some(report.fuel_limit),
            fuel_remaining: Some(report.fuel_remaining),
            error_code: None,
            error_message: None,
            trap_code: None,
        },
        signature: None,
    };

    submit_replay_artifact(cfg, queue, replay_payload).await;

    tracing::info!(
        output_hash = %result.output_hash,
        output_len = result.output_len,
        fuel_limit = report.fuel_limit,
        fuel_remaining = report.fuel_remaining,
        "assignment completed"
    );
    Ok(())
}

async fn submit_replay_artifact(
    cfg: &WorkerConfig,
    queue: &mut SubmissionQueue,
    mut replay_payload: WorkerReplayArtifactReport,
) {
    replay_payload.signature = sign_worker_payload(cfg, replay_signing_message(&replay_payload));
    if !submit_with_retry(
        cfg,
        queue,
        SubmissionKind::Replay,
        replay_payload.idempotency_key.clone(),
        &replay_payload,
    )
    .await
    {
        tracing::warn!(
            job_id = %replay_payload.job_id,
            idempotency_key = %replay_payload.idempotency_key,
            "replay artifact queued for retry"
        );
    }
}

async fn submit_failure_report(
    cfg: &WorkerConfig,
    queue: &mut SubmissionQueue,
    mut failure_payload: WorkerFailureReport,
) {
    failure_payload.signature = sign_worker_payload(cfg, failure_signing_message(&failure_payload));
    if !submit_with_retry(
        cfg,
        queue,
        SubmissionKind::Failure,
        failure_payload.idempotency_key.clone(),
        &failure_payload,
    )
    .await
    {
        tracing::warn!(
            job_id = %failure_payload.job_id,
            idempotency_key = %failure_payload.idempotency_key,
            "failure report queued for retry"
        );
    }
}

async fn submit_with_retry<T: Serialize>(
    cfg: &WorkerConfig,
    queue: &mut SubmissionQueue,
    kind: SubmissionKind,
    idempotency_key: String,
    payload: &T,
) -> bool {
    let body = match kind {
        SubmissionKind::Result => {
            let Some(value) = bincode::serialize(payload)
                .ok()
                .and_then(|bytes| bincode::deserialize::<WorkerResultReport>(&bytes).ok())
            else {
                tracing::error!("failed to encode worker.result payload");
                return false;
            };
            ControlWsRequestPayload::WorkerResult(value)
        }
        SubmissionKind::Failure => {
            let Some(value) = bincode::serialize(payload)
                .ok()
                .and_then(|bytes| bincode::deserialize::<WorkerFailureReport>(&bytes).ok())
            else {
                tracing::error!("failed to encode worker.failure payload");
                return false;
            };
            ControlWsRequestPayload::WorkerFailure(value)
        }
        SubmissionKind::Replay => {
            let Some(value) = bincode::serialize(payload)
                .ok()
                .and_then(|bytes| bincode::deserialize::<WorkerReplayArtifactReport>(&bytes).ok())
            else {
                tracing::error!("failed to encode worker.replay payload");
                return false;
            };
            ControlWsRequestPayload::WorkerReplay(value)
        }
    };

    if let Err(err) = post_submission(cfg, kind, &body).await {
        enqueue_submission(queue, kind, idempotency_key, body);
        tracing::warn!(error = %err, op = kind.op(), "submission queued for retry");
        return false;
    }

    true
}

async fn flush_submission_queue(cfg: &WorkerConfig, queue: &mut SubmissionQueue) {
    let now = Instant::now();
    let mut inspected = 0usize;
    let mut sent = 0usize;
    let max_inspect = queue.items.len().min(queue.flush_batch.max(1));

    while inspected < max_inspect {
        inspected += 1;
        let Some(mut item) = queue.items.pop_front() else {
            break;
        };
        if item.next_attempt_at > now {
            queue.items.push_back(item);
            continue;
        }

        match post_submission(cfg, item.kind, &item.body).await {
            Ok(()) => {
                sent += 1;
            }
            Err(err) => {
                item.attempts = item.attempts.saturating_add(1);
                item.next_attempt_at = Instant::now() + retry_backoff_delay(queue, item.attempts);
                tracing::warn!(
                    error = %err,
                    op = item.kind.op(),
                    idempotency_key = %item.idempotency_key,
                    attempts = item.attempts,
                    "queued submission retry failed"
                );
                queue.items.push_back(item);
            }
        }
    }

    if sent > 0 {
        tracing::info!(
            sent = sent,
            remaining = queue.items.len(),
            "flushed queued worker submissions"
        );
    }
}

fn enqueue_submission(
    queue: &mut SubmissionQueue,
    kind: SubmissionKind,
    idempotency_key: String,
    body: ControlWsRequestPayload,
) {
    if queue
        .items
        .iter()
        .any(|item| item.idempotency_key == idempotency_key && item.kind.op() == kind.op())
    {
        return;
    }

    if queue.items.len() >= queue.max_len {
        if let Some(dropped) = queue.items.pop_front() {
            tracing::warn!(
                op = dropped.kind.op(),
                idempotency_key = %dropped.idempotency_key,
                "dropping oldest queued submission due to queue limit"
            );
        }
    }

    queue.items.push_back(PendingSubmission {
        kind,
        idempotency_key,
        body,
        attempts: 0,
        next_attempt_at: Instant::now(),
    });
}

fn retry_backoff_delay(queue: &SubmissionQueue, attempts: u32) -> Duration {
    let factor = 2u32.saturating_pow(attempts.saturating_sub(1).min(16));
    let delay = queue.base_backoff.saturating_mul(factor);
    delay.min(queue.max_backoff)
}

async fn post_submission(
    cfg: &WorkerConfig,
    kind: SubmissionKind,
    body: &ControlWsRequestPayload,
) -> Result<()> {
    let resp = control_ws_request(cfg, body.clone())
        .await
        .with_context(|| format!("submission request failed for {}", kind.op()))?;
    if !matches!(
        resp,
        ControlWsResponsePayload::WorkerResult(_)
            | ControlWsResponsePayload::WorkerFailure(_)
            | ControlWsResponsePayload::WorkerReplay(_)
    ) {
        anyhow::bail!(
            "submission request returned unexpected payload variant: {:?}",
            resp
        );
    }
    Ok(())
}

fn control_ws_url(base: &str, client_id: &str) -> Result<Url> {
    let mut url = Url::parse(base).context("invalid scheduler base url")?;
    let scheme = match url.scheme() {
        "http" => "ws".to_string(),
        "https" => "wss".to_string(),
        "ws" | "wss" => url.scheme().to_string(),
        other => {
            anyhow::bail!("unsupported scheduler scheme for control ws: {other}");
        }
    };
    url.set_scheme(&scheme)
        .map_err(|_| anyhow::anyhow!("failed to set websocket scheme"))?;
    url.set_path("/v1/control/ws");
    url.set_query(None);
    url.query_pairs_mut().append_pair("client_id", client_id);
    Ok(url)
}

fn next_control_request_id() -> String {
    let seq = CONTROL_REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{}-{seq}", now_unix_seconds())
}

async fn control_ws_request(
    cfg: &WorkerConfig,
    payload: ControlWsRequestPayload,
) -> Result<ControlWsResponsePayload> {
    let client_id = format!("worker-{}", cfg.worker_pubkey);
    let ws_url = control_ws_url(&cfg.scheduler_base_url, &client_id)?;
    let (mut socket, _resp) = timeout(
        CONTROL_WS_TIMEOUT,
        tokio_tungstenite::connect_async(ws_url.to_string()),
    )
    .await
    .context("control ws connect timed out")?
    .context("control ws connect failed")?;

    let request_id = next_control_request_id();
    let outbound = ControlWsClientMessage {
        request_id: request_id.clone(),
        payload,
    };
    let encoded = bincode::serialize(&outbound).context("control ws request encode failed")?;
    timeout(
        CONTROL_WS_TIMEOUT,
        socket.send(Message::Binary(encoded.into())),
    )
    .await
    .context("control ws send timed out")?
    .context("control ws send failed")?;

    loop {
        let next = timeout(CONTROL_WS_TIMEOUT, socket.next())
            .await
            .context("control ws receive timed out")?;
        let Some(frame) = next else {
            anyhow::bail!("control ws closed before response");
        };
        let frame = frame.context("control ws frame error")?;
        let Message::Binary(bytes) = frame else {
            continue;
        };
        let msg: ControlWsServerMessage =
            bincode::deserialize(&bytes).context("control ws response decode failed")?;
        if msg.request_id != request_id {
            continue;
        }
        if !msg.ok {
            let status = msg.status.map(|v| format!(" ({v})")).unwrap_or_default();
            let err = msg
                .error
                .unwrap_or_else(|| format!("control ws request failed{status}"));
            anyhow::bail!("{err}");
        }
        let data = msg
            .data
            .ok_or_else(|| anyhow::anyhow!("control ws response missing data"))?;
        return Ok(data);
    }
}

fn idempotency_key(
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

fn parse_signing_key_hex(value: &str) -> Result<SigningKey> {
    let bytes = hex::decode(value.trim()).context("worker signing key must be hex")?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("worker signing key must decode to 32 bytes"))?;
    Ok(SigningKey::from_bytes(&arr))
}

fn sign_worker_payload(cfg: &WorkerConfig, message: String) -> Option<String> {
    let signing_key = cfg.worker_signing_key.as_ref()?;
    let digest = edgerun_crypto::blake3_256(message.as_bytes());
    let sig = edgerun_crypto::sign(signing_key, &digest);
    Some(base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()))
}

fn build_and_send_submit_result_tx(
    cfg: &ChainSubmitConfig,
    job_id_hex: &str,
    bundle_hash: [u8; 32],
    output_hash: [u8; 32],
    runtime_id: [u8; 32],
) -> Result<(String, String)> {
    let job_id = parse_hex32(job_id_hex).context("job_id must be 32-byte hex")?;
    let worker = cfg.wallet.pubkey();
    let attestation_message = build_result_digest(&job_id, &bundle_hash, &output_hash, &runtime_id);
    let attestation_signature = {
        let sig = cfg.wallet.sign_message(&attestation_message);
        let mut out = [0_u8; 64];
        out.copy_from_slice(sig.as_ref());
        out
    };
    let worker_bytes = worker.to_bytes();
    let attestation_sig_b64 =
        base64::engine::general_purpose::STANDARD.encode(attestation_signature);

    let verify_ix = solana_sdk::ed25519_instruction::new_ed25519_instruction_with_signature(
        &attestation_message,
        &attestation_signature,
        &worker_bytes,
    );
    let (job_pda, _) = Pubkey::find_program_address(&[b"job", &job_id], &cfg.program_id);
    let (job_result_pda, _) =
        Pubkey::find_program_address(&[b"job_result", &job_id, worker.as_ref()], &cfg.program_id);

    let submit_ix = Instruction {
        program_id: cfg.program_id,
        accounts: vec![
            AccountMeta::new(worker, true),
            AccountMeta::new_readonly(job_pda, false),
            AccountMeta::new(job_result_pda, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: encode_submit_result_data(output_hash, attestation_signature),
    };

    let rpc = RpcClient::new(cfg.rpc_url.clone());
    let blockhash = rpc
        .get_latest_blockhash()
        .context("failed to fetch latest blockhash")?;
    let tx = Transaction::new_signed_with_payer(
        &[verify_ix, submit_ix],
        Some(&worker),
        &[&cfg.wallet],
        blockhash,
    );
    let signature = rpc
        .send_and_confirm_transaction(&tx)
        .context("failed to send submit_result transaction")?;
    Ok((signature.to_string(), attestation_sig_b64))
}

fn build_result_digest(
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

fn encode_submit_result_data(output_hash: [u8; 32], attestation_sig: [u8; 64]) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 32 + 64);
    data.extend_from_slice(&anchor_discriminator("submit_result"));
    data.extend_from_slice(&output_hash);
    data.extend_from_slice(&attestation_sig);
    data
}

fn anchor_discriminator(ix_name: &str) -> [u8; 8] {
    let preimage = format!("global:{ix_name}");
    let h = hash(preimage.as_bytes());
    let mut out = [0_u8; 8];
    out.copy_from_slice(&h.to_bytes()[..8]);
    out
}

fn parse_hex32(value: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(value).context("value must be hex")?;
    if bytes.len() != 32 {
        anyhow::bail!("value must decode to 32 bytes");
    }
    let mut out = [0_u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn default_policy_verify_pubkey_hex() -> String {
    let sk_bytes_vec = hex::decode(DEFAULT_POLICY_SIGNING_KEY_HEX)
        .expect("DEFAULT_POLICY_SIGNING_KEY_HEX must be valid hex");
    let sk_bytes: [u8; 32] = sk_bytes_vec
        .as_slice()
        .try_into()
        .expect("DEFAULT_POLICY_SIGNING_KEY_HEX must decode to 32 bytes");
    let signing = SigningKey::from_bytes(&sk_bytes);
    hex::encode(signing.verifying_key().as_bytes())
}

fn verify_assignment_policy(cfg: &WorkerConfig, assignment: &QueuedAssignment) -> Result<()> {
    let verifiers = cfg.policy_verifiers.read().expect("lock poisoned");
    let matched_verifier = verifiers
        .iter()
        .find(|v| v.key_id == assignment.policy_key_id && v.version == assignment.policy_version)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "assignment policy tuple ({},{}) not allowed by worker",
                assignment.policy_key_id,
                assignment.policy_version
            )
        })?;
    let now = now_unix_seconds();
    let earliest = assignment
        .policy_valid_after_unix_s
        .saturating_sub(cfg.policy_clock_skew_secs);
    let latest = assignment
        .policy_valid_until_unix_s
        .saturating_add(cfg.policy_clock_skew_secs);
    if now < earliest || now > latest {
        anyhow::bail!(
            "assignment policy window invalid at now={} (valid {}..{}, skew {})",
            now,
            assignment.policy_valid_after_unix_s,
            assignment.policy_valid_until_unix_s,
            cfg.policy_clock_skew_secs
        );
    }

    let expected_pk = {
        let bytes = hex::decode(matched_verifier.verify_pubkey_hex.trim())
            .context("policy verify key must be hex")?;
        let arr: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("policy verify key must be 32 bytes"))?;
        VerifyingKey::from_bytes(&arr).context("invalid policy verify key bytes")?
    };

    let signer_pk = {
        let bytes = hex::decode(assignment.policy_signer_pubkey.trim())
            .context("assignment signer pubkey must be hex")?;
        let arr: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("assignment signer pubkey must be 32 bytes"))?;
        VerifyingKey::from_bytes(&arr).context("invalid assignment signer pubkey bytes")?
    };

    if signer_pk != expected_pk {
        anyhow::bail!(
            "assignment signer key {} does not match configured verify key {}",
            assignment.policy_signer_pubkey,
            matched_verifier.verify_pubkey_hex
        );
    }

    let sig = {
        let bytes = hex::decode(assignment.policy_signature.trim())
            .context("assignment signature must be hex")?;
        let arr: [u8; 64] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("assignment signature must be 64 bytes"))?;
        Signature::from_bytes(&arr)
    };

    let message = assignment_policy_message(assignment);
    if !edgerun_crypto::verify(&signer_pk, message.as_bytes(), &sig) {
        anyhow::bail!("assignment signature verification failed");
    }
    Ok(())
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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

fn parse_runtime_id_hex(runtime_id: &str) -> Result<[u8; 32]> {
    let raw = runtime_id.trim();
    let hex_str = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    let bytes = hex::decode(hex_str).context("runtime_id must be hex")?;
    if bytes.len() != 32 {
        anyhow::bail!("runtime_id must decode to 32 bytes, got {}", bytes.len());
    }

    let mut out = [0_u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_types::control_plane::SubmissionAck;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn test_cfg(base_url: String) -> WorkerConfig {
        WorkerConfig {
            worker_pubkey: "worker-test".to_string(),
            scheduler_base_url: base_url,
            runtime_ids: vec![],
            version: "test".to_string(),
            capacity: WorkerCapacity {
                max_concurrent: 1,
                mem_bytes: 1024,
            },
            worker_signing_key: None,
            chain_submit: None,
            policy_verifiers: Arc::new(RwLock::new(vec![PolicyVerifier {
                key_id: default_policy_key_id(),
                version: default_policy_version(),
                verify_pubkey_hex: default_policy_verify_pubkey_hex(),
            }])),
            policy_session: Arc::new(Mutex::new(PolicySessionState {
                token: None,
                session_key: None,
                expires_at_unix_s: 0,
                bound_origin: None,
                bootstrap_token: None,
                nonce_counter: 0,
            })),
            policy_clock_skew_secs: 30,
            policy_refresh_interval: Duration::from_millis(50),
            pending_queue_max: 16,
            retry_base_ms: 5,
            retry_max_ms: 20,
            retry_flush_batch: 16,
        }
    }

    fn test_queue() -> SubmissionQueue {
        SubmissionQueue {
            items: VecDeque::new(),
            max_len: 16,
            base_backoff: Duration::from_millis(5),
            max_backoff: Duration::from_millis(20),
            flush_batch: 16,
        }
    }

    async fn spawn_control_ws_ok_server() -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mut ws = match tokio_tungstenite::accept_async(stream).await {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                while let Some(Ok(Message::Binary(bytes))) = ws.next().await {
                    let request: ControlWsClientMessage = match bincode::deserialize(&bytes) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let request_id = request.request_id.trim().to_string();
                    if request_id.is_empty() {
                        continue;
                    }
                    let data = match request.payload {
                        ControlWsRequestPayload::WorkerResult(_) => {
                            ControlWsResponsePayload::WorkerResult(SubmissionAck {
                                ok: true,
                                duplicate: false,
                                quorum_reached: Some(false),
                            })
                        }
                        ControlWsRequestPayload::WorkerFailure(_) => {
                            ControlWsResponsePayload::WorkerFailure(SubmissionAck {
                                ok: true,
                                duplicate: false,
                                quorum_reached: None,
                            })
                        }
                        ControlWsRequestPayload::WorkerReplay(_) => {
                            ControlWsResponsePayload::WorkerReplay(SubmissionAck {
                                ok: true,
                                duplicate: false,
                                quorum_reached: None,
                            })
                        }
                        _ => continue,
                    };
                    let response = ControlWsServerMessage {
                        request_id,
                        ok: true,
                        data: Some(data),
                        error: None,
                        status: None,
                    };
                    let Ok(encoded) = bincode::serialize(&response) else {
                        continue;
                    };
                    let _ = ws.send(Message::Binary(encoded.into())).await;
                }
            }
        });
        (format!("http://{addr}"), handle)
    }

    async fn spawn_policy_info_server(body: String) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let handle = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0_u8; 4096];
                let _ = stream.read(&mut buf).await;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });
        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn retries_result_submission_and_drains_after_recovery() {
        let mut queue = test_queue();

        let down_cfg = test_cfg("http://127.0.0.1:9".to_string());
        let payload = WorkerResultReport {
            idempotency_key: "ik-result-1".to_string(),
            worker_pubkey: "worker-test".to_string(),
            job_id: "job-test".to_string(),
            bundle_hash: "bundle-hash".to_string(),
            output_hash: "output-hash".to_string(),
            output_len: 7,
            attestation_sig: None,
            attestation_claim: None,
            signature: None,
        };

        let ok = submit_with_retry(
            &down_cfg,
            &mut queue,
            SubmissionKind::Result,
            payload.idempotency_key.clone(),
            &payload,
        )
        .await;
        assert!(!ok, "initial submission should fail and queue");
        assert_eq!(queue.items.len(), 1);

        let (base_url, server) = spawn_control_ws_ok_server().await;
        let up_cfg = test_cfg(base_url);
        flush_submission_queue(&up_cfg, &mut queue).await;
        assert!(
            queue.items.is_empty(),
            "queue should drain when endpoint recovers"
        );

        server.abort();
    }

    #[tokio::test]
    async fn queue_dedupes_same_idempotency_key_for_same_endpoint() {
        let mut queue = test_queue();
        enqueue_submission(
            &mut queue,
            SubmissionKind::Failure,
            "ik-failure-1".to_string(),
            ControlWsRequestPayload::WorkerFailure(WorkerFailureReport {
                idempotency_key: "ik-failure-1".to_string(),
                worker_pubkey: "worker-test".to_string(),
                job_id: "job-test".to_string(),
                bundle_hash: "bundle-hash".to_string(),
                phase: "exec".to_string(),
                error_code: "err".to_string(),
                error_message: "boom".to_string(),
                signature: None,
            }),
        );
        enqueue_submission(
            &mut queue,
            SubmissionKind::Failure,
            "ik-failure-1".to_string(),
            ControlWsRequestPayload::WorkerFailure(WorkerFailureReport {
                idempotency_key: "ik-failure-1".to_string(),
                worker_pubkey: "worker-test".to_string(),
                job_id: "job-test".to_string(),
                bundle_hash: "bundle-hash".to_string(),
                phase: "exec".to_string(),
                error_code: "err".to_string(),
                error_message: "boom".to_string(),
                signature: None,
            }),
        );
        assert_eq!(queue.items.len(), 1);

        enqueue_submission(
            &mut queue,
            SubmissionKind::Replay,
            "ik-failure-1".to_string(),
            ControlWsRequestPayload::WorkerReplay(WorkerReplayArtifactReport {
                idempotency_key: "ik-failure-1".to_string(),
                worker_pubkey: "worker-test".to_string(),
                job_id: "job-test".to_string(),
                artifact: ReplayArtifactPayload {
                    bundle_hash: "bundle-hash".to_string(),
                    ok: false,
                    abi_version: None,
                    runtime_id: Some("runtime-id".to_string()),
                    output_hash: Some("output-hash".to_string()),
                    output_len: Some(1),
                    input_len: Some(1),
                    max_memory_bytes: None,
                    max_instructions: None,
                    fuel_limit: None,
                    fuel_remaining: None,
                    error_code: Some("err".to_string()),
                    error_message: Some("boom".to_string()),
                    trap_code: None,
                },
                signature: None,
            }),
        );
        assert_eq!(
            queue.items.len(),
            2,
            "same key on different endpoints should be distinct"
        );
    }

    #[tokio::test]
    async fn refresh_policy_verifiers_adds_scheduler_tuple() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        let key_hex = default_policy_verify_pubkey_hex();
        let body = format!(
            "{{\"key_id\":\"rotated-key\",\"version\":2,\"signer_pubkey\":\"{}\",\"ttl_secs\":300}}",
            key_hex
        );
        let (base_url, server) = spawn_policy_info_server(body).await;
        let cfg = test_cfg(base_url);

        refresh_policy_verifiers(&client, &cfg)
            .await
            .expect("refresh should succeed");
        let verifiers = cfg.policy_verifiers.read().expect("lock poisoned");
        assert!(verifiers
            .iter()
            .any(|v| v.key_id == "rotated-key" && v.version == 2));

        server.abort();
    }

    #[tokio::test]
    async fn refresh_policy_verifiers_creates_session_after_unauthorized() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(200))
            .build()
            .expect("client");
        let key_hex = default_policy_verify_pubkey_hex();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        let handle = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0_u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                let (status, body) = if req.contains("post /v1/session/create ") {
                    (
                        "200 OK",
                        "{\"token\":\"tok-1\",\"session_key\":\"session-key-1\",\"ttl_secs\":300}"
                            .to_string(),
                    )
                } else if req.contains("get /v1/policy/info ")
                    && req.contains("authorization: bearer tok-1")
                {
                    (
                        "200 OK",
                        format!(
                            "{{\"key_id\":\"rotated-key\",\"version\":2,\"signer_pubkey\":\"{}\",\"ttl_secs\":300}}",
                            key_hex
                        ),
                    )
                } else {
                    (
                        "401 Unauthorized",
                        "{\"error\":\"unauthorized\"}".to_string(),
                    )
                };
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });
        let cfg = test_cfg(format!("http://{addr}"));

        refresh_policy_verifiers(&client, &cfg)
            .await
            .expect("refresh should create session and succeed");
        let verifiers = cfg.policy_verifiers.read().expect("lock poisoned");
        assert!(verifiers
            .iter()
            .any(|v| v.key_id == "rotated-key" && v.version == 2));

        handle.abort();
    }
}
