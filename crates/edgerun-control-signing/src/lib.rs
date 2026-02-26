// SPDX-License-Identifier: Apache-2.0
use edgerun_types::control_plane::{
    HeartbeatRequest, WorkerAssignmentsRequest, WorkerFailureReport, WorkerReplayArtifactReport,
    WorkerResultReport,
};

pub fn heartbeat_signing_message(payload: &HeartbeatRequest) -> String {
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

pub fn assignments_signing_message(payload: &WorkerAssignmentsRequest) -> String {
    format!(
        "assignments|{}|{}",
        payload.worker_pubkey, payload.signed_at_unix_s
    )
}

pub fn result_signing_message(payload: &WorkerResultReport) -> String {
    let attestation_sig = payload.attestation_sig.clone().unwrap_or_default();
    let attestation_claim = payload
        .attestation_claim
        .as_ref()
        .map(canonical_attestation_claim)
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

fn canonical_attestation_claim(claim: &edgerun_types::AttestationClaim) -> String {
    format!(
        "measurement={};issued_at_unix_s={};expires_at_unix_s={};nonce={};format={};evidence={}",
        claim.measurement,
        claim.issued_at_unix_s,
        claim.expires_at_unix_s,
        claim.nonce.as_deref().unwrap_or_default(),
        claim.format.as_deref().unwrap_or_default(),
        claim.evidence.as_deref().unwrap_or_default()
    )
}

pub fn failure_signing_message(payload: &WorkerFailureReport) -> String {
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

pub fn replay_signing_message(payload: &WorkerReplayArtifactReport) -> String {
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

pub fn route_register_signing_message(
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
