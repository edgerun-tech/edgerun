// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};

use crate::control_plane::{
    HeartbeatRequest, RouteHeartbeatRequest, RouteRegisterRequest, WorkerAssignmentsRequest,
    WorkerFailureReport, WorkerReplayArtifactReport, WorkerResultReport,
};

pub const EVENT_SCHEMA_VERSION_V1: u32 = 1;

pub const EVENT_PAYLOAD_TYPE_WORKER_LIFECYCLE_START: &str = "worker_lifecycle_start";
pub const EVENT_PAYLOAD_TYPE_WORKER_HEARTBEAT_STATUS: &str = "worker_heartbeat_status";
pub const EVENT_PAYLOAD_TYPE_TERM_SERVER_LIFECYCLE_START: &str = "term_server_lifecycle_start";
pub const EVENT_PAYLOAD_TYPE_TERM_SERVER_ROUTE_ANNOUNCER: &str = "term_server_route_announcer";
pub const EVENT_PAYLOAD_TYPE_WORKER_HEARTBEAT: &str = "worker_heartbeat";
pub const EVENT_PAYLOAD_TYPE_WORKER_ASSIGNMENTS_POLL: &str = "worker_assignments_poll";
pub const EVENT_PAYLOAD_TYPE_WORKER_RESULT: &str = "worker_result";
pub const EVENT_PAYLOAD_TYPE_WORKER_FAILURE: &str = "worker_failure";
pub const EVENT_PAYLOAD_TYPE_WORKER_REPLAY: &str = "worker_replay";
pub const EVENT_PAYLOAD_TYPE_ROUTE_CHALLENGE: &str = "route_challenge";
pub const EVENT_PAYLOAD_TYPE_ROUTE_REGISTER: &str = "route_register";
pub const EVENT_PAYLOAD_TYPE_ROUTE_HEARTBEAT: &str = "route_heartbeat";

pub const EVENT_TOPIC_WORKER_LIFECYCLE_START: &str = "worker.lifecycle.start";
pub const EVENT_TOPIC_WORKER_HEARTBEAT: &str = "worker.heartbeat";
pub const EVENT_TOPIC_TERM_SERVER_LIFECYCLE_START: &str = "term_server.lifecycle.start";
pub const EVENT_TOPIC_TERM_SERVER_ROUTE_ANNOUNCER: &str = "term_server.route.announcer";
pub const EVENT_TOPIC_SCHEDULER_WORKER_HEARTBEAT: &str = "scheduler.worker.heartbeat";
pub const EVENT_TOPIC_SCHEDULER_WORKER_ASSIGNMENTS_POLL: &str = "scheduler.worker.assignments_poll";
pub const EVENT_TOPIC_SCHEDULER_WORKER_RESULT: &str = "scheduler.worker.result";
pub const EVENT_TOPIC_SCHEDULER_WORKER_FAILURE: &str = "scheduler.worker.failure";
pub const EVENT_TOPIC_SCHEDULER_WORKER_REPLAY: &str = "scheduler.worker.replay";
pub const EVENT_TOPIC_SCHEDULER_ROUTE_CHALLENGE: &str = "scheduler.route.challenge";
pub const EVENT_TOPIC_SCHEDULER_ROUTE_REGISTER: &str = "scheduler.route.register";
pub const EVENT_TOPIC_SCHEDULER_ROUTE_HEARTBEAT: &str = "scheduler.route.heartbeat";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentPipelineStage {
    IngressReceived,
    IntentNormalized,
    PolicyEvaluated,
    CapabilityPlanned,
    CapabilityLeased,
    ExecutionRouted,
    ExecutionRunning,
    ExecutionCompleted,
    ExecutionFailed,
    PolicyDenied,
    ExecutionCanceled,
    ExecutionTimedOut,
}

