//! Command validation and decision-event recording.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::NodeConfig;
use crate::bootstrap::{archive_node_genesis_payload, node_genesis_payload};
use edgerun_hardware_signing::MeshSigner;
use edgerun_protocols::core_protocol::collections::{HashMap, HashSet};
use edgerun_protocols::core_protocol::command::{
    CommandExecutionContext, CommandValidationContext, command_hash, validate_command,
};
use edgerun_protocols::core_protocol::protocol::{
    CommandDecision, CommandEnvelope, CommandType, EventType, ObjectKind, ObjectRef,
    command_envelope, enum_from_i32,
};
use edgerun_protocols::core_protocol::result::Verdict;
use edgerun_protocols::core_protocol::util::now_unix_millis_i64;
use edgerun_protocols::sign::ProtocolSigner;
use edgerun_storage::NodeStore;

use crate::protocol_signer::BorrowedMeshProtocolSigner;

#[derive(Clone, Debug, Default)]
pub struct ControllerSet {
    controllers: HashSet<Vec<u8>>,
}

impl ControllerSet {
    pub fn new(initial: Vec<Vec<u8>>) -> Self {
        Self {
            controllers: initial.into_iter().collect(),
        }
    }

    pub fn add(&mut self, identity_id: Vec<u8>) {
        self.controllers.insert(identity_id);
    }

    pub fn remove(&mut self, identity_id: &Vec<u8>) -> bool {
        self.controllers.remove(identity_id)
    }

    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }
}

pub struct CommandDispatchResult {
    pub event_type: EventType,
    pub decision: i32,
    pub reason_code: String,
    pub response_bytes: Vec<u8>,
}

pub use crate::stream_append::append_command_stream_event;

pub fn dispatch_command(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &HashSet<Vec<u8>>,
    trusted_root_ids: &[Vec<u8>],
    local_assurance_class: i32,
    exec_ctx: &CommandExecutionContext,
) -> CommandDispatchResult {
    let local_node_id = signer.node_id().0;
    dispatch_command_with_protocol_signer(
        command,
        store,
        stream_id,
        &local_node_id,
        &BorrowedMeshProtocolSigner::new(signer),
        controllers,
        replay_cache,
        revoked_delegations,
        trusted_root_ids,
        local_assurance_class,
        exec_ctx,
    )
}

pub fn dispatch_command_with_protocol_signer(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    local_node_id: &[u8; 64],
    signer: &(impl ProtocolSigner + ?Sized),
    controllers: &mut ControllerSet,
    replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &HashSet<Vec<u8>>,
    trusted_root_ids: &[Vec<u8>],
    local_assurance_class: i32,
    exec_ctx: &CommandExecutionContext,
) -> CommandDispatchResult {
    if command_is_duplicate(command, store, replay_cache) {
        return respond(command, store, stream_id, signer, true, "duplicate_command");
    }

    let empty_counts: HashMap<Vec<u8>, u64> = HashMap::new();
    let empty_rate_events: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();
    let location_classes: Vec<&str> = exec_ctx
        .location_classes
        .iter()
        .map(String::as_str)
        .collect();
    let ctx = CommandValidationContext {
        local_node_id,
        replay_cache,
        revoked_delegation_ids: revoked_delegations,
        delegation_use_counts: &empty_counts,
        delegation_rate_events_ms: &empty_rate_events,
        now_ms: now_unix_millis_i64(),
        trusted_root_ids,
        local_assurance_class,
        accepted_assurance_claims: &exec_ctx.accepted_assurance_claims,
        has_local_session: exec_ctx.has_local_session,
        has_user_presence: exec_ctx.has_user_presence,
        transport_class: exec_ctx
            .transport_class
            .as_deref()
            .and_then(transport_class),
        location_classes: &location_classes,
        target_stream_id: exec_ctx.target_stream_id.as_deref(),
        target_view_type: exec_ctx.target_view_type.as_deref(),
        target_domain: exec_ctx.target_domain.as_deref(),
        execution_class: exec_ctx
            .execution_class
            .as_deref()
            .and_then(execution_class),
        storage_class: exec_ctx.storage_class.as_deref().and_then(storage_class),
    };

    match validate_command(command, &ctx).verdict {
        Verdict::Reject => return respond(command, store, stream_id, signer, false, "rejected"),
        Verdict::Defer => return respond(command, store, stream_id, signer, false, "deferred"),
        Verdict::Duplicate => {
            return respond(command, store, stream_id, signer, true, "duplicate_command");
        }
        Verdict::Accept => {}
    }

    if let Err(reason) = validate_payload_boundary(command) {
        return respond(command, store, stream_id, signer, false, reason);
    }

    match command_type(command) {
        Some(CommandType::AddController) => {
            let id = extract_identity_from_command(command);
            if id.is_empty() {
                respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    false,
                    "missing_controller_identity",
                )
            } else {
                controllers.add(id);
                respond(command, store, stream_id, signer, true, "")
            }
        }
        Some(CommandType::RemoveController) => {
            let id = extract_identity_from_command(command);
            if id.is_empty() || !controllers.remove(&id) {
                respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    false,
                    "controller_not_found",
                )
            } else if controllers.to_vec().is_empty() {
                controllers.add(id);
                respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    false,
                    "cannot_remove_last_controller",
                )
            } else {
                respond(command, store, stream_id, signer, true, "")
            }
        }
        Some(CommandType::TransferControl) => {
            let id = extract_identity_from_command(command);
            if id.is_empty() {
                respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    false,
                    "missing_controller_identity",
                )
            } else {
                controllers.add(id);
                respond(command, store, stream_id, signer, true, "")
            }
        }
        Some(CommandType::Query) => respond(command, store, stream_id, signer, true, ""),
        Some(CommandType::PublishSnapshot) => respond(
            command,
            store,
            stream_id,
            signer,
            false,
            "use_produce_snapshot_request",
        ),
        Some(CommandType::FetchObject) => respond(
            command,
            store,
            stream_id,
            signer,
            false,
            "use_fetch_object_request",
        ),
        Some(CommandType::ExecuteWorkload) => respond(
            command,
            store,
            stream_id,
            signer,
            false,
            "execute_workload_not_supported",
        ),
        Some(CommandType::TerminateWorkload) => respond(
            command,
            store,
            stream_id,
            signer,
            false,
            "terminate_workload_not_supported",
        ),
        Some(_) => respond(
            command,
            store,
            stream_id,
            signer,
            false,
            "unsupported_command_type",
        ),
        None => respond(
            command,
            store,
            stream_id,
            signer,
            false,
            "unknown_command_type",
        ),
    }
}

