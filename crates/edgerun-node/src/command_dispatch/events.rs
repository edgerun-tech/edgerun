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

pub fn create_node_genesis_payload(
    store: &mut NodeStore,
    stream_id: &[u8],
    node_id: &edgerun_hardware_signing::NodeID,
    initial_controllers: &[Vec<u8>],
) -> edgerun_proto::edgerun::v0::common::ObjectRef {
    use edgerun_proto::edgerun::v0::stream::NodeGenesisPayload;
    use edgerun_proto::edgerun::v0::common::IdentityRef;

    let controllers: Vec<IdentityRef> = initial_controllers.iter().map(|id| IdentityRef {
        identity_id: id.clone(),
        identity_kind: Some(2), // NODE
        key_hint: None,
    }).collect();

    let payload = NodeGenesisPayload {
        payload_version: 1,
        node_id: node_id.0.to_vec(),
        primary_node_identity: Some(IdentityRef {
            identity_id: node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: None,
        }),
        initial_controllers: controllers,
        initial_policy_object: None,
        bootstrap_records: vec![],
        assurance_claims: vec![],
        node_roles: vec!["validator".to_string()],
        genesis_metadata: None,
    };

    let payload_bytes = prost::Message::encode_to_vec(&payload);
    store_object_or_log(store, &payload_bytes, 1, &[stream_id.to_vec()])
}

/// Appends a signed event to the node's stream and returns the event sequence number.
pub(crate) fn append_signed_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    event_type: EventType,
    event_version: u32,
    payload_object: Option<edgerun_proto::edgerun::v0::common::ObjectRef>,
    related_commands: Vec<edgerun_proto::edgerun::v0::common::CommandRef>,
    related_delegations: Vec<edgerun_proto::edgerun::v0::common::DelegationRef>,
) -> Option<u64> {
    let head_seq = store.get_head(stream_id).ok().flatten().map(|(s, _)| s).unwrap_or(-1);
    let mut event = EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq: (head_seq + 1) as u64,
        prev_event_hash: store.get_head(stream_id).ok().flatten().map(|(_, h)| Digest {
            algorithm: 1,
            value: h,
        }),
        event_type: event_type as i32,
        event_version,
        recorded_at: Some(edgerun_core::util::system_time_to_prost(SystemTime::now())),
        effective_at: None,
        payload_object,
        related_events: vec![],
        related_commands,
        related_objects: vec![],
        related_delegations,
        related_revocations: vec![],
        event_metadata: None,
        signature: None,
    };

    if let Err(e) = sign_event_envelope(&mut event, signer) {
        edgerun_log::warn!("failed to sign event: {}", e);
        return None;
    }
    if let Err(e) = store.append_event(&event) {
        edgerun_log::warn!("failed to append event: {}", e);
        return None;
    }
    Some(event.seq)
}

/// Creates a CommandSent payload and appends the event to the local stream.
/// Called when a command is issued to a peer (recorded in the sender's stream).
pub fn record_command_sent_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &CommandEnvelope,
) {
    use edgerun_proto::edgerun::v0::stream::CommandSentPayload;
    use edgerun_proto::edgerun::v0::common::CommandRef;

    let command_ref = command_ref_from(command);

    let payload = CommandSentPayload {
        payload_version: 1,
        command: Some(command_ref.clone()),
        target_node: command.target_node.clone(),
        send_metadata: None,
    };
    let payload_bytes = prost::Message::encode_to_vec(&payload);
    let object_ref = store_object_or_log(store, &payload_bytes, 1, &[stream_id.to_vec()]);

    let _seq = append_signed_event(
        store, stream_id, signer,
        EventType::CommandSent, 1,
        Some(object_ref),
        vec![command_ref],
        vec![],
    );
}

