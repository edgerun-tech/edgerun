// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

use crate::{AttestationClaim, AttestationPolicy, Limits, SyncTrustPolicy, BUNDLE_ABI_CURRENT};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInfoResponse {
    pub key_id: String,
    pub version: u32,
    pub signer_pubkey: String,
    pub ttl_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_policy: Option<SyncTrustPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_policy: Option<AttestationPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_origin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCreateResponse {
    pub token: String,
    pub session_key: String,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCapacity {
    pub max_concurrent: u32,
    pub mem_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub worker_pubkey: String,
    pub runtime_ids: Vec<String>,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<WorkerCapacity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub next_poll_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_time_unix_s: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentsResponse {
    pub jobs: Vec<QueuedAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedAssignment {
    pub job_id: String,
    pub bundle_hash: String,
    pub bundle_url: String,
    pub runtime_id: String,
    #[serde(default = "default_abi_version")]
    pub abi_version: u8,
    pub limits: Limits,
    pub escrow_lamports: u64,
    #[serde(default)]
    pub policy_signer_pubkey: String,
    #[serde(default)]
    pub policy_signature: String,
    #[serde(default = "default_policy_key_id")]
    pub policy_key_id: String,
    #[serde(default = "default_policy_version")]
    pub policy_version: u32,
    #[serde(default)]
    pub policy_valid_after_unix_s: u64,
    #[serde(default)]
    pub policy_valid_until_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResultReport {
    #[serde(default)]
    pub idempotency_key: String,
    pub worker_pubkey: String,
    pub job_id: String,
    pub bundle_hash: String,
    pub output_hash: String,
    pub output_len: usize,
    #[serde(default)]
    pub attestation_sig: Option<String>,
    #[serde(default)]
    pub attestation_claim: Option<AttestationClaim>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerFailureReport {
    #[serde(default)]
    pub idempotency_key: String,
    pub worker_pubkey: String,
    pub job_id: String,
    pub bundle_hash: String,
    pub phase: String,
    pub error_code: String,
    pub error_message: String,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayArtifactPayload {
    pub bundle_hash: String,
    pub ok: bool,
    pub abi_version: Option<u8>,
    pub runtime_id: Option<String>,
    pub output_hash: Option<String>,
    pub output_len: Option<usize>,
    pub input_len: Option<usize>,
    pub max_memory_bytes: Option<u32>,
    pub max_instructions: Option<u64>,
    pub fuel_limit: Option<u64>,
    pub fuel_remaining: Option<u64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub trap_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerReplayArtifactReport {
    #[serde(default)]
    pub idempotency_key: String,
    pub worker_pubkey: String,
    pub job_id: String,
    pub artifact: ReplayArtifactPayload,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreateRequest {
    pub runtime_id: String,
    pub wasm_base64: String,
    pub input_base64: String,
    pub abi_version: Option<u8>,
    pub limits: Limits,
    pub escrow_lamports: u64,
    pub assignment_worker_pubkey: Option<String>,
    pub client_pubkey: Option<String>,
    pub client_signed_at_unix_s: Option<u64>,
    pub client_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCreateResponse {
    pub job_id: String,
    pub bundle_hash: String,
    pub bundle_url: String,
    pub post_job_tx: String,
    pub post_job_sig: Option<String>,
    pub assign_workers_tx: Option<String>,
    pub assign_workers_sig: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub reports: Vec<WorkerResultReport>,
    pub failures: Vec<WorkerFailureReport>,
    pub replay_artifacts: Vec<WorkerReplayArtifactReport>,
    pub quorum: Option<JobQuorumState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobQuorumState {
    #[serde(default)]
    pub expected_bundle_hash: String,
    #[serde(default)]
    pub expected_runtime_id: String,
    pub committee_workers: Vec<String>,
    pub committee_size: u8,
    pub quorum: u8,
    #[serde(default)]
    pub assign_tx: Option<String>,
    #[serde(default)]
    pub assign_sig: Option<String>,
    #[serde(default)]
    pub assign_submitted: bool,
    pub quorum_reached: bool,
    pub winning_output_hash: Option<String>,
    pub winning_workers: Vec<String>,
    #[serde(default)]
    pub finalize_triggered: bool,
    pub finalize_tx: Option<String>,
    pub finalize_sig: Option<String>,
    #[serde(default)]
    pub finalize_submitted: bool,
    #[serde(default)]
    pub cancel_triggered: bool,
    #[serde(default)]
    pub cancel_tx: Option<String>,
    #[serde(default)]
    pub cancel_sig: Option<String>,
    #[serde(default)]
    pub cancel_submitted: bool,
    #[serde(default)]
    pub onchain_status: Option<String>,
    #[serde(default)]
    pub onchain_last_observed_slot: Option<u64>,
    #[serde(default)]
    pub onchain_last_update_unix_s: Option<u64>,
    #[serde(default)]
    pub onchain_deadline_slot: Option<u64>,
    #[serde(default)]
    pub slash_artifacts: Vec<SlashWorkerArtifact>,
    pub created_at_unix_s: u64,
    pub quorum_reached_at_unix_s: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashWorkerArtifact {
    pub worker_pubkey: String,
    pub tx: String,
    pub sig: Option<String>,
    pub submitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResolveRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteOwnerRequest {
    pub owner_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteChallengeRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteChallengeResponse {
    pub nonce: String,
    pub expires_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRegisterRequest {
    pub device_id: String,
    pub owner_pubkey: String,
    pub reachable_urls: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    // Keep explicit field position for bincode control-ws framing.
    // Skipping this field when None misaligns following fields for non-self-describing formats.
    #[serde(default)]
    pub relay_session_id: Option<String>,
    pub ttl_secs: u64,
    pub challenge_nonce: String,
    pub signed_at_unix_s: u64,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRegisterResponse {
    pub ok: bool,
    pub heartbeat_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHeartbeatRequest {
    pub device_id: String,
    pub token: String,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHeartbeatResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResolveEntry {
    pub device_id: String,
    pub owner_pubkey: String,
    pub reachable_urls: Vec<String>,
    pub capabilities: Vec<String>,
    pub relay_session_id: Option<String>,
    pub online: bool,
    pub last_seen_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub updated_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResolveResponse {
    pub ok: bool,
    pub found: bool,
    pub route: Option<RouteResolveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerRoutesResponse {
    pub ok: bool,
    pub owner_pubkey: String,
    pub devices: Vec<RouteResolveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerAssignmentsRequest {
    pub worker_pubkey: String,
    #[serde(default)]
    pub signed_at_unix_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleGetRequest {
    pub bundle_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleGetResponse {
    pub ok: bool,
    pub bundle_hash: String,
    pub payload_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionAck {
    pub ok: bool,
    pub duplicate: bool,
    pub quorum_reached: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlWsRequestPayload {
    JobCreate(JobCreateRequest),
    JobStatus(JobStatusRequest),
    RouteChallenge(RouteChallengeRequest),
    RouteRegister(RouteRegisterRequest),
    RouteHeartbeat(RouteHeartbeatRequest),
    RouteResolve(RouteResolveRequest),
    RouteOwner(RouteOwnerRequest),
    WorkerHeartbeat(HeartbeatRequest),
    WorkerAssignments(WorkerAssignmentsRequest),
    WorkerResult(WorkerResultReport),
    WorkerFailure(WorkerFailureReport),
    WorkerReplay(WorkerReplayArtifactReport),
    BundleGet(BundleGetRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlWsClientMessage {
    pub request_id: String,
    pub payload: ControlWsRequestPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlWsResponsePayload {
    JobCreate(JobCreateResponse),
    JobStatus(Box<JobStatusResponse>),
    RouteChallenge(RouteChallengeResponse),
    RouteRegister(RouteRegisterResponse),
    RouteHeartbeat(RouteHeartbeatResponse),
    RouteResolve(RouteResolveResponse),
    RouteOwner(Box<OwnerRoutesResponse>),
    WorkerHeartbeat(HeartbeatResponse),
    WorkerAssignments(Box<AssignmentsResponse>),
    WorkerResult(SubmissionAck),
    WorkerFailure(SubmissionAck),
    WorkerReplay(SubmissionAck),
    BundleGet(BundleGetResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlWsServerMessage {
    pub request_id: String,
    pub ok: bool,
    pub data: Option<ControlWsResponsePayload>,
    pub error: Option<String>,
    pub status: Option<u16>,
}

pub fn default_abi_version() -> u8 {
    BUNDLE_ABI_CURRENT
}

pub fn default_policy_key_id() -> String {
    "dev-key-1".to_string()
}

pub fn default_policy_version() -> u32 {
    1
}

pub fn assignment_policy_message(assignment: &QueuedAssignment) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_ws_route_register_roundtrip_with_none_relay_session_id() {
        let msg = ControlWsClientMessage {
            request_id: "term-route-2".to_string(),
            payload: ControlWsRequestPayload::RouteRegister(RouteRegisterRequest {
                device_id: "device-1".to_string(),
                owner_pubkey: "owner-1".to_string(),
                reachable_urls: vec![
                    "http://127.0.0.1:5577".to_string(),
                    "http://172.245.67.49:5577".to_string(),
                ],
                capabilities: vec!["terminal-ws".to_string(), "webrtc-datachannel".to_string()],
                relay_session_id: None,
                ttl_secs: 90,
                challenge_nonce: "nonce-1".to_string(),
                signed_at_unix_s: 1_700_000_000,
                signature: "sig-1".to_string(),
            }),
        };
        let encoded = bincode::serialize(&msg).expect("serialize");
        let decoded: ControlWsClientMessage = bincode::deserialize(&encoded).expect("deserialize");
        match decoded.payload {
            ControlWsRequestPayload::RouteRegister(payload) => {
                assert_eq!(payload.ttl_secs, 90);
                assert_eq!(payload.relay_session_id, None);
                assert_eq!(payload.reachable_urls.len(), 2);
            }
            other => panic!("unexpected payload variant: {other:?}"),
        }
    }
}
