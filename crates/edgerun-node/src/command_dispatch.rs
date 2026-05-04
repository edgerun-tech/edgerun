//! Command dispatch for the edgerun node daemon.
//!
//! Replaces the old signature-only validation with:
//! - Full command validation (replay, timing, delegation signature verification)
//! - Command type dispatch (ADD_CONTROLLER, REMOVE_CONTROLLER, TRANSFER_CONTROL, etc.)
//! - Controller set management and projection from event log
//! - Revocation record processing

// Handler modules — split from monolithic command_dispatch.rs
pub mod command_handlers;

// Re-export key types for handlers
pub use command_handlers::{
    control, custom, apps, identity, infrastructure, user, config,
};

use crate::command_authority::{build_validation_context, check_replay_cache, CommandGateDecision};
use crate::command_dispatch_payload::extract_payload_or_reject;
use crate::config::{parse_config, NodeConfig};
use edgerun_core::collections::{HashMap, HashSet};
use edgerun_core::command::{
    command_hash, validate_command, CommandExecutionContext, CommandValidationContext,
};
use edgerun_core::encrypted_envelope::validate_encrypted_envelope;
use edgerun_proto::edgerun::v0::common::EncryptedEnvelope;
use edgerun_core::protocol::{canonical_bytes, Digest, EventEnvelope, ProtocolRecord};
use edgerun_core::result::Verdict;
use edgerun_core::util::{
    now_prost_timestamp, now_unix_micros_u64, now_unix_millis_i64, now_unix_secs_i64,
};
use edgerun_core::validators_proto::{
    validate_control_change_command, validate_delegation_chain, validate_revocation_record,
};
use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::MeshSigner;
use edgerun_json::Value as JsonValue;
use edgerun_proto::edgerun::v0::common::{CommandRef, EventRef, ObjectRef};
use edgerun_proto::edgerun::v0::stream::{
    CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload,
    CommandType, EventType,
};
use edgerun_proto::edgerun::v0::trust::{
    DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord,
};
use edgerun_storage::NodeStore;
use prost::Message;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Controller state
// ---------------------------------------------------------------------------