fn validate_payload_boundary(command: &CommandEnvelope) -> Result<(), &'static str> {
    if matches!(
        command_type(command),
        Some(CommandType::StoreObject | CommandType::ExecuteWorkload)
    ) {
        if matches!(&command.payload, Some(command_envelope::Payload::InlinePayload(bytes)) if !bytes.is_empty())
        {
            return Err("PLAINTEXT_PAYLOAD_REJECTED");
        }
    }
    Ok(())
}

fn command_type(command: &CommandEnvelope) -> Option<CommandType> {
    enum_from_i32(command.command_type)
}

fn respond(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &(impl ProtocolSigner + ?Sized),
    committed: bool,
    reason_code: &str,
) -> CommandDispatchResult {
    match crate::command_dispatch_event::append_command_result_event_with_protocol_signer(
        command,
        store,
        stream_id,
        signer,
        committed,
        reason_code,
        None,
    ) {
        Ok(write) => CommandDispatchResult {
            event_type: write.event_type,
            decision: if committed {
                CommandDecision::Committed as i32
            } else {
                CommandDecision::Rejected as i32
            },
            reason_code: reason_code.to_string(),
            response_bytes: write.response_bytes,
        },
        Err(e) => CommandDispatchResult {
            event_type: EventType::CommandRejected,
            decision: CommandDecision::Rejected as i32,
            reason_code: format!("decision_event_write_failed: {e}"),
            response_bytes: Vec::new(),
        },
    }
}

fn command_is_duplicate(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    replay_cache: &HashMap<Vec<u8>, (Vec<u8>, i64)>,
) -> bool {
    let computed_hash = command_hash(command);
    if let Some(target) = command.target_node.as_ref() {
        let target_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(&target.node_id);
        let hash_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(&computed_hash.value);
        if matches!(store.get_replay_entry(&target_hex, &hash_hex), Ok(Some(_))) {
            return true;
        }
    }
    replay_cache.contains_key(&computed_hash.value)
}

fn transport_class(value: &str) -> Option<i32> {
    match value {
        "lan" => Some(1),
        "mesh" => Some(2),
        "internet" => Some(3),
        _ => None,
    }
}

fn execution_class(value: &str) -> Option<i32> {
    match value {
        "wasm" => Some(1),
        "native" => Some(2),
        "container" => Some(3),
        _ => None,
    }
}

fn storage_class(value: &str) -> Option<i32> {
    match value {
        "file" => Some(1),
        "memory" => Some(2),
        "object" => Some(3),
        _ => None,
    }
}

pub fn extract_identity_from_command(command: &CommandEnvelope) -> Vec<u8> {
    match &command.payload {
        Some(command_envelope::Payload::InlinePayload(bytes)) if !bytes.is_empty() => bytes.clone(),
        _ => command.command_id.clone(),
    }
}

pub fn record_command_sent_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &CommandEnvelope,
) {
    record_command_sent_event_with_protocol_signer(
        store,
        stream_id,
        &BorrowedMeshProtocolSigner::new(signer),
        command,
    );
}

pub fn record_command_sent_event_with_protocol_signer(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &(impl ProtocolSigner + ?Sized),
    command: &CommandEnvelope,
) {
    let _ = crate::stream_append::append_signed_stream_event_blocking_with_protocol_signer(
        store,
        stream_id,
        signer,
        edgerun_stream::EventDraft {
            event_type: EventType::CommandSent as i32,
            event_version: 1,
            related_commands: vec![crate::command_dispatch_result::command_ref_from(command)],
            ..Default::default()
        },
    );
}

pub fn project_controller_set(
    _store: &NodeStore,
    _stream_id: &[u8],
    initial: Vec<Vec<u8>>,
) -> ControllerSet {
    ControllerSet::new(initial)
}

pub fn project_config(
    _store: &NodeStore,
    _stream_id: &[u8],
    _base_text: &str,
) -> Result<NodeConfig, String> {
    Err("text node config was removed; project typed state from the event stream".into())
}

pub fn project_config_from_base(
    _store: &NodeStore,
    _stream_id: &[u8],
    base: NodeConfig,
) -> Result<NodeConfig, String> {
    Ok(base)
}

pub fn create_node_genesis_payload(
    store: &mut NodeStore,
    stream_id: &[u8],
    node_id: &edgerun_hardware_signing::NodeID,
    initial_controllers: &[Vec<u8>],
) -> ObjectRef {
    let payload = node_genesis_payload(node_id.0.to_vec(), initial_controllers.to_vec());
    let bytes = archive_node_genesis_payload(&payload);
    store
        .put_object(&bytes, ObjectKind::Payload as i32, &[stream_id.to_vec()])
        .unwrap_or_else(|_| ObjectRef {
            object_id: bytes,
            object_kind: Some(ObjectKind::Payload as i32),
        })
}