impl IntentPipelineStage {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::ExecutionCompleted
                | Self::ExecutionFailed
                | Self::PolicyDenied
                | Self::ExecutionCanceled
                | Self::ExecutionTimedOut
        )
    }

    pub fn is_start(self) -> bool {
        matches!(self, Self::IngressReceived)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentEnvelope {
    pub intent_id: String,
    pub node_id: String,
    pub issuer: String,
    pub action: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub stage: IntentPipelineStage,
    pub ts_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntentDecision {
    pub intent_id: String,
    pub stage: IntentPipelineStage,
    pub allowed: bool,
    pub reason: String,
    pub policy_id: String,
    pub ts_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityLease {
    pub lease_id: String,
    pub intent_id: String,
    pub node_id: String,
    pub component_id: String,
    pub capability: String,
    pub scope: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepAssignment {
    pub assignment_id: String,
    pub intent_id: String,
    pub step_id: String,
    pub node_id: String,
    pub executor_kind: String,
    #[serde(default)]
    pub required_leases: Vec<String>,
    pub stage: IntentPipelineStage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    pub receipt_id: String,
    pub intent_id: String,
    pub step_id: String,
    pub node_id: String,
    pub executor_kind: String,
    pub status: IntentPipelineStage,
    pub output_hash: String,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerLifecycleEvent {
    pub schema_version: u32,
    pub worker_pubkey: String,
    pub runtime_ids: Vec<String>,
    pub version: String,
    pub scheduler_base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerHeartbeatEvent {
    pub schema_version: u32,
    pub worker_pubkey: String,
    pub scheduler_ok: bool,
    pub next_poll_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermServerLifecycleEvent {
    pub schema_version: u32,
    pub backend: String,
    pub device_pubkey_b64url: String,
    pub bind_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteAnnouncerEvent {
    pub schema_version: u32,
    pub device_id: String,
    pub owner_pubkey: String,
    pub phase: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerHeartbeatIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: HeartbeatRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerAssignmentsPollIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: WorkerAssignmentsRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResultIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: WorkerResultReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerFailureIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: WorkerFailureReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerReplayIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: WorkerReplayArtifactReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteChallengeIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub device_id: String,
    pub nonce: String,
    pub expires_at_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRegisterIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: RouteRegisterRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteHeartbeatIngestEvent {
    pub schema_version: u32,
    pub observed_at_unix_ms: u64,
    pub payload: RouteHeartbeatRequest,
}

#[cfg(test)]
mod tests {
    use super::{IntentPipelineStage, WorkerHeartbeatEvent, EVENT_SCHEMA_VERSION_V1};

    #[test]
    fn terminal_stages_are_explicit() {
        assert!(IntentPipelineStage::ExecutionCompleted.is_terminal());
        assert!(IntentPipelineStage::ExecutionFailed.is_terminal());
        assert!(IntentPipelineStage::PolicyDenied.is_terminal());
        assert!(IntentPipelineStage::ExecutionCanceled.is_terminal());
        assert!(IntentPipelineStage::ExecutionTimedOut.is_terminal());
        assert!(!IntentPipelineStage::ExecutionRunning.is_terminal());
    }

    #[test]
    fn start_stage_is_explicit() {
        assert!(IntentPipelineStage::IngressReceived.is_start());
        assert!(!IntentPipelineStage::IntentNormalized.is_start());
    }

    #[test]
    fn worker_heartbeat_event_roundtrip_bincode() {
        let payload = WorkerHeartbeatEvent {
            schema_version: EVENT_SCHEMA_VERSION_V1,
            worker_pubkey: "worker-1".to_string(),
            scheduler_ok: true,
            next_poll_ms: 500,
        };
        let encoded = bincode::serialize(&payload).expect("serialize");
        let decoded: WorkerHeartbeatEvent = bincode::deserialize(&encoded).expect("deserialize");
        assert_eq!(decoded.worker_pubkey, payload.worker_pubkey);
        assert_eq!(decoded.scheduler_ok, payload.scheduler_ok);
        assert_eq!(decoded.next_poll_ms, payload.next_poll_ms);
    }
}