/// The current set of controller identities authorized to influence this node.
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

    pub fn contains(&self, identity_id: &Vec<u8>) -> bool {
        self.controllers.contains(identity_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.controllers.iter()
    }

    pub fn controller_ids(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }

    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Command dispatch result
// ---------------------------------------------------------------------------

pub struct CommandDispatchResult {
    pub event_type: EventType,
    pub decision: i32, // COMMAND_DECISION_COMMITTED = 1, REJECTED = 2
    pub reason_code: String,
    pub response_bytes: Vec<u8>,
}

// Removed dead signature verification functions (verify_command_signature, verify_delegation_signature).
// These used a single-hash signing path inconsistent with spec §17 (double-hash).
// All signature verification is now handled by validate_command() via verify_canonical_record().

// ---------------------------------------------------------------------------
// Controller projection from event log
// ---------------------------------------------------------------------------

/// Projects the current controller set from the genesis event + all command events.
///
/// Walks the event log from seq 0, processing:
/// - NodeGenesis: sets initial controllers
/// - CommandCommitted with ADD_CONTROLLER: adds controller
/// Replays controller changes from the persistent change log to rebuild the ControllerSet.
///
/// Delegates to `NodeStore::project_controller_set()` which reads from the
/// FileIndex's `controller_changes.bin` — an append-only log of all controller
/// mutations recorded during command processing.
///
/// This ensures controller state survives node restarts.
pub fn project_controller_set(
    store: &NodeStore,
    _stream_id: &[u8],
    initial_controllers: Vec<Vec<u8>>,
) -> ControllerSet {
    let head_seq = match store.get_head(_stream_id) {
        Ok(Some((seq, _))) => seq,
        _ => return ControllerSet::new(initial_controllers),
    };

    match store.project_controller_set(initial_controllers, head_seq) {
        Ok(set) => ControllerSet::new(set.to_vec()),
        Err(e) => {
            edgerun_log::warn!("failed to project controller set: {}", e);
            ControllerSet::new(Vec::new())
        }
    }
}

/// Projects node configuration from the event store.
///
/// This replays UpdateConfig commands from the event log and applies them
/// sequentially to build the current configuration state.
///
/// On first boot (no events), returns the initial config from the YAML file.
pub fn project_config(
    store: &NodeStore,
    stream_id: &[u8],
    initial_config_yaml: &str,
) -> Result<NodeConfig, String> {
    let config = parse_config(initial_config_yaml).map_err(|e| e.to_string())?;
    project_config_from_base(store, stream_id, config)
}

pub fn project_config_from_base(
    store: &NodeStore,
    stream_id: &[u8],
    mut config: NodeConfig,
) -> Result<NodeConfig, String> {
    let head_seq = match store.get_head(stream_id) {
        Ok(Some((seq, _))) => seq,
        Ok(None) => {
            // No events yet, use initial YAML config
            return Ok(config);
        }
        Err(e) => return Err(format!("failed to get head: {}", e)),
    };

    if head_seq == 0 {
        return Ok(config);
    }

    let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
    let events = store
        .list_event_range(&stream_id_hex, 1, head_seq)
        .map_err(|e| format!("failed to list config projection events: {e}"))?;

    for (seq, _hash, _ver) in events {
        let Some((event, Some(payload_bytes))) = store
            .get_event_with_payload(stream_id, seq as u64)
            .map_err(|e| format!("failed to load event {seq}: {e}"))?
        else {
            continue;
        };
        if event.event_type != EventType::CommandCommitted as i32 {
            continue;
        }

        let Ok(result) = ProtoCommandResultPayload::decode(&payload_bytes[..]) else {
            continue;
        };
        let Some(result_object) = result.result_object.as_ref() else {
            continue;
        };
        let Some(patch) = store
            .get_object(result_object)
            .map_err(|e| format!("failed to load config patch object at event {seq}: {e}"))?
        else {
            continue;
        };
        if try_apply_projected_config_patch(&mut config, &patch.content)
            .map_err(|e| format!("invalid committed config patch at event {seq}: {e}"))?
        {
            edgerun_log::info!("applied projected config patch from event {}", seq);
        }
    }

    Ok(config)
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Processes a command through full validation and type-specific dispatch.
///
/// `workload_policy` and `rate_limiter` are projected from the immutable
/// event log by the caller — they are NOT loaded from mutable files here.
///
/// `running_workloads` is a shared registry for tracking active containers
/// so they can be preempted.
///
/// `local_assurance_class` is the node's assurance capability
/// (ASSURANCE_CLASS_SOFTWARE=1, HARDWARE_BACKED=2, ATTESTED_RUNTIME=3).
///
/// Returns the response bytes to send back to the caller.
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
    let now_ms = now_unix_millis_i64();

    let local_node_id = signer.node_id().0;

    // Check persistent replay cache first (survives restarts, spec §19.10)
    let computed_hash = command_hash(command);
    let cmd_hash_hex = edgerun_core::util::bytes_to_hex(&computed_hash.value);
    if let Some(ref target) = command.target_node {
        let target_hex = edgerun_core::util::bytes_to_hex(&target.node_id);
        if let Ok(Some((_cmd_id, _event_seq))) = store.get_replay_entry(&target_hex, &cmd_hash_hex)
        {
            // Previously processed command — return DUPLICATE without re-execution
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                true,
                "duplicate_command",
                Vec::new(),
                None,
            );
        }
    }

    // Also check in-memory replay cache for this session's commands
    if replay_cache.contains_key(&computed_hash.value) {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            true,
            "duplicate_command",
            Vec::new(),
            None,
        );
    }

    // Build validation context from execution context + node-local state
    let delegation_use_counts: HashMap<Vec<u8>, u64> = HashMap::new();
    let delegation_rate_events_ms: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();
    let ctx = CommandValidationContext {
        local_node_id: &local_node_id,
        replay_cache,
        revoked_delegation_ids: revoked_delegations,
        delegation_use_counts: &delegation_use_counts,
        delegation_rate_events_ms: &delegation_rate_events_ms,
        now_ms,
        trusted_root_ids,
        local_assurance_class,
        accepted_assurance_claims: &exec_ctx.accepted_assurance_claims,
        has_local_session: exec_ctx.has_local_session,
        has_user_presence: exec_ctx.has_user_presence,
        transport_class: exec_ctx.transport_class.as_deref().and_then(|s| {
            match s as &str {
                "lan" => Some(1),      // TRANSPORT_CLASS_LAN
                "mesh" => Some(2),     // TRANSPORT_CLASS_MESH
                "internet" => Some(3), // TRANSPORT_CLASS_INTERNET
                _ => None,
            }
        }),
        location_classes: &exec_ctx.location_classes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        target_stream_id: exec_ctx.target_stream_id.as_deref(),
        target_view_type: exec_ctx.target_view_type.as_deref(),
        target_domain: exec_ctx.target_domain.as_deref(),
        execution_class: exec_ctx.execution_class.as_deref().and_then(|s| {
            match s as &str {
                "wasm" => Some(1),     // EXECUTION_CLASS_WASM
                "native" => Some(2),   // EXECUTION_CLASS_NATIVE
                "container" => Some(3), // EXECUTION_CLASS_CONTAINER
                _ => None,
            }
        }),
        storage_class: exec_ctx.storage_class.as_deref().and_then(|s| {
            match s as &str {
                "file" => Some(1),     // STORAGE_CLASS_FILE
                "memory" => Some(2),   // STORAGE_CLASS_MEMORY
                "object" => Some(3),   // STORAGE_CLASS_OBJECT
                _ => None,
            }
        }),
    };

    // Run full validation (replay, timing, delegation chain, cryptographic signatures)
    // validate_command already verifies both command and delegation chain signatures,
    // so we don't need separate verify_command_signature/verify_delegation_signature calls.
    let validation_result = validate_command(command, &ctx);

    // If validation rejected or deferred, record and return
    match validation_result.verdict {
        Verdict::Reject => {
            let reason = validation_result
                .reason_code
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "rejected".into());
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &reason,
                Vec::new(),
                None,
            );
        }
        Verdict::Defer => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "deferred",
                Vec::new(),
                None,
            );
        }
        Verdict::Duplicate => {
            // Already processed — return prior acknowledgment per spec §19.10.
            // Same command_hash means the command was already committed/rejected.
            // No re-execution of side effects.
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                true,
                "duplicate_command",
                Vec::new(),
                None,
            );
        }
        Verdict::Accept => {}
    }

    // --- POLICY_CHECK stage (spec §18.6) ---
    // After deterministic core validation, apply local policy.
    // This is a bounded local policy rule: different nodes may configure
    // different controller sets and command-type allow-lists.
    let issuer_id = command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default();
    let policy_ctx = edgerun_core::command::CommandPolicyContext {
        issuer_identity_id: &issuer_id,
        command_type: command.command_type,
        has_valid_delegation: !command.delegation_chain.is_empty(),
        controller_ids: &controllers.to_vec(),
        allowed_command_types: &[], // Empty = all types allowed by local policy
    };
    let policy_result = edgerun_core::command::validate_command_policy(&policy_ctx);
    if policy_result.verdict == Verdict::Reject {
        let reason = policy_result
            .reason_code
            .map(|r| r.as_str().to_string())
            .unwrap_or_else(|| "policy_denied".into());
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            &reason,
            Vec::new(),
            None,
        );
    }

    if matches!(
        CommandType::from_i32(command.command_type),
        Some(
            CommandType::AddController
                | CommandType::RemoveController
                | CommandType::TransferControl
        )
    ) {
        let current_controllers = controllers.to_vec();
        let control_result = validate_control_change_command(command, &current_controllers, 1);
        if control_result.verdict != Verdict::Accept {
            let reason = control_result
                .reason_code
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "control_change_rejected".into());
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &reason,
                Vec::new(),
                None,
            );
        }
    }

    // ===========================================================================
    // Encrypted envelope validation — strict communication invariant
    //
    // ALL private data MUST be:
    // - encrypted before entering the system
    // - recipient-bound (>= 1 recipient)
    // - signature-protected
    //
    // Node MUST NEVER accept plaintext for private payloads.
    // CommandEnvelope.payload MUST be EncryptedEnvelope or explicitly PUBLIC object.
    // ===========================================================================
    let needs_encryption = matches!(
        CommandType::from_i32(command.command_type),
        Some(
            CommandType::StoreObject
                | CommandType::ExecuteWorkload
        )
    );

    if needs_encryption {
        let payload_bytes = match &command.payload {
            Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(b)) => Some(b.clone()),
            _ => None,
        };

        if let Some(raw) = payload_bytes {
            // Check if it's a plaintext payload that should be encrypted
            // Try to decode as EncryptedEnvelope first
            match EncryptedEnvelope::decode(raw.as_slice()) {
                Ok(env) => {
                    if let Err(reason) = validate_encrypted_envelope(&env) {
                        return record_and_respond(
                            command,
                            store,
                            stream_id,
                            signer,
                            controllers,
                            false,
                            &reason,
                            Vec::new(),
                            None,
                        );
                    }
                }
                Err(_) => {
                    // Not a valid EncryptedEnvelope - reject plaintext payloads
                    return record_and_respond(
                        command,
                        store,
                        stream_id,
                        signer,
                        controllers,
                        false,
                        "PLAINTEXT_PAYLOAD_REJECTED",
                        Vec::new(),
                        None,
                    );
                }
            }
        }
    }

    // Dispatch by command type — uses handler modules in command_handlers/
    let command_type = command.command_type;
    match command_type {
        x if x == CommandType::AddController as i32 => {
            control::dispatch_add_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RemoveController as i32 => {
            control::dispatch_remove_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::TransferControl as i32 => {
            control::dispatch_transfer_control(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::PublishSnapshot as i32 => {
            // Snapshot publishing is handled via ProduceSnapshot request
            record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "use_produce_snapshot_request",
                Vec::new(),
                None,
            )
        }
        x if x == CommandType::FetchObject as i32 => {
            // Object fetching is handled via FetchObject request
            record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "use_fetch_object_request",
                Vec::new(),
                None,
            )
        }
        x if x == CommandType::Query as i32 => {
            // Queries are handled via Query request
            record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                true,
                "",
                Vec::new(),
                None,
            )
        }
        x if x == CommandType::ExecuteWorkload as i32 => record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "execute_workload_not_supported",
            Vec::new(),
            None,
        ),
        x if x == CommandType::TerminateWorkload as i32 => record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "terminate_workload_not_supported",
            Vec::new(),
            None,
        ),
        x if x == CommandType::CreateDelegation as i32
            || x == CommandType::CreateRevocation as i32 =>
        {
            custom::dispatch_custom_command(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::UpdateConfig as i32 => {
            config::dispatch_update_config(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::InstallApp as i32 => {
            apps::dispatch_install_app(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::UninstallApp as i32 => {
            apps::dispatch_uninstall_app(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::CreateIdentity as i32 => {
            identity::dispatch_create_identity(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::ImportIdentity as i32 => {
            identity::dispatch_import_identity(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::AddBootstrapNode as i32 => {
            infrastructure::dispatch_add_bootstrap_node(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::AddReachabilityHint as i32 => {
            infrastructure::dispatch_add_reachability_hint(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::QueryNodeState as i32 => {
            infrastructure::dispatch_query_node_state(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RequestUserPresence as i32 => {
            user::dispatch_request_user_presence(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RequestSignature as i32 => {
            user::dispatch_request_signature(command, store, stream_id, signer, controllers)
        }
        _ => {
            // Spec §11.1: unknown values in authority-critical enums MUST cause rejection.
            // CommandType is authority-critical. The proto reserves 13..=999.
            // Extension types start at 1000; unknown types below 1000 are rejected.
            if command_type > 0 && command_type < 1000 {
                record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "unknown_command_type_reserved",
                    Vec::new(),
                    None,
                )
            } else {
                // Unknown or unsupported extension types are authority-critical.
                // Local policy can add explicit handlers, but the default is fail-closed.
                record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "unsupported_extension_command_type",
                    Vec::new(),
                    None,
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Command type handlers
// ---------------------------------------------------------------------------

/// UpdateConfig command handler - applies config patches to the event store
// ---------------------------------------------------------------------------
// Bootstrap / Settings app command handlers
// ---------------------------------------------------------------------------



fn extract_app_id_from_command(command: &CommandEnvelope) -> Vec<u8> {
    use edgerun_proto::edgerun::v0::stream::AppIntent;

    if !command.app_intent.is_empty() {
        if let Ok(intent) = AppIntent::decode(command.app_intent.as_slice()) {
            return intent.app_id;
        }
    }
    command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// User Authority Acquisition handlers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Custom command dispatch (delegation, revocation)
// ---------------------------------------------------------------------------

/// Handles custom commands: delegation records and revocation records.
// ---------------------------------------------------------------------------
// Event payload object creation helpers
// ---------------------------------------------------------------------------

/// Creates a NodeGenesisPayload, stores it as an encrypted object, and returns the ObjectRef.
pub fn create_node_genesis_payload(
    store: &mut NodeStore,
    stream_id: &[u8],
    node_id: &edgerun_hardware_signing::NodeID,
    initial_controllers: &[Vec<u8>],
) -> edgerun_proto::edgerun::v0::common::ObjectRef {
    use edgerun_proto::edgerun::v0::common::IdentityRef;
    use edgerun_proto::edgerun::v0::stream::NodeGenesisPayload;

    // Spec §14.12: initial_controllers is repeated required — must have at least one
    if initial_controllers.is_empty() {
        edgerun_log::error!("genesis requires at least one initial controller");
        return edgerun_proto::edgerun::v0::common::ObjectRef {
            object_id: vec![],
            object_kind: None,
        };
    }

    let controllers: Vec<IdentityRef> = initial_controllers
        .iter()
        .map(|id| IdentityRef {
            identity_id: id.clone(),
            identity_kind: Some(2), // NODE
            key_hint: None,
        })
        .collect();

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
///
/// `related_events` populates the `related_events` field for causal linkage
/// (e.g. ActionCompleted → ActionStarted for the same command).
pub(crate) fn append_signed_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    event_type: EventType,
    event_version: u32,
    payload_object: Option<edgerun_proto::edgerun::v0::common::ObjectRef>,
    related_commands: Vec<edgerun_proto::edgerun::v0::common::CommandRef>,
    related_delegations: Vec<edgerun_proto::edgerun::v0::common::DelegationRef>,
    related_events: Vec<edgerun_proto::edgerun::v0::common::EventRef>,
) -> Option<u64> {
    let head_seq = store
        .get_head(stream_id)
        .ok()
        .flatten()
        .map(|(s, _)| s)
        .unwrap_or(-1);
    let mut event = EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq: (head_seq + 1) as u64,
        prev_event_hash: store
            .get_head(stream_id)
            .ok()
            .flatten()
            .map(|(_, h)| Digest {
                algorithm: 1,
                value: h,
            }),
        event_type: event_type as i32,
        event_version,
        recorded_at: Some(now_prost_timestamp()),
        effective_at: None,
        payload_object,
        related_events,
        related_commands,
        related_objects: vec![],
        related_delegations,
        related_revocations: vec![],
        event_metadata: None,
        signature: None,
    };

    let seq = event.seq;
    if let Err(e) = sign_and_append_event_blocking(store, event, signer) {
        edgerun_log::warn!("failed to sign and append event: {}", e);
        return None;
    }
    Some(seq)
}

/// Creates a CommandSent payload and appends the event to the local stream.
/// Called when a command is issued to a peer (recorded in the sender's stream).
pub fn record_command_sent_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &CommandEnvelope,
) {
    use edgerun_proto::edgerun::v0::common::{CommandRef, EventRef};
    use edgerun_proto::edgerun::v0::stream::CommandSentPayload;

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
        store,
        stream_id,
        signer,
        EventType::CommandSent,
        1,
        Some(object_ref),
        vec![command_ref],
        vec![],
        vec![],
    );
}

/// Creates an ActionStarted/Completed/Failed payload and appends the event.
///
/// `related_event_refs` populates `related_events` for causal linkage
/// (e.g. ActionCompleted referencing ActionStarted for the same command).
pub fn record_action_event(
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    command: &CommandEnvelope,
    action_status: i32, // ACTION_STATUS_STARTED=1, COMPLETED=2, FAILED=3
    event_type: EventType,
    related_event_refs: Vec<edgerun_proto::edgerun::v0::common::EventRef>,
) {
    use edgerun_proto::edgerun::v0::stream::ActionLifecyclePayload;

    let command_ref = command_ref_from(command);

    let action_id = format!(
        "action-{}",
        edgerun_core::util::bytes_to_hex_prefixed(
            &command.command_id[..4.min(command.command_id.len())]
        )
    )
    .into_bytes();
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
        store,
        stream_id,
        signer,
        event_type,
        1,
        Some(object_ref),
        vec![command_ref],
        vec![],
        related_event_refs,
    );
}

// ---------------------------------------------------------------------------
// Canonical CommandResultPayload builder
// ---------------------------------------------------------------------------

/// Build a `CommandResultPayload` with all required fields, always including
/// a valid `CommandRef` (command_id + command_hash).
///
/// Per protocol invariants, every `CommandResultPayload` MUST reference the
/// command it is answering. This helper ensures that invariant.
fn build_command_result_payload(
    command: &CommandEnvelope,
    decision: i32,
    reason_code: &str,
    decision_basis: Option<ObjectRef>,
    effect_summary_object: Option<ObjectRef>,
    result_object: Option<ObjectRef>,
) -> ProtoCommandResultPayload {
    ProtoCommandResultPayload {
        payload_version: 1,
        command: Some(CommandRef {
            command_id: command.command_id.clone(),
            command_hash: Some(command_hash(command)),
        }),
        issuer: command.issuer.clone(),
        decision,
        decision_basis,
        reason_code: reason_code.to_string(),
        effect_summary_object,
        result_object,
    }
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
    command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default()
}

/// Records a command result event and returns the response.
/// Emits the full action lifecycle: ActionStarted → CommandCommitted/Rejected → ActionCompleted/Failed.
/// ActionCompleted and ActionFailed reference the ActionStarted event via `related_events`
/// for causal linkage through the event chain.
/// If `controller_change` is Some, records the controller change in the database.
/// If the command has `requested_assurance` and is committed, generates an AssuranceClaim.
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
    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        _controllers,
        committed,
        reason_code,
        response_bytes,
        controller_change,
        None,
    )
}

fn record_and_respond_with_result_object(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    _controllers: &ControllerSet,
    committed: bool,
    reason_code: &str,
    response_bytes: Vec<u8>,
    controller_change: Option<(&str, &str)>, // (controller_hex, change_type)
    result_object: Option<edgerun_proto::edgerun::v0::common::ObjectRef>,
) -> CommandDispatchResult {
    let command_id_bytes = command.command_id.clone();
    let command_ref = CommandRef {
        command_id: command_id_bytes.clone(),
        command_hash: Some(command_hash(command)),
    };

    let delegations: Vec<edgerun_proto::edgerun::v0::common::DelegationRef> = command
        .delegation_chain
        .iter()
        .map(|d| edgerun_proto::edgerun::v0::common::DelegationRef {
            delegation_id: d.delegation_id.clone(),
            delegation_hash: Some(delegation_hash(d)),
        })
        .collect();

    // 1. Emit ActionStarted for committed commands
    if committed {
        record_action_event(
            store,
            stream_id,
            signer,
            command,
            1, // ACTION_STATUS_STARTED
            EventType::ActionStarted,
            vec![],
        );
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
        result_object,
    };
    let result_bytes = prost::Message::encode_to_vec(&result_payload);
    let object_ref = store_object_or_log(store, &result_bytes, 6, &[stream_id.to_vec()]);

    // 3. Emit CommandCommitted or CommandRejected event with payload
    let event_type = if committed {
        EventType::CommandCommitted
    } else {
        EventType::CommandRejected
    };
    let event_seq = append_signed_event(
        store,
        stream_id,
        signer,
        event_type,
        1,
        Some(object_ref),
        vec![command_ref],
        delegations,
        vec![],
    );

    // 3b. Write persistent replay cache entry (spec §19.10)
    // The replay key is command_hash, not command_id.
    if let Some(ref target) = command.target_node {
        let target_hex = edgerun_core::util::bytes_to_hex(&target.node_id);
        let cmd_hash_hex = edgerun_core::util::bytes_to_hex(&command_hash(command).value);
        if let Err(e) = store.put_replay_entry(
            &target_hex,
            &cmd_hash_hex,
            &edgerun_core::util::bytes_to_hex(&command.command_id),
            event_seq.unwrap_or(0) as i64,
        ) {
            edgerun_log::warn!("failed to write replay cache entry: {}", e);
        }
    }

    // 4. Emit ActionCompleted or ActionFailed
    if committed {
        record_action_event(
            store,
            stream_id,
            signer,
            command,
            2, // ACTION_STATUS_COMPLETED
            EventType::ActionCompleted,
            vec![],
        );

        // Generate assurance claim if the command requested one
        if let Some(ref req) = command.requested_assurance {
            if req.required_class > 0 {
                if let Some(claim_ref) = super::assurance::generate_and_record_assurance_claim(
                    store,
                    stream_id,
                    signer,
                    command,
                    req.required_class,
                ) {
                    edgerun_log::info!("assurance claim generated for committed command");
                }
            }
        }
    } else {
        record_action_event(
            store,
            stream_id,
            signer,
            command,
            3, // ACTION_STATUS_FAILED
            EventType::ActionFailed,
            vec![],
        );
    }

    // 5. Record controller change if this was a committed control command
    if committed {
        if let Some((controller_hex, change_type)) = controller_change {
            if let Err(e) = store.record_controller_change(
                controller_hex,
                change_type,
                event_seq.unwrap_or(0) as i64,
            ) {
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

fn apply_config_patch(config: &mut NodeConfig, patch_bytes: &[u8]) -> Result<bool, String> {
    let patch: JsonValue = edgerun_json::from_slice(patch_bytes)
        .map_err(|e| format!("invalid_config_patch_json: {e}"))?;
    apply_config_patch_value(config, patch)
}

fn try_apply_projected_config_patch(
    config: &mut NodeConfig,
    patch_bytes: &[u8],
) -> Result<bool, String> {
    let Ok(patch) = edgerun_json::from_slice(patch_bytes) else {
        return Ok(false);
    };
    if !is_config_patch_value(&patch) {
        return Ok(false);
    }
    apply_config_patch_value(config, patch)
}

fn apply_config_patch_value(config: &mut NodeConfig, patch: JsonValue) -> Result<bool, String> {
    let JsonValue::Object(object) = patch else {
        return Err("config patch must be a JSON object".into());
    };

    let mut changed = false;
    if let Some(name) = object.get("name").and_then(JsonValue::as_str) {
        config.name = Some(name.to_string());
        changed = true;
    }
    for key in [
        "controllers",
        "trust_nodes",
        "allowed_peers",
        "bootstrap_peers",
    ] {
        if let Some(values) = object.get(key) {
            let parsed = json_string_array(values)
                .ok_or_else(|| format!("{key} must be an array of strings"))?;
            match key {
                "controllers" => config.controllers = parsed,
                "trust_nodes" => config.trust_nodes = parsed,
                "allowed_peers" => config.allowed_peers = parsed,
                "bootstrap_peers" => config.bootstrap_peers = parsed,
                _ => {}
            }
            changed = true;
        }
    }

    if object.get("stream_id").is_some() || object.get("signer").is_some() {
        return Err("stream_id and signer are immutable through UpdateConfig".into());
    }

    Ok(changed)
}

fn is_config_patch_value(value: &JsonValue) -> bool {
    let JsonValue::Object(object) = value else {
        return false;
    };
    [
        "name",
        "controllers",
        "trust_nodes",
        "allowed_peers",
        "bootstrap_peers",
        "stream_id",
        "signer",
    ]
    .iter()
    .any(|key| object.get(key).is_some())
}

fn json_string_array(value: &JsonValue) -> Option<Vec<String>> {
    let JsonValue::Array(items) = value else {
        return None;
    };
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        parsed.push(item.as_str()?.to_string());
    }
    Some(parsed)
}

/// Sign an event envelope with the local node's key.
/// Shared utility used by both command_dispatch and main node logic.
pub(crate) fn sign_event_envelope(
    event: &mut EventEnvelope,
    signer: &dyn MeshSigner,
) -> Result<(), String> {
    use edgerun_core::crypto::SIG_DOMAIN_EVENT_ENVELOPE;
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let sig = signer
        .sign_record(SIG_DOMAIN_EVENT_ENVELOPE, &canonical)
        .map_err(|e| format!("signing failed: {}", e))?;
    event.signature = Some(edgerun_core::protocol::Signature {
        algorithm: 1,
        value: sig.to_vec(),
    });
    Ok(())
}

/// Signs an event and appends it to the authoritative store in one operation.
pub(crate) async fn sign_and_append_event(
    store: &NodeStore,
    event: EventEnvelope,
    signer: &dyn MeshSigner,
) -> Result<u64, String> {
    store
        .append_signed_event(event, signer)
        .await
        .map_err(|e| format!("append failed: {e}"))
}

/// Blocking form of `sign_and_append_event` for the store task.
pub(crate) fn sign_and_append_event_blocking(
    store: &NodeStore,
    event: EventEnvelope,
    signer: &dyn MeshSigner,
) -> Result<u64, String> {
    store
        .append_signed_event_blocking(event, signer)
        .map_err(|e| format!("append failed: {e}"))
}

fn delegation_hash(delegation: &edgerun_proto::edgerun::v0::trust::DelegationRecord) -> Digest {
    let mut signable = delegation.clone();
    signable.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable, &mut canonical).expect("prost encode failed for delegation");
    Digest {
        algorithm: 1,
        value: edgerun_core::crypto::record_hash(
            edgerun_core::crypto::HASH_DOMAIN_DELEGATION_RECORD,
            &canonical,
        ),
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
fn delegation_refs_from(
    command: &CommandEnvelope,
) -> Vec<edgerun_proto::edgerun::v0::common::DelegationRef> {
    command
        .delegation_chain
        .iter()
        .map(|d| edgerun_proto::edgerun::v0::common::DelegationRef {
            delegation_id: d.delegation_id.clone(),
            delegation_hash: Some(delegation_hash(d)),
        })
        .collect()
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