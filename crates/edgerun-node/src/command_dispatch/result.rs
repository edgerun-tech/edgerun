use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::{CommandRef, DelegationRef};
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// Command dispatch result
// ---------------------------------------------------------------------------

pub struct CommandDispatchResult {
    pub event_type: EventType,
    pub decision: i32, // COMMAND_DECISION_COMMITTED = 1, REJECTED = 2
    pub reason_code: String,