/// Creates an ActionStarted/Completed/Failed payload and appends the event.
pub fn record_action_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &CommandEnvelope,
    action_status: i32, // ACTION_STATUS_STARTED=1, COMPLETED=2, FAILED=3
    event_type: EventType,
) {
    use edgerun_proto::edgerun::v0::stream::ActionLifecyclePayload;

    let command_ref = command_ref_from(command);

    let action_id = format!("action-{}", edgerun_core::util::bytes_to_hex_prefixed(&command.command_id[..4.min(command.command_id.len())])).into_bytes();
    let payload = ActionLifecyclePayload {
        payload_version: 1,
        origin_command: Some(command_ref.clone()),
        action_instance_id: action_id,
        status: action_status,
        result_object: None,
        error_object: None,
        progress_object: None,
        action_metadata: None,
    };
    let payload_bytes = prost::Message::encode_to_vec(&payload);
    let object_ref = store_object_or_log(store, &payload_bytes, 1, &[stream_id.to_vec()]);

    let _seq = append_signed_event(
        store, stream_id, signer,
        event_type, 1,
        Some(object_ref),
        vec![command_ref],
        vec![],
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extracts an identity reference from the command's issuer identity.
/// Control commands identify the target controller by their identity_id.
fn extract_identity_from_command(command: &CommandEnvelope) -> Vec<u8> {
    // For control commands, the target identity is in the command's payload
    // as a raw identity_id. In v0, we extract it from the command_id field
    // which conventionally carries the target identity for control commands.
    if !command.command_id.is_empty() {
        return command.command_id.clone();
    }

    // Fallback: use issuer identity
    command.issuer.as_ref().map(|i| i.identity_id.clone()).unwrap_or_default()
}

/// Records a command result event and returns the response.
/// Emits the full action lifecycle: ActionStarted → CommandCommitted/Rejected → ActionCompleted/Failed.
/// If `controller_change` is Some, records the controller change in the database.
fn record_and_respond(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    _controllers: &ControllerSet,
    committed: bool,
    reason_code: &str,
    response_bytes: Vec<u8>,
    controller_change: Option<(&str, &str)>, // (controller_hex, change_type)
) -> CommandDispatchResult {
    let command_id_bytes = command.command_id.clone();
    let command_ref = CommandRef {
        command_id: command_id_bytes.clone(),
        command_hash: Some(command_hash(command)),
    };

    let delegations: Vec<edgerun_proto::edgerun::v0::common::DelegationRef> =
        command.delegation_chain.iter().map(|d| {
            edgerun_proto::edgerun::v0::common::DelegationRef {
                delegation_id: d.delegation_id.clone(),
                delegation_hash: Some(delegation_hash(d)),
            }
        }).collect();

    // 1. Emit ActionStarted for committed commands
    if committed {
        record_action_event(store, stream_id, signer, command,
            1, // ACTION_STATUS_STARTED
            EventType::ActionStarted);
    }

    // 2. Build CommandResultPayload and store as encrypted object
    let decision = if committed { 1 } else { 2 };
    let result_payload = ProtoCommandResultPayload {
        payload_version: 1,
        command: Some(command_ref.clone()),
        issuer: command.issuer.clone(),
        decision,
        decision_basis: None,
        reason_code: reason_code.to_string(),
        effect_summary_object: None,
        result_object: None,
    };
    let result_bytes = prost::Message::encode_to_vec(&result_payload);
    let object_ref = store_object_or_log(store, &result_bytes, 6, &[stream_id.to_vec()]);

    // 3. Emit CommandCommitted or CommandRejected event with payload
    let event_type = if committed {
        EventType::CommandCommitted
    } else {
        EventType::CommandRejected
    };
    let _event_seq = append_signed_event(
        store, stream_id, signer,
        event_type, 1,
        Some(object_ref),
        vec![command_ref],
        delegations,
    );

    // 4. Emit ActionCompleted or ActionFailed
    if committed {
        record_action_event(store, stream_id, signer, command,
            2, // ACTION_STATUS_COMPLETED
            EventType::ActionCompleted);
    } else {
        record_action_event(store, stream_id, signer, command,
            3, // ACTION_STATUS_FAILED
            EventType::ActionFailed);
    }

    // 5. Record controller change if this was a committed control command
    if committed {
        if let Some((controller_hex, change_type)) = controller_change {
            if let Err(e) = store.record_controller_change(controller_hex, change_type, 0) {
                edgerun_log::warn!("failed to record controller change: {}", e);
            }
        }
    }

    CommandDispatchResult {
        event_type,
        decision,
        reason_code: reason_code.to_string(),
        response_bytes,
    }
}

/// Sign an event envelope with the local node's key.
/// Shared utility used by both command_dispatch and main node logic.
pub(crate) fn sign_event_envelope(event: &mut EventEnvelope, signer: &dyn MeshSigner) -> Result<(), String> {
    use edgerun_core::crypto::SIG_DOMAIN_EVENT_ENVELOPE;
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let sig = signer.sign_record(SIG_DOMAIN_EVENT_ENVELOPE, &canonical)
        .map_err(|e| format!("signing failed: {}", e))?;
    event.signature = Some(edgerun_core::protocol::Signature {
        algorithm: 1,
        value: sig.to_vec(),
    });
    Ok(())
}

fn delegation_hash(delegation: &edgerun_proto::edgerun::v0::trust::DelegationRecord) -> Digest {
    let mut signable = delegation.clone();
    signable.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable, &mut canonical)
        .expect("prost encode failed for delegation");
    Digest {
        algorithm: 1,
        value: edgerun_core::crypto::sha256(&canonical).to_vec(),
    }
}

/// Construct workload temporary directory paths from a work_id.
#[cfg(feature = "oci")]
#[derive(Clone)]
struct WorkloadPaths {
    tmp_base: std::path::PathBuf,
    bundle_dir: std::path::PathBuf,
    store_dir: std::path::PathBuf,
    cgroup_path: String,
    thread_name: String,
}

#[cfg(feature = "oci")]
impl WorkloadPaths {
    fn new(work_id: &[u8]) -> Self {
        let hex8 = edgerun_core::util::bytes_to_hex(&work_id[..8.min(work_id.len())]);
        let hex4 = edgerun_core::util::bytes_to_hex(&work_id[..4.min(work_id.len())]);
        let tmp_base = std::env::temp_dir().join(format!("eg_wk_{}_{}", hex8, std::process::id()));
        Self {
            tmp_base: tmp_base.clone(),
            bundle_dir: tmp_base.join("bundle"),
            store_dir: tmp_base.join("store"),
            cgroup_path: format!("/edgerun/eg_{}", hex8),
            thread_name: format!("work-{}", hex4),
        }
    }
}

/// Build a CommandRef from a command envelope.
fn command_ref_from(command: &CommandEnvelope) -> CommandRef {
    CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    }
}

/// Build DelegationRef vector from a command's delegation chain.
fn delegation_refs_from(command: &CommandEnvelope) -> Vec<edgerun_proto::edgerun::v0::common::DelegationRef> {
    command.delegation_chain.iter().map(|d| {
        edgerun_proto::edgerun::v0::common::DelegationRef {
            delegation_id: d.delegation_id.clone(),
            delegation_hash: Some(delegation_hash(d)),
        }
    }).collect()
}

/// Store an object in the node store, logging warnings on failure.
/// Returns an empty ObjectRef if storage fails.
fn store_object_or_log(
    store: &mut NodeStore,
    data: &[u8],
    kind: i32,
    stream_tags: &[Vec<u8>],
) -> edgerun_proto::edgerun::v0::common::ObjectRef {
    match store.put_object(data, kind, stream_tags) {
        Ok(r) => r,
        Err(e) => {
            edgerun_log::warn!("failed to store object (kind={}): {}", kind, e);
            edgerun_proto::edgerun::v0::common::ObjectRef {
                object_id: vec![],
                object_kind: Some(kind),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

