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
