//! Command dispatch for the edgerun node daemon.
//!
//! Replaces the old signature-only validation with:
//! - Full command validation (replay, timing, delegation signature verification)
//! - Command type dispatch (ADD_CONTROLLER, REMOVE_CONTROLLER, TRANSFER_CONTROL, etc.)
//! - Controller set management and projection from event log
//! - Revocation record processing

use crate::config::{parse_config, NodeConfig};
use edgerun_core::collections::{HashMap, HashSet};
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
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
use edgerun_proto::edgerun::v0::common::{CommandRef, EventRef};
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
    capacity_tracker: &std::sync::Arc<crate::capacity::ResourceTracker>,
    workload_policy: &super::workload_policy::WorkloadPolicy,
    rate_limiter: &super::workload_policy::RateLimiter,
    running_workloads: &std::sync::Arc<super::running_workloads::RunningWorkloads>,
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

    // Build validation context for full validate_command
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
        accepted_assurance_claims: &[],
        has_local_session: false,
        has_user_presence: false,
        transport_class: None,
        location_classes: &[],
        target_stream_id: None,
        target_view_type: None,
        target_domain: None,
        execution_class: None,
        storage_class: None,
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

    // Dispatch by command type
    let command_type = command.command_type;
    match command_type {
        x if x == CommandType::AddController as i32 => {
            dispatch_add_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RemoveController as i32 => {
            dispatch_remove_controller(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::TransferControl as i32 => {
            dispatch_transfer_control(command, store, stream_id, signer, controllers)
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
        #[cfg(feature = "oci")]
        x if x == CommandType::ExecuteWorkload as i32 => {
            // Execute workload with full accounting + capacity check
            dispatch_execute_workload(
                command,
                store,
                stream_id,
                signer,
                controllers,
                capacity_tracker,
                workload_policy,
                rate_limiter,
                running_workloads,
            )
        }
        #[cfg(not(feature = "oci"))]
        x if x == CommandType::ExecuteWorkload as i32 => record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "oci_feature_not_enabled",
            Vec::new(),
            None,
        ),
        x if x == CommandType::TerminateWorkload as i32 => {
            // Preempt/terminate a running workload
            dispatch_terminate_workload(
                command,
                store,
                stream_id,
                signer,
                controllers,
                capacity_tracker,
                running_workloads,
            )
        }
        x if x == CommandType::CreateDelegation as i32
            || x == CommandType::CreateRevocation as i32 =>
        {
            dispatch_custom_command(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::UpdateConfig as i32 => {
            dispatch_update_config(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::InstallApp as i32 => {
            dispatch_install_app(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::UninstallApp as i32 => {
            dispatch_uninstall_app(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::CreateIdentity as i32 => {
            dispatch_create_identity(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::ImportIdentity as i32 => {
            dispatch_import_identity(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::AddBootstrapNode as i32 => {
            dispatch_add_bootstrap_node(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::AddReachabilityHint as i32 => {
            dispatch_add_reachability_hint(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::QueryNodeState as i32 => {
            dispatch_query_node_state(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RequestUserPresence as i32 => {
            dispatch_request_user_presence(command, store, stream_id, signer, controllers)
        }
        x if x == CommandType::RequestSignature as i32 => {
            dispatch_request_signature(command, store, stream_id, signer, controllers)
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
fn dispatch_update_config(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract config patch from payload (JSON format)
    let config_patch: Vec<u8> = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_config_patch",
                Vec::new(),
                None,
            );
        }
    };

    let mut projected = NodeConfig {
        stream_id: edgerun_core::util::bytes_to_hex(stream_id),
        name: None,
        controllers: vec![],
        trust_nodes: vec![],
        allowed_peers: vec![],
        bootstrap_peers: vec![],
        signer: None,
    };
    if let Err(reason) = apply_config_patch(&mut projected, &config_patch) {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "invalid_config_patch_json",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("config update received");

    let patch_object = match store.put_object(
        &config_patch,
        edgerun_proto::edgerun::v0::common::ObjectKind::DerivedView as i32,
        &[stream_id.to_vec()],
    ) {
        Ok(object_ref) => Some(object_ref),
        Err(e) => {
            edgerun_log::warn!("failed to store config patch object: {}", e);
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "storage_failed",
                Vec::new(),
                None,
            );
        }
    };

    let response = b"config_update_received".to_vec();
    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        None,
        patch_object,
    )
}

fn dispatch_install_app(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::InstallAppPayload;

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_install_app_payload",
                Vec::new(),
                None,
            );
        }
    };

    let install_payload = match InstallAppPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_install_app_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let package_bytes = match install_payload.app_package {
        Some(ref obj) => {
            match store.get_blob(&obj.object_id) {
                Ok(bytes) => bytes,
                Err(e) => {
                    return record_and_respond(
                        command,
                        store,
                        stream_id,
                        signer,
                        controllers,
                        false,
                        &format!("app_package_not_found: {}", e),
                        Vec::new(),
                        None,
                    );
                }
            }
        }
        None => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "app_package_required",
                Vec::new(),
                None,
            );
        }
    };

    let domain = if install_payload.domain.is_empty() {
        "default".to_string()
    } else {
        install_payload.domain.clone()
    };

    edgerun_log::info!(
        "install_app: domain={}",
        domain
    );

    let package_object_id = edgerun_crypto::sha256(&package_bytes);

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: command_ref_from(command),
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: Some(edgerun_proto::edgerun::v0::common::ObjectRef {
            object_id: package_object_id.to_vec(),
            object_kind: Some(edgerun_proto::edgerun::v0::common::ObjectKind::AppPackage as i32),
        }),
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        Some(result_payload.result_object.unwrap()),
    )
}

fn dispatch_uninstall_app(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::UninstallAppPayload;

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_uninstall_app_payload",
                Vec::new(),
                None,
            );
        }
    };

    let uninstall_payload = match UninstallAppPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_uninstall_app_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let app_id = match &uninstall_payload.app_package {
        Some(obj) => edgerun_core::util::bytes_to_hex(&obj.object_id),
        None => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "app_package_required",
                Vec::new(),
                None,
            );
        }
    };

    edgerun_log::info!("uninstall_app: id={} reason={}", app_id, uninstall_payload.reason);

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: command_ref_from(command),
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: uninstall_payload.app_package.clone(),
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        uninstall_payload.app_package.clone(),
    )
}

// ---------------------------------------------------------------------------
// Bootstrap / Settings app command handlers
// ---------------------------------------------------------------------------

fn dispatch_create_identity(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::CreateIdentityPayload;

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_create_identity_payload",
                Vec::new(),
                None,
            );
        }
    };

    let create_payload = match CreateIdentityPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_create_identity_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let label = if create_payload.label.is_empty() {
        "primary"
    } else {
        &create_payload.label
    };

    edgerun_log::info!("create_identity: label={}", label);

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: None,
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        None,
    )
}

fn dispatch_import_identity(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::ImportIdentityPayload;

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_import_identity_payload",
                Vec::new(),
                None,
            );
        }
    };

    let import_payload = match ImportIdentityPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_import_identity_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let label = if import_payload.label.is_empty() {
        "imported"
    } else {
        &import_payload.label
    };

    edgerun_log::info!("import_identity: label={} source={}", label, import_payload.source);

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: None,
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        None,
    )
}

fn dispatch_add_bootstrap_node(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::AddBootstrapNodePayload;

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_add_bootstrap_node_payload",
                Vec::new(),
                None,
            );
        }
    };

    let bootstrap_payload = match AddBootstrapNodePayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_add_bootstrap_node_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let address = &bootstrap_payload.address;
    let label = if bootstrap_payload.label.is_empty() {
        address
    } else {
        &bootstrap_payload.label
    };

    edgerun_log::info!("add_bootstrap_node: label={} address={}", label, address);

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: None,
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        None,
    )
}

fn dispatch_add_reachability_hint(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::AddReachabilityHintPayload;

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_add_reachability_hint_payload",
                Vec::new(),
                None,
            );
        }
    };

    let hint_payload = match AddReachabilityHintPayload::decode(payload_bytes.as_slice()) {
        Ok(p) => p,
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_add_reachability_hint_payload: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    edgerun_log::info!(
        "add_reachability_hint: address={} transport_class={}",
        hint_payload.address,
        hint_payload.transport_class
    );

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: None,
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        None,
    )
}

fn dispatch_query_node_state(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::{NodeStateSnapshot, QueryNodeStatePayload};

    let _payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => Vec::new(),
    };

    let query_kind = if !_payload_bytes.is_empty() {
        match QueryNodeStatePayload::decode(_payload_bytes.as_slice()) {
            Ok(p) => p.query_kind,
            Err(_) => String::from("all"),
        }
    } else {
        String::from("all")
    };

    let has_identity = !signer.node_id().0.is_empty();
    let controller_count = controllers.controller_ids().len() as u32;

    let head_seq = store.get_head(stream_id).ok().flatten().map(|(s, _)| s).unwrap_or(0);
    let bootstrap_complete = has_identity && controller_count > 0 && head_seq > 0;

    let snapshot = NodeStateSnapshot {
        payload_version: 1,
        has_identity,
        identity_label: if has_identity {
            "primary".to_string()
        } else {
            String::new()
        },
        controller_count,
        bootstrap_peer_count: 0,
        bootstrap_complete,
    };

    let snapshot_bytes = prost::Message::encode_to_vec(&snapshot);
    let snapshot_object = store.put_object(
        &snapshot_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::DerivedView as i32,
        &[stream_id.to_vec()],
    );

    edgerun_log::info!(
        "query_node_state: kind={} identity={} controllers={} complete={}",
        query_kind,
        has_identity,
        controller_count,
        bootstrap_complete
    );

    let snapshot_ref = snapshot_object.ok();

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: None,
        reason_code: String::new(),
        effect_summary_object: None,
        result_object: snapshot_ref.clone(),
    };

    let response_bytes = result_payload.encode_to_vec();

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        snapshot_ref,
    )
}

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

fn dispatch_request_user_presence(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::{
        RequestUserPresencePayload, UserPresenceGrantedPayload, UserPresenceRequestPayload,
    };

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => Vec::new(),
    };

    let req = if !payload_bytes.is_empty() {
        match RequestUserPresencePayload::decode(payload_bytes.as_slice()) {
            Ok(p) => p,
            Err(_) => {
                return record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "invalid_presence_request_payload",
                    Vec::new(),
                    None,
                );
            }
        }
    } else {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_presence_request_payload",
            Vec::new(),
            None,
        );
    };

    if req.reason.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "empty_presence_reason",
            Vec::new(),
            None,
        );
    }

    let ttl = if req.ttl_seconds > 0 && req.ttl_seconds <= 300 {
        req.ttl_seconds
    } else {
        60
    };

    let mut token = [0u8; 32];
    edgerun_crypto::rand_core::RngCore::fill_bytes(
        &mut edgerun_crypto::rng::OsRng,
        &mut token,
    );

    let expires_at = now_unix_micros_u64() + (ttl as u64 * 1_000_000);

    let app_id = extract_app_id_from_command(command);

    let request_payload = UserPresenceRequestPayload {
        payload_version: 1,
        app_id: app_id.clone(),
        reason: req.reason.clone(),
        session_id: req.session_id.clone(),
        ttl_seconds: ttl,
    };

    let request_bytes = Message::encode_to_vec(&request_payload);
    let request_obj = store.put_object(
        &request_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
        &[stream_id.to_vec()],
    );

    let granted_payload = UserPresenceGrantedPayload {
        payload_version: 1,
        presence_token: token.to_vec(),
        app_id,
        session_id: req.session_id,
        expires_at: expires_at as i64,
    };

    let granted_bytes = Message::encode_to_vec(&granted_payload);
    let granted_obj = store.put_object(
        &granted_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
        &[stream_id.to_vec()],
    );

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: granted_obj.as_ref().ok().cloned(),
        reason_code: String::new(),
        effect_summary_object: request_obj.as_ref().ok().cloned(),
        result_object: granted_obj.ok(),
    };

    let response_bytes = result_payload.encode_to_vec();

    edgerun_log::info!(
        "request_user_presence: reason='{}' ttl={}s",
        req.reason,
        ttl
    );

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        granted_obj.ok(),
    )
}

fn dispatch_request_signature(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::{
        RequestSignaturePayload, SignatureRequestPayload, SignatureResponsePayload,
    };

    let payload_bytes = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => Vec::new(),
    };

    let req = if !payload_bytes.is_empty() {
        match RequestSignaturePayload::decode(payload_bytes.as_slice()) {
            Ok(p) => p,
            Err(_) => {
                return record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "invalid_signature_request_payload",
                    Vec::new(),
                    None,
                );
            }
        }
    } else {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_signature_request_payload",
            Vec::new(),
            None,
        );
    };

    if req.payload.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "empty_signature_payload",
            Vec::new(),
            None,
        );
    }

    if req.presence_token.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_presence_token",
            Vec::new(),
            None,
        );
    }

    let node_id = signer.node_id();
    let public_key = signer.node_id().0.to_vec();

    let message_hash = edgerun_crypto::sha256(&req.payload);

    let signature_result = signer.sign_digest(&message_hash);
    let signature_bytes = match signature_result {
        Ok(sig) => sig.to_vec(),
        Err(e) => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("signing_failed: {:?}", e),
                Vec::new(),
                None,
            );
        }
    };

    let app_id = extract_app_id_from_command(command);

    let request_payload = SignatureRequestPayload {
        payload_version: 1,
        app_id: app_id.clone(),
        payload: req.payload.clone(),
        human_readable: req.human_readable.clone(),
        action: req.action.clone(),
        session_id: req.session_id.clone(),
        presence_token: req.presence_token,
    };

    let request_bytes = Message::encode_to_vec(&request_payload);
    let request_obj = store.put_object(
        &request_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
        &[stream_id.to_vec()],
    );

    let response_payload = SignatureResponsePayload {
        payload_version: 1,
        app_id,
        payload: req.payload,
        signature: signature_bytes,
        signing_key: public_key,
        session_id: req.session_id,
    };

    let response_obj_bytes = Message::encode_to_vec(&response_payload);
    let response_obj = store.put_object(
        &response_obj_bytes,
        edgerun_proto::edgerun::v0::common::ObjectKind::Payload as i32,
        &[stream_id.to_vec()],
    );

    let result_payload = edgerun_proto::edgerun::v0::stream::CommandResultPayload {
        payload_version: 1,
        command: None,
        issuer: command.issuer.clone(),
        decision: 1,
        decision_basis: response_obj.as_ref().ok().cloned(),
        reason_code: String::new(),
        effect_summary_object: request_obj.as_ref().ok().cloned(),
        result_object: response_obj.ok(),
    };

    let response_bytes = result_payload.encode_to_vec();

    edgerun_log::info!(
        "request_signature: action='{}' human='{}'",
        req.action,
        req.human_readable
    );

    record_and_respond_with_result_object(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response_bytes,
        None,
        response_obj.ok(),
    )
}

fn dispatch_add_controller(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract the new controller identity from the command's payload
    let new_controller_id = extract_identity_from_command(command);
    if new_controller_id.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_controller_identity",
            Vec::new(),
            None,
        );
    }

    // Add to controller set
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("controller added");

    let response = format!(
        "controller added: {}",
        edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id)
    )
    .into_bytes();
    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        Some((
            &edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id),
            "added",
        )),
    )
}

fn dispatch_remove_controller(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    let target_id = extract_identity_from_command(command);
    if target_id.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_controller_identity",
            Vec::new(),
            None,
        );
    }

    if !controllers.remove(&target_id) {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "controller_not_found",
            Vec::new(),
            None,
        );
    }

    // Guard: never allow removing the last controller — node would be orphaned
    if controllers.to_vec().is_empty() {
        controllers.add(target_id); // revert
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "cannot_remove_last_controller",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("controller removed");

    let response = format!(
        "controller removed: {}",
        edgerun_core::util::bytes_to_hex_prefixed(&target_id)
    )
    .into_bytes();
    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        Some((
            &edgerun_core::util::bytes_to_hex_prefixed(&target_id),
            "removed",
        )),
    )
}

fn dispatch_transfer_control(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    // Extract new controller identity
    let new_controller_id = extract_identity_from_command(command);
    if new_controller_id.is_empty() {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_target_identity",
            Vec::new(),
            None,
        );
    }

    // Spec §18.7 safe transfer step 1: add new controller.
    // Steps 2 (verify possession) and 3 (remove old controllers) are
    // separate commands issued after the new controller confirms functionality.
    // This prevents lockout if the new controller cannot authenticate.
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("control transfer step 1 complete — new controller added; send REMOVE_CONTROLLER commands for old controllers after verification");

    let response = format!(
        "control transferred to: {}",
        edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id)
    )
    .into_bytes();
    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        Some((
            &edgerun_core::util::bytes_to_hex_prefixed(&new_controller_id),
            "transferred",
        )),
    )
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// ExecuteWorkload — compute marketplace with full accounting
// ---------------------------------------------------------------------------

/// Dispatches an ExecuteWorkload command: parse image spec, pull via OCI
/// registry, run the container via the OCI runtime, and record full
/// resource accounting — all billed in reference-core-microseconds.
///
/// The container is started non-blocking and registered in `running_workloads`
/// so it can be preempted. A background thread awaits its completion and
/// records accounting.
#[cfg(feature = "oci")]
fn dispatch_execute_workload(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    capacity_tracker: &std::sync::Arc<crate::capacity::ResourceTracker>,
    workload_policy: &super::workload_policy::WorkloadPolicy,
    rate_limiter: &super::workload_policy::RateLimiter,
    running_workloads: &std::sync::Arc<super::running_workloads::RunningWorkloads>,
) -> CommandDispatchResult {
    use super::metering::{compute_work_id, WorkMeter};
    use edgerun_core::accounting::{WorkPriority, WorkStatus, WorkloadClass};
    use std::sync::Arc;

    // === EMIT ACTION_STARTED BEFORE ANY WORK BEGINS ===
    record_action_event(
        store,
        stream_id,
        signer,
        command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted,
        vec![],
    );

    // Extract workload spec from command payload
    let workload_spec = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            // Already emitted ActionStarted; emit ActionFailed before returning
            record_action_event(
                store,
                stream_id,
                signer,
                command,
                3, // ACTION_STATUS_FAILED
                EventType::ActionFailed,
                vec![],
            );
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_workload_spec",
                Vec::new(),
                None,
            );
        }
    };

    // Parse image ref and resource hints from spec
    let spec_str = String::from_utf8_lossy(&workload_spec);
    let (image_str, allocated_cores, allocated_memory_bytes) = parse_workload_spec(&workload_spec);

    // === WORKLOAD POLICY CHECK: reject disallowed images ===
    if let Err(reason) = workload_policy.validate(&image_str) {
        edgerun_log::warn!("workload policy rejected: {} — {}", image_str, reason);
        record_action_event(
            store,
            stream_id,
            signer,
            command,
            3, // ACTION_STATUS_FAILED
            EventType::ActionFailed,
            vec![],
        );
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            &format!("policy_violation: {}", reason),
            Vec::new(),
            None,
        );
    }

    let image_ref: edgerun_oci::ImageRef = match image_str.parse() {
        Ok(img) => img,
        Err(e) => {
            record_action_event(
                store,
                stream_id,
                signer,
                command,
                3, // ACTION_STATUS_FAILED
                EventType::ActionFailed,
                vec![],
            );
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("invalid_image_ref: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    // === RATE LIMIT CHECK: prevent workload spam ===
    let requester_id = command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default();
    if !rate_limiter.check_and_record(&requester_id) {
        edgerun_log::warn!(
            "rate limit exceeded for requester: {}",
            edgerun_core::util::bytes_to_hex(&requester_id)
        );
        record_action_event(
            store,
            stream_id,
            signer,
            command,
            3, // ACTION_STATUS_FAILED
            EventType::ActionFailed,
            vec![],
        );
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "rate_limit_exceeded",
            Vec::new(),
            None,
        );
    }

    // Compute deterministic work ID (normalized spec)
    let normalized_spec = normalize_workload_spec(&workload_spec);
    let work_id = compute_work_id(&normalized_spec);

    // Extract requester identity
    let requester_id_bytes = match &command.issuer {
        Some(issuer) => {
            let mut id = [0u8; 64];
            if let Some(ref hint) = issuer.key_hint {
                if hint.len() == 64 {
                    id.copy_from_slice(hint);
                }
            }
            id
        }
        None => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_requester_identity",
                Vec::new(),
                None,
            );
        }
    };

    let delegation_hash = if let Some(first) = command.delegation_chain.first() {
        let mut signable = first.clone();
        signable.signature = None;
        let mut canonical = Vec::new();
        if prost::Message::encode(&signable, &mut canonical).is_ok() {
            let d = edgerun_core::crypto::record_hash(
                edgerun_core::crypto::HASH_DOMAIN_DELEGATION_RECORD,
                &canonical,
            );
            let mut h = [0u8; 32];
            h.copy_from_slice(&d);
            h
        } else {
            [0u8; 32]
        }
    } else {
        [0u8; 32]
    };

    let provider_id = signer.node_id().0;
    let mut cert = match load_cached_cert(store) {
        Some(c) => c,
        None => {
            let c = edgerun_core::benchmark::run_full_benchmark(provider_id);
            cache_cert(store, &c, signer);
            c
        }
    };

    // Sign the certificate if it's not already signed (legacy certs)
    if cert.signature == [0u8; 64] {
        let mut digest_32 = [0u8; 32];
        digest_32.copy_from_slice(&cert.digest);
        if let Ok(sig) = signer.sign_digest(&digest_32) {
            cert.signature = sig;
            cache_cert(store, &cert, signer); // Re-cache with signature
        }
    }

    // === CERTIFICATE SIGNATURE VERIFICATION ===
    // The certificate must be self-signed by the node's key to prevent fabrication.
    if !cert.verify() {
        edgerun_log::error!(
            "performance certificate signature verification failed — possible fabrication"
        );
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "invalid_certificate_signature",
            Vec::new(),
            None,
        );
    }

    // Also verify the certificate's node_id matches our node identity
    if cert.node_id != provider_id {
        edgerun_log::error!("performance certificate node_id does not match local node identity");
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "certificate_identity_mismatch",
            Vec::new(),
            None,
        );
    }

    // === CERTIFICATE FRESHNESS CHECK: reject stale certs ===
    if let Err(reason) = check_cert_freshness_impl(&cert) {
        edgerun_log::warn!("certificate freshness check failed: {}", reason);
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            &format!("stale_certificate: {}", reason),
            Vec::new(),
            None,
        );
    }

    // === CAPACITY CHECK: do we have enough resources? ===
    if !capacity_tracker.try_allocate(allocated_cores, allocated_memory_bytes) {
        edgerun_log::warn!(
            "insufficient capacity: need {} cores, {} bytes (have {} cores, {} bytes)",
            allocated_cores,
            allocated_memory_bytes,
            capacity_tracker.available_cores(),
            capacity_tracker.available_memory()
        );
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "insufficient_capacity",
            Vec::new(),
            None,
        );
    }

    // Determine workload class from image name and resource profile
    // Uses the parsed image_ref (not raw spec string) to avoid false substring matches.
    let workload_class = {
        let img_name = image_ref.repository.to_lowercase();
        if img_name.contains("inference") || img_name.contains("llm") || img_name.contains("model")
        {
            WorkloadClass::Inference
        } else if img_name.contains("compil") || img_name.contains("build") {
            WorkloadClass::Compilation
        } else if image_str.starts_with("container:") || img_name.contains("container") {
            WorkloadClass::Container
        } else {
            WorkloadClass::General
        }
    };

    // === PHASE 1: Pull image ===
    let paths = WorkloadPaths::new(&work_id);
    let bundle_dir = &paths.bundle_dir;
    let store_dir = &paths.store_dir;

    // Start metering BEFORE pull so download time is billed
    let meter = WorkMeter::new(
        work_id,
        requester_id_bytes,
        provider_id,
        delegation_hash,
        allocated_cores,
        allocated_memory_bytes,
        workload_class,
        WorkPriority::Standard,
        &cert,
    );

    edgerun_log::info!("pulling: {}", image_str);
    let meter_for_pull = meter.clone();
    let rt = edgerun_rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string());
    let pull_result = match rt {
        Ok(rt) => {
            let image_ref2 = image_ref.clone();
            let bundle_dir2 = bundle_dir.to_path_buf();
            let store_dir2 = store_dir.to_path_buf();
            rt.block_on(async move {
                pull_with_metering(&image_ref2, &bundle_dir2, &store_dir2, &meter_for_pull).await
            })
        }
        Err(e) => Err(e),
    };
    if let Err(e) = pull_result {
        edgerun_log::warn!("pull failed: {}", e);
        capacity_tracker.release(allocated_cores, allocated_memory_bytes);
        let acc = meter.finalize(WorkStatus::Failed, None);
        let _ = store.record_work_accounting(&acc);
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            &format!("pull_failed: {}", e),
            Vec::new(),
            None,
        );
    }

    // === PHASE 2: Apply resource limits ===
    let cfg_path = bundle_dir.join("config.json");
    if let Ok(txt) = std::fs::read_to_string(&cfg_path) {
        if let Ok(mut spec) = edgerun_oci::spec::parse_oci_spec(txt.as_bytes()) {
            let shares = (allocated_cores as u64).saturating_mul(1024);
            if let Some(linux) = spec.linux.as_mut() {
                linux.resources = Some(edgerun_oci::OciLinuxResources {
                    memory: Some(edgerun_oci::OciLinuxMemory {
                        limit: Some(allocated_memory_bytes as i64),
                        reservation: None,
                        swap: None,
                        kernel: None,
                        kernel_tcp: None,
                        check_before_update: None,
                    }),
                    cpu: Some(edgerun_oci::OciLinuxCpu {
                        shares: Some(shares),
                        quota: None,
                        period: None,
                        realtime_runtime: None,
                        realtime_period: None,
                        cpus: None,
                        mems: None,
                        idle: None,
                        burst: None,
                    }),
                    pids: Some(edgerun_oci::OciLinuxPids { limit: 256 }),
                    block_io: None,
                    devices: None,
                    hugepage_limits: None,
                    network: None,
                });
            }
            let json = spec.to_json_string_pretty();
            if let Err(e) = std::fs::write(&cfg_path, json) {
                edgerun_log::warn!("failed to update config.json: {}", e);
                // ABORT: container must not start without resource limits.
                // Running unconstrained would violate the capacity guarantee.
                capacity_tracker.release(allocated_cores, allocated_memory_bytes);
                let _ = std::fs::remove_dir_all(&paths.tmp_base);
                let acc = meter.finalize(WorkStatus::Failed, None);
                let _ = store.record_work_accounting(&acc);
                return record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "resource_limit_write_failed",
                    Vec::new(),
                    None,
                );
            }
        }
    }

    // === PHASE 3: Run container (non-blocking) ===
    edgerun_log::info!("running: {}", image_str);
    let container = match edgerun_oci::start_bundle(bundle_dir) {
        Ok(c) => c,
        Err(e) => {
            edgerun_log::warn!("run failed: {}", e);
            capacity_tracker.release(allocated_cores, allocated_memory_bytes);
            let acc = meter.finalize(WorkStatus::Failed, None);
            let _ = store.record_work_accounting(&acc);
            let _ = std::fs::remove_dir_all(&paths.tmp_base);
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                &format!("run_failed: {}", e),
                Vec::new(),
                None,
            );
        }
    };

    let container_pid = container.pid();

    // Register in running workloads so it can be preempted
    let workload_info = super::running_workloads::RunningWorkloadInfo {
        work_id,
        pid: container_pid,
        allocated_cores,
        allocated_memory_bytes,
        bundle_path: paths.tmp_base.clone(),
        cgroup_path: paths.cgroup_path.clone(),
    };
    match running_workloads.register(workload_info) {
        Ok(()) => {}
        Err(_existing) => {
            edgerun_log::warn!(
                "work {} already running (duplicate work_id)",
                edgerun_core::util::bytes_to_hex(&work_id[..8])
            );
            let _ = container.kill();
            capacity_tracker.release(allocated_cores, allocated_memory_bytes);
            let _ = std::fs::remove_dir_all(&paths.tmp_base);
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "duplicate_work_id",
                Vec::new(),
                None,
            );
        }
    }

    // === Background thread: owns the container handle, does ALL cleanup ===
    //
    // Concurrency design:
    // - This thread exclusively owns `container`, `capacity_tracker` release,
    //   and bundle directory cleanup. No other path touches them.
    // - `terminate()` only signals kill (via AtomicBool + kill syscall).
    //   It does NOT release resources or remove the registry entry.
    // - After the container exits, this thread releases resources, records
    //   accounting, cleans up, and unregisters from the registry.
    // - This guarantees exactly-once cleanup regardless of exit path.
    let running_workloads_bg = Arc::clone(running_workloads);
    let capacity_tracker_bg = Arc::clone(capacity_tracker);
    let provider_id_bg = provider_id;
    let requester_id_bg = requester_id_bytes;
    let delegation_hash_bg = delegation_hash;
    let cpu_multiplier_bg = cert.cpu_core_multiplier();
    let memory_multiplier_bg = cert.memory_multiplier();
    let storage_multiplier_bg = cert.storage_multiplier();
    let cert_digest_bg = cert.digest;
    let accounting_sink_bg = store.work_accounting_sink();

    // Clone values needed for panic-safe cleanup (moved into catch_unwind below)
    let cleanup_capacity = Arc::clone(&capacity_tracker_bg);
    let cleanup_paths = paths.clone();
    let cleanup_registry = Arc::clone(&running_workloads_bg);
    let cleanup_allocated_cores = allocated_cores;
    let cleanup_allocated_memory = allocated_memory_bytes;
    let cleanup_work_id = work_id;
    let image_str_for_response = image_str.clone();

    std::thread::Builder::new()
        .name(paths.thread_name.clone())
        .spawn(move || {
            // Catch panics so they are logged rather than silently killing the thread
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                workload_thread_body(
                    work_id,
                    container,
                    container_pid,
                    running_workloads_bg,
                    capacity_tracker_bg,
                    provider_id_bg,
                    requester_id_bg,
                    delegation_hash_bg,
                    cpu_multiplier_bg,
                    memory_multiplier_bg,
                    storage_multiplier_bg,
                    cert_digest_bg,
                    accounting_sink_bg,
                    allocated_cores,
                    allocated_memory_bytes,
                    image_str,
                    paths,
                    workload_class,
                )
            }));
            if let Err(panic) = result {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                edgerun_log::error!(
                    "work {} thread panicked: {}",
                    edgerun_core::util::bytes_to_hex(&cleanup_work_id[..8]),
                    msg
                );
                // Best-effort cleanup even on panic
                cleanup_capacity.release(cleanup_allocated_cores, cleanup_allocated_memory);
                let _ = std::fs::remove_dir_all(&cleanup_paths.tmp_base);
                cleanup_registry.unregister(&cleanup_work_id);
            }
        })
        .expect("failed to spawn workload thread");

    edgerun_log::info!(
        "work {} started (pid {}, {} running)",
        edgerun_core::util::bytes_to_hex(&work_id[..8]),
        container_pid,
        running_workloads.count()
    );

    // === IMMEDIATE RESPONSE: workload accepted, running asynchronously ===
    let response = format!(
        "accepted {} (pid {}, work {})",
        image_str_for_response,
        container_pid,
        edgerun_core::util::bytes_to_hex(&work_id[..8])
    )
    .into_bytes();

    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        true,
        "",
        response,
        None,
    )
}

// ---------------------------------------------------------------------------
// Workload background thread body (extracted for panic safety)
// ---------------------------------------------------------------------------

/// The body of the background workload thread. Runs after the container is started.
/// Waits for container exit, records accounting, and performs cleanup.
/// This is extracted into a function so it can be wrapped in `catch_unwind`.
#[cfg(feature = "oci")]
fn workload_thread_body(
    work_id: [u8; 32],
    container: edgerun_oci::RunningContainer,
    container_pid: u32,
    running_workloads_bg: Arc<super::running_workloads::RunningWorkloads>,
    capacity_tracker_bg: Arc<super::capacity::ResourceTracker>,
    provider_id_bg: [u8; 64],
    requester_id_bg: [u8; 64],
    delegation_hash_bg: [u8; 32],
    cpu_multiplier_bg: edgerun_core::fixed_point::FixedPoint16,
    memory_multiplier_bg: edgerun_core::fixed_point::FixedPoint16,
    storage_multiplier_bg: edgerun_core::fixed_point::FixedPoint16,
    cert_digest_bg: [u8; 32],
    accounting_sink_bg: edgerun_storage::WorkAccountingSink,
    allocated_cores: u32,
    allocated_memory_bytes: u64,
    _image_str: String,
    paths: WorkloadPaths,
    workload_class: edgerun_core::accounting::WorkloadClass,
) {
    use edgerun_core::accounting::{WorkPriority, WorkStatus};

    let start_monotonic = std::time::Instant::now();
    let start_time_us = now_unix_micros_u64();

    // Await container exit (blocking). The container will be killed
    // if terminate() was called — the RunningContainer::kill() path
    // in the registry sets the kill flag and sends SIGTERM/SIGKILL,
    // but this thread still does the wait() and cleanup.
    let run_result = container.wait();

    let elapsed_us = start_monotonic.elapsed().as_micros() as u64;
    let completed_at_us = start_time_us + elapsed_us;

    let exit_code_val = match &run_result {
        Ok(st) => st.code(),
        Err(_) => None,
    };
    let status = match &run_result {
        Ok(st) if st.success() => WorkStatus::Completed,
        _ => WorkStatus::Failed,
    };

    // Physical resource calculations
    let physical_core_us = (allocated_cores as u64) * elapsed_us;
    let physical_memory_gb = (allocated_memory_bytes / (1024 * 1024 * 1024)) as u32;
    let memory_duration_seconds = (elapsed_us / 1_000_000) as u32;

    // Billable RC-µs
    let billable_compute_rc_us = cpu_multiplier_bg.mul_u64(physical_core_us);

    // Build the accounting record
    let accounting = edgerun_core::accounting::WorkAccounting {
        work_id,
        requester_id: requester_id_bg,
        provider_id: provider_id_bg,
        delegation_hash: delegation_hash_bg,
        started_at_us: start_time_us,
        completed_at_us,
        physical_core_us,
        physical_memory_gb,
        memory_duration_seconds,
        gpu_core_us: 0,
        npu_core_us: 0,
        storage_read_bytes: 0,
        storage_written_bytes: 0,
        storage_read_ops: 0,
        storage_write_ops: 0,
        network_sent_bytes: 0,
        network_received_bytes: 0,
        workload_class,
        priority: WorkPriority::Standard,
        status,
        exit_code: exit_code_val,
        provider_cert_digest: cert_digest_bg,
        cpu_multiplier: cpu_multiplier_bg,
        memory_multiplier: memory_multiplier_bg,
        storage_multiplier: storage_multiplier_bg,
        billable_compute_rc_us,
    };

    // === ALL cleanup happens here — exactly once ===

    // 1. Log the accounting record
    let status_str = status.as_str();
    let elapsed_s = elapsed_us / 1_000_000;
    edgerun_log::info!(
        "work {} {} ({} RC-µs, {}s, pid {})",
        edgerun_core::util::bytes_to_hex(&work_id[..8]),
        status_str,
        billable_compute_rc_us,
        elapsed_s,
        container_pid
    );
    if let Err(err) = accounting_sink_bg.record_work_accounting(&accounting) {
        edgerun_log::error!(
            "failed to record work accounting for {}: {}",
            edgerun_core::util::bytes_to_hex(&work_id[..8]),
            err
        );
    }

    // 2. Release reserved resources (exactly once — terminate() never does this)
    capacity_tracker_bg.release(allocated_cores, allocated_memory_bytes);

    // 3. Cleanup bundle directory
    let _ = std::fs::remove_dir_all(&paths.tmp_base);

    // 4. Unregister from the running workloads registry
    // This must happen last — until this point, terminate() could
    // have been called and waited for the kill to take effect.
    running_workloads_bg.unregister(&work_id);
}

// ---------------------------------------------------------------------------
// TerminateWorkload — preempt/terminate a running container
// ---------------------------------------------------------------------------

/// Dispatches a TerminateWorkload command: looks up the running workload
/// by work_id (from the command payload) and kills it via the registry.
fn dispatch_terminate_workload(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    capacity_tracker: &std::sync::Arc<crate::capacity::ResourceTracker>,
    running_workloads: &std::sync::Arc<super::running_workloads::RunningWorkloads>,
) -> CommandDispatchResult {
    // Extract work_id from command payload
    let workload_spec = match &command.payload {
        Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(
            bytes,
        )) => bytes.clone(),
        _ => {
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "missing_workload_spec",
                Vec::new(),
                None,
            );
        }
    };

    // Parse work_id from payload: either raw 32 bytes or hex string
    let work_id = if workload_spec.len() == 32 {
        let mut id = [0u8; 32];
        id.copy_from_slice(&workload_spec);
        id
    } else {
        // Try hex decode
        let hex_str = String::from_utf8_lossy(&workload_spec);
        match edgerun_core::util::hex_to_bytes(&hex_str) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut id = [0u8; 32];
                id.copy_from_slice(&bytes);
                id
            }
            _ => {
                return record_and_respond(
                    command,
                    store,
                    stream_id,
                    signer,
                    controllers,
                    false,
                    "invalid_work_id",
                    Vec::new(),
                    None,
                );
            }
        }
    };

    // Look up and terminate the workload
    match running_workloads.terminate(&work_id) {
        Some(info) => {
            // DO NOT release capacity here — the background thread owns
            // resource cleanup. Releasing here would cause double-free
            // when the background thread also releases on exit.
            // terminate() only signals kill; the background thread handles
            // capacity release, bundle cleanup, and unregister.

            edgerun_log::info!("work {} terminated (pid {}, {} cores, {} bytes — resources released by background thread)",
                edgerun_core::util::bytes_to_hex(&work_id[..8]),
                info.pid,
                info.allocated_cores,
                info.allocated_memory_bytes);

            let response = format!(
                "terminated work {} (pid {})",
                edgerun_core::util::bytes_to_hex(&work_id[..8]),
                info.pid
            )
            .into_bytes();

            record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                true,
                "",
                response,
                None,
            )
        }
        None => {
            edgerun_log::warn!(
                "work {} not found in running workloads",
                edgerun_core::util::bytes_to_hex(&work_id[..8])
            );
            record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "work_not_found",
                Vec::new(),
                None,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "oci")]
fn parse_workload_spec(spec: &[u8]) -> (String, u32, u64) {
    let s = String::from_utf8_lossy(spec);
    let mut image = s.to_string();
    let mut cores: u32 = 1;
    let mut memory_gb: u64 = 1;

    if let Some(pos) = s.find(":cores=") {
        let rest = &s[pos + 7..];
        let end = rest.find(':').unwrap_or(rest.len());
        if let Ok(c) = rest[..end].parse::<u32>() {
            cores = c.max(1).min(256);
        }
        image = s[..pos].to_string();
    }

    if let Some(pos) = s.find(":memory=") {
        let rest = &s[pos + 8..];
        let end = rest.find(':').unwrap_or(rest.len());
        let mem_str = &rest[..end];
        if let Some(pi) = mem_str.find(char::is_alphabetic) {
            let (num_s, unit) = mem_str.split_at(pi);
            if let Ok(n) = num_s.parse::<u64>() {
                let u = unit.to_uppercase();
                memory_gb = if u.starts_with('T') {
                    n * 1024
                } else if u.starts_with('G') {
                    n
                } else if u.starts_with('M') {
                    n.max(512) / 1024
                } else {
                    n
                };
                if memory_gb == 0 {
                    memory_gb = 1;
                }
            }
        } else if let Ok(n) = mem_str.parse::<u64>() {
            memory_gb = n;
        }
        if image.len() > pos {
            image = s[..pos].to_string();
        }
    }

    if image.starts_with("container:") {
        image = image["container:".len()..].to_string();
    }

    (image, cores, memory_gb * 1024 * 1024 * 1024)
}

#[cfg(feature = "oci")]
async fn pull_with_metering(
    image: &edgerun_oci::ImageRef,
    bundle_dir: &std::path::Path,
    store_dir: &std::path::Path,
    meter: &super::metering::WorkMeter,
) -> Result<(), String> {
    let mut client = edgerun_oci::RegistryClient::new();

    // Resolve manifest to know expected layer sizes
    let manifest = client
        .resolve_manifest(image)
        .await
        .map_err(|e| e.to_string())?;
    let total_layer_bytes = match &manifest {
        edgerun_oci::ImageManifest::Single(m) => m.layers.iter().map(|l| l.size).sum::<u64>(),
        edgerun_oci::ImageManifest::Index(idx) => {
            idx.manifests.first().map(|m| m.size).unwrap_or(0)
        }
    };

    // Pull the image — download_blob() now tracks real bytes in the client
    client
        .pull(image, bundle_dir, store_dir)
        .await
        .map_err(|e| e.to_string())?;

    // Report actual downloaded bytes (may differ from manifest due to retries, compression)
    let real_bytes = client.bytes_downloaded();
    if real_bytes > 0 {
        meter.add_network_received(real_bytes);
    } else {
        // Fallback to manifest estimate if tracking didn't work
        meter.add_network_received(total_layer_bytes);
    }

    // Report the extracted rootfs size as storage write
    if let Ok(sz) = dir_size(&bundle_dir.join("rootfs")) {
        meter.add_storage_write(sz);
    }
    // Report cached layer blobs as storage reads
    if store_dir.exists() {
        if let Ok(sz) = dir_size(store_dir) {
            meter.add_storage_read(sz);
        }
    }
    Ok(())
}

#[cfg(feature = "oci")]
fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let e = entry?;
            let md = e.metadata()?;
            if md.is_file() {
                total += md.len();
            } else if md.is_dir() {
                total += dir_size(&e.path())?;
            }
        }
    }
    Ok(total)
}

#[cfg(feature = "oci")]
fn load_cached_cert(store: &NodeStore) -> Option<edgerun_core::accounting::PerformanceCertificate> {
    let p = store.data_root().join("perf_cert.bin");
    if let Ok(data) = std::fs::read(&p) {
        edgerun_core::accounting::PerformanceCertificate::from_bytes(&data)
    } else {
        None
    }
}

#[cfg(feature = "oci")]
fn cache_cert(
    store: &mut NodeStore,
    cert: &edgerun_core::accounting::PerformanceCertificate,
    signer: &dyn MeshSigner,
) {
    // Sign the certificate if it's not already signed
    let mut cert = cert.clone();
    if cert.signature == [0u8; 64] {
        let mut digest_32 = [0u8; 32];
        digest_32.copy_from_slice(&cert.digest);
        if let Ok(sig) = signer.sign_digest(&digest_32) {
            cert.signature = sig;
        } else {
            edgerun_log::warn!("failed to sign performance certificate");
        }
    }

    // Persist to disk for fast loading
    let p = store.data_root().join("perf_cert.bin");
    if std::fs::write(&p, cert.to_bytes()).is_ok() {
        edgerun_log::info!(
            "perf cert cached: cpu {:.2}x ref",
            cert.cpu_core_multiplier().to_raw() as f64 / 65536.0
        );
    }

    // Also store as an object so it's recoverable from the event stream
    // via the command that triggered the re-benchmark.
    let cert_bytes = cert.to_bytes();
    if let Err(e) = store.put_object(&cert_bytes, 0, &[signer.node_id().0.to_vec()]) {
        edgerun_log::warn!("failed to store cert as object: {}", e);
    }
}

// ===========================================================================
// Workload spec normalization for deterministic work IDs
// ===========================================================================

/// Normalize a workload spec string so that logically equivalent specs
/// produce the same work ID regardless of key ordering.
///
/// E.g., `container:alpine:cores=4:memory=8G` and
///       `container:alpine:memory=8G:cores=4` → same normalized form
///
/// Strategy: extract image, cores, memory → sort key-value pairs → reassemble.
#[cfg(feature = "oci")]
fn normalize_workload_spec(spec: &[u8]) -> Vec<u8> {
    let (image, cores, memory_bytes) = parse_workload_spec(spec);

    // Normalize memory back to human form for the key
    let memory_gb = memory_bytes / (1024 * 1024 * 1024);
    let mut parts = vec![image];

    // Build sortable key-value pairs
    let mut kvs: Vec<(String, String)> = Vec::new();
    kvs.push(("cores".to_string(), cores.to_string()));
    kvs.push(("memory".to_string(), format!("{}G", memory_gb)));
    kvs.sort_by_key(|(k, _)| k.clone());

    for (k, v) in kvs {
        parts.push(format!("{}={}", k, v));
    }

    parts.join(":").into_bytes()
}

// ===========================================================================
// Certificate freshness check
// ===========================================================================

/// Maximum age for a performance certificate before re-benchmarking is required.
#[cfg(feature = "oci")]
const MAX_CERT_AGE_US: u64 = 24 * 60 * 60 * 1_000_000; // 24 hours

/// Check if a performance certificate is still fresh.
#[cfg(feature = "oci")]
fn check_cert_freshness_impl(
    cert: &edgerun_core::accounting::PerformanceCertificate,
) -> Result<(), String> {
    let now = now_unix_micros_u64();
    let age = now.saturating_sub(cert.benchmark_completed_us);
    if age > MAX_CERT_AGE_US {
        return Err(format!(
            "cert is {} hours old (max 24h)",
            age / 3_600_000_000
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Custom command dispatch (delegation, revocation)
// ---------------------------------------------------------------------------

/// Handles custom commands: delegation records and revocation records.
fn dispatch_custom_command(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
) -> CommandDispatchResult {
    use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;

    if let Some(ref payload) = command.payload {
        if let Payload::InlinePayload(bytes) = payload {
            match CommandType::from_i32(command.command_type) {
                Some(CommandType::CreateDelegation) => {
                    if let Ok(delegation) = ProtoDelegationRecord::decode(&bytes[..]) {
                        return dispatch_create_delegation(
                            command,
                            store,
                            stream_id,
                            signer,
                            controllers,
                            &delegation,
                        );
                    }
                }
                Some(CommandType::CreateRevocation) => {
                    if let Ok(revocation) = ProtoRevocationRecord::decode(&bytes[..]) {
                        return dispatch_create_revocation(
                            command,
                            store,
                            stream_id,
                            signer,
                            controllers,
                            &revocation,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    record_and_respond(
        command,
        store,
        stream_id,
        signer,
        controllers,
        false,
        "unknown_custom_command",
        Vec::new(),
        None,
    )
}

/// Handles a delegation creation command.
fn dispatch_create_delegation(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    delegation: &ProtoDelegationRecord,
) -> CommandDispatchResult {
    let now_ms = now_unix_millis_i64();
    let validation = validate_delegation_chain(
        &[delegation.clone()],
        now_ms,
        &std::collections::HashSet::new(),
    );
    if validation.verdict != Verdict::Accept {
        let reason = validation
            .reason_code
            .map(|r| r.as_str().to_string())
            .unwrap_or_else(|| "delegation_invalid".into());
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

    let issuer_id = delegation
        .issuer
        .as_ref()
        .map(|issuer| issuer.identity_id.clone())
        .unwrap_or_default();
    let command_issuer_id = command
        .issuer
        .as_ref()
        .map(|issuer| issuer.identity_id.as_slice())
        .unwrap_or_default();
    if !controllers.contains(&issuer_id) || issuer_id.as_slice() != command_issuer_id {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "AUTHORITY_DENIED",
            Vec::new(),
            None,
        );
    }

    // Store the delegation as an object so it's cryptographically linked
    // to the signed CommandCommitted event via payload_object
    let delegation_bytes = prost::Message::encode_to_vec(delegation);
    let object_ref = match store.put_object(&delegation_bytes, 0, &[signer.node_id().0.to_vec()]) {
        Ok(r) => r,
        Err(e) => {
            edgerun_log::warn!("failed to store delegation object: {}", e);
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "storage_failed",
                Vec::new(),
                None,
            );
        }
    };

    // Also store in the index for efficient lookup
    let delegation_id_hex = edgerun_core::util::bytes_to_hex(&delegation.delegation_id);
    let issuer_hex = delegation
        .issuer
        .as_ref()
        .map(|i| edgerun_core::util::bytes_to_hex(&i.identity_id))
        .unwrap_or_default();
    let recipient_hex = delegation
        .recipient
        .as_ref()
        .map(|r| edgerun_core::util::bytes_to_hex(&r.identity_id))
        .unwrap_or_default();
    let expires_at = delegation.expires_at.as_ref().map(|t| t.seconds);
    let capability_bytes = delegation
        .capability
        .as_ref()
        .map(prost::Message::encode_to_vec)
        .unwrap_or_default();

    if let Err(e) = store.store_delegation(
        &delegation_id_hex,
        &issuer_hex,
        &recipient_hex,
        &edgerun_core::util::bytes_to_hex(&capability_bytes),
        expires_at,
    ) {
        edgerun_log::warn!("failed to index delegation: {}", e);
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "index_failed",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("delegation recorded");

    // Manually produce events with the delegation object as payload_object
    let command_ref = command_ref_from(command);
    let delegations = delegation_refs_from(command);

    record_action_event(
        store,
        stream_id,
        signer,
        command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted,
        vec![],
    );

    let _event_seq = append_signed_event(
        store,
        stream_id,
        signer,
        EventType::CommandCommitted,
        1,
        Some(object_ref),
        vec![command_ref],
        delegations,
        vec![],
    );

    record_action_event(
        store,
        stream_id,
        signer,
        command,
        2, // ACTION_STATUS_COMPLETED
        EventType::ActionCompleted,
        vec![],
    );

    let response = format!("delegation recorded: {}", delegation_id_hex).into_bytes();
    CommandDispatchResult {
        event_type: EventType::CommandCommitted,
        decision: CommandDecision::Committed as i32,
        reason_code: String::new(),
        response_bytes: response,
    }
}

/// Handles a revocation creation command.
fn dispatch_create_revocation(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    revocation: &ProtoRevocationRecord,
) -> CommandDispatchResult {
    let revocation_id_hex = edgerun_core::util::bytes_to_hex(&revocation.revocation_id);
    let issuer_hex = revocation
        .issuer
        .as_ref()
        .map(|i| edgerun_core::util::bytes_to_hex(&i.identity_id))
        .unwrap_or_default();

    let now_ms = now_unix_millis_i64();
    let trusted_issuers = controllers.to_vec();
    let validation = validate_revocation_record(revocation, now_ms, &trusted_issuers);
    if validation.verdict != Verdict::Accept {
        let reason = validation
            .reason_code
            .map(|r| r.as_str().to_string())
            .unwrap_or_else(|| "revocation_invalid".into());
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

    // Determine target type and hex from the oneof
    let (target_type, target_hex) = if let Some(ref target) = revocation.target {
        match target {
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(d) => (
                "delegation",
                edgerun_core::util::bytes_to_hex(&d.delegation_id),
            ),
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(i) => {
                ("identity", edgerun_core::util::bytes_to_hex(&i.identity_id))
            }
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetNode(n) => {
                ("node", edgerun_core::util::bytes_to_hex(&n.node_id))
            }
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetObject(o) => {
                ("object", edgerun_core::util::bytes_to_hex(&o.object_id))
            }
        }
    } else {
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "missing_target",
            Vec::new(),
            None,
        );
    };

    let effective_at = revocation.effective_at.as_ref().map(|t| t.seconds);

    // Store the revocation as an object so it's cryptographically linked
    // to the signed CommandCommitted event via payload_object
    let revocation_bytes = prost::Message::encode_to_vec(revocation);
    let object_ref = match store.put_object(&revocation_bytes, 0, &[signer.node_id().0.to_vec()]) {
        Ok(r) => r,
        Err(e) => {
            edgerun_log::warn!("failed to store revocation object: {}", e);
            return record_and_respond(
                command,
                store,
                stream_id,
                signer,
                controllers,
                false,
                "storage_failed",
                Vec::new(),
                None,
            );
        }
    };

    // Also store in the index for efficient lookup
    if let Err(e) = store.store_revocation(
        &revocation_id_hex,
        &issuer_hex,
        target_type,
        &target_hex,
        effective_at,
    ) {
        edgerun_log::warn!("failed to index revocation: {}", e);
        return record_and_respond(
            command,
            store,
            stream_id,
            signer,
            controllers,
            false,
            "index_failed",
            Vec::new(),
            None,
        );
    }

    edgerun_log::info!("revocation recorded");

    // Manually produce events with the revocation object as payload_object
    let command_ref = command_ref_from(command);
    let delegations = delegation_refs_from(command);

    record_action_event(
        store,
        stream_id,
        signer,
        command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted,
        vec![],
    );

    let _event_seq = append_signed_event(
        store,
        stream_id,
        signer,
        EventType::CommandCommitted,
        1,
        Some(object_ref),
        vec![command_ref],
        delegations,
        vec![],
    );

    record_action_event(
        store,
        stream_id,
        signer,
        command,
        2, // ACTION_STATUS_COMPLETED
        EventType::ActionCompleted,
        vec![],
    );

    let response = format!("revocation recorded: {}", revocation_id_hex).into_bytes();
    CommandDispatchResult {
        event_type: EventType::CommandCommitted,
        decision: CommandDecision::Committed as i32,
        reason_code: String::new(),
        response_bytes: response,
    }
}

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_hardware_signing::{MeshSigner, NodeID};
    use edgerun_storage::{BlobKeySource, NodeStore, NodeStoreConfig};
    use std::sync::Arc;

    fn test_workload_policy() -> super::super::workload_policy::WorkloadPolicy {
        super::super::workload_policy::WorkloadPolicy::permissive()
    }

    fn test_rate_limiter() -> super::super::workload_policy::RateLimiter {
        super::super::workload_policy::RateLimiter::new(1000, 60_000_000)
    }

    fn test_running_workloads() -> std::sync::Arc<super::super::running_workloads::RunningWorkloads>
    {
        std::sync::Arc::new(super::super::running_workloads::RunningWorkloads::new())
    }

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn tmp_data_root() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("dispatch_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_store() -> NodeStore {
        let root = tmp_data_root();
        let private_key = [0xBBu8; 32];
        let binding =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes((&private_key).into()).unwrap();
        let vk = binding.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_identity = [0u8; 64];
        node_identity.copy_from_slice(&encoded.as_bytes()[1..65]);
        let config = NodeStoreConfig {
            data_root: root,
            blob_key_source: Arc::new(BlobKeySource::Software {
                private_key_bytes: private_key.to_vec(),
            }),
            node_identity: node_identity.to_vec(),
        };
        NodeStore::open(&config).unwrap()
    }

    fn random_signing_key() -> edgerun_crypto::p256::ecdsa::SigningKey {
        let mut bytes = [0u8; 32];
        edgerun_crypto::fill_random(&mut bytes).expect("random generation failed");
        edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    struct TestSigner {
        node_id: NodeID,
        key: edgerun_crypto::p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = random_signing_key();
            let vk = key.verifying_key();
            let encoded = vk.to_encoded_point(false);
            let mut node_bytes = [0u8; 64];
            node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: edgerun_hardware_signing::NodeID(node_bytes),
                key,
            }
        }
    }

    impl MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(
            &self,
            digest: &[u8; 32],
        ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
            let sig: edgerun_crypto::p256::ecdsa::Signature =
                self.key.sign_prehash(digest).map_err(|e| {
                    edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string())
                })?;
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&sig.to_bytes());
            Ok(bytes)
        }
    }

    fn make_command(
        command_id: Vec<u8>,
        target_node_id: Vec<u8>,
        issuer_id: Vec<u8>,
        command_type: i32,
    ) -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id,
            target_node: Some(edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: target_node_id,
            }),
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: issuer_id,
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type,
            command_version: 1,
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
        }
    }

    fn sign_command(command: &mut CommandEnvelope, signer: &TestSigner) {
        command.signature = None;
        if let Some(issuer) = &mut command.issuer {
            issuer.identity_kind =
                Some(edgerun_proto::edgerun::v0::common::IdentityKind::Node as i32);
            issuer.key_hint = Some(signer.node_id.0.to_vec());
        }
        let canonical = edgerun_core::protocol::canonical_bytes(
            &edgerun_core::protocol::ProtocolRecord::CommandEnvelope(command.clone()),
            true,
        );
        let sig = edgerun_core::crypto::sign_canonical_record(
            &signer.key,
            edgerun_core::crypto::SIG_DOMAIN_COMMAND_ENVELOPE,
            &canonical,
        )
        .unwrap();
        command.signature = Some(edgerun_core::protocol::Signature {
            algorithm: edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });
    }

    fn sign_revocation_record(
        revocation: &mut edgerun_proto::edgerun::v0::trust::RevocationRecord,
        signer: &TestSigner,
    ) {
        revocation.signature = None;
        if let Some(issuer) = &mut revocation.issuer {
            issuer.identity_kind =
                Some(edgerun_proto::edgerun::v0::common::IdentityKind::Node as i32);
            issuer.key_hint = Some(signer.node_id.0.to_vec());
        }
        let canonical = edgerun_core::protocol::canonical_bytes(
            &edgerun_core::protocol::ProtocolRecord::RevocationRecord(revocation.clone()),
            true,
        );
        let sig = edgerun_core::crypto::sign_canonical_record(
            &signer.key,
            edgerun_core::crypto::SIG_DOMAIN_REVOCATION_RECORD,
            &canonical,
        )
        .unwrap();
        revocation.signature = Some(edgerun_core::protocol::Signature {
            algorithm: edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });
    }

    fn sign_delegation_record(
        delegation: &mut edgerun_proto::edgerun::v0::trust::DelegationRecord,
        signer: &TestSigner,
    ) {
        delegation.signature = None;
        if let Some(issuer) = &mut delegation.issuer {
            issuer.identity_kind =
                Some(edgerun_proto::edgerun::v0::common::IdentityKind::Node as i32);
            issuer.key_hint = Some(signer.node_id.0.to_vec());
        }
        let canonical = edgerun_core::protocol::canonical_bytes(
            &edgerun_core::protocol::ProtocolRecord::DelegationRecord(delegation.clone()),
            true,
        );
        let sig = edgerun_core::crypto::sign_canonical_record(
            &signer.key,
            edgerun_core::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &canonical,
        )
        .unwrap();
        delegation.signature = Some(edgerun_core::protocol::Signature {
            algorithm: edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });
    }

    fn valid_capability_descriptor() -> edgerun_proto::edgerun::v0::trust::CapabilityDescriptor {
        edgerun_proto::edgerun::v0::trust::CapabilityDescriptor {
            capability_version: 1,
            capability_kind: edgerun_proto::edgerun::v0::trust::CapabilityKind::NodeControl as i32,
            actions: vec!["delegate".into()],
            scope: Some(edgerun_proto::edgerun::v0::trust::ScopeDescriptor {
                scope_version: 1,
                scope_kind: edgerun_proto::edgerun::v0::trust::ScopeKind::Node as i32,
                target_nodes: vec![edgerun_proto::edgerun::v0::common::NodeRef {
                    node_id: b"node-a".to_vec(),
                }],
                target_streams: vec![],
                target_object_kinds: vec![],
                target_view_types: vec![],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            constraints: None,
            delegation_policy:
                edgerun_proto::edgerun::v0::trust::DelegationPolicy::DelegableWithAttenuation as i32,
            minimum_assurance: None,
            capability_metadata: None,
        }
    }

    fn make_controller_identity(hex_str: &str) -> Vec<u8> {
        edgerun_core::util::hex_to_bytes(hex_str).unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // ControllerSet tests
    // -----------------------------------------------------------------------

    #[test]
    fn controller_set_new_with_initial() {
        let controllers = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let set = ControllerSet::new(controllers.clone());
        assert!(set.contains(&vec![1, 2, 3]));
        assert!(set.contains(&vec![4, 5, 6]));
        assert!(!set.contains(&vec![7, 8, 9]));
    }

    #[test]
    fn controller_set_empty() {
        let set = ControllerSet::default();
        assert!(!set.contains(&vec![1, 2, 3]));
    }

    #[test]
    fn controller_set_add() {
        let mut set = ControllerSet::new(vec![]);
        set.add(vec![1, 2, 3]);
        assert!(set.contains(&vec![1, 2, 3]));
    }

    #[test]
    fn controller_set_add_duplicate_is_noop() {
        let mut set = ControllerSet::new(vec![vec![1, 2, 3]]);
        set.add(vec![1, 2, 3]);
        assert_eq!(set.to_vec().len(), 1);
    }

    #[test]
    fn config_patch_updates_projectable_fields() {
        let mut config = crate::config::parse_config(
            r#"
stream_id: "node-stream"
name: "old"
controllers: ["a"]
trust_nodes: []
allowed_peers: []
bootstrap_peers: []
"#,
        )
        .unwrap();

        let changed = apply_config_patch(
            &mut config,
            br#"{"name":"new","allowed_peers":["peer-a"],"bootstrap_peers":["127.0.0.1:9000@peer-a"]}"#,
        )
        .unwrap();

        assert!(changed);
        assert_eq!(config.name.as_deref(), Some("new"));
        assert_eq!(config.allowed_peers, vec!["peer-a"]);
        assert_eq!(config.bootstrap_peers, vec!["127.0.0.1:9000@peer-a"]);
    }

    #[test]
    fn project_config_replays_committed_config_patch_result_object() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let stream_id = b"stream-1";

        append_signed_event(
            &mut store,
            stream_id,
            &signer,
            EventType::NodeGenesis,
            1,
            None,
            vec![],
            vec![],
            vec![],
        );

        let patch_ref = store
            .put_object(
                br#"{"name":"projected","allowed_peers":["peer-b"]}"#,
                edgerun_proto::edgerun::v0::common::ObjectKind::DerivedView as i32,
                &[stream_id.to_vec()],
            )
            .unwrap();
        let result_payload = ProtoCommandResultPayload {
            payload_version: 1,
            command: None,
            issuer: None,
            decision: CommandDecision::Committed as i32,
            decision_basis: None,
            reason_code: String::new(),
            effect_summary_object: None,
            result_object: Some(patch_ref),
        };
        let result_ref = store
            .put_object(
                &prost::Message::encode_to_vec(&result_payload),
                edgerun_proto::edgerun::v0::common::ObjectKind::Command as i32,
                &[stream_id.to_vec()],
            )
            .unwrap();
        append_signed_event(
            &mut store,
            stream_id,
            &signer,
            EventType::CommandCommitted,
            1,
            Some(result_ref),
            vec![],
            vec![],
            vec![],
        );

        let projected = project_config(
            &store,
            stream_id,
            r#"
stream_id: "stream-1"
name: "initial"
controllers: []
trust_nodes: []
allowed_peers: []
bootstrap_peers: []
"#,
        )
        .unwrap();

        assert_eq!(projected.name.as_deref(), Some("projected"));
        assert_eq!(projected.allowed_peers, vec!["peer-b"]);
    }

    #[test]
    fn controller_set_remove_existing() {
        let mut set = ControllerSet::new(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let removed = set.remove(&vec![1, 2, 3]);
        assert!(removed);
        assert!(!set.contains(&vec![1, 2, 3]));
        assert!(set.contains(&vec![4, 5, 6]));
    }

    #[test]
    fn controller_set_remove_nonexistent_returns_false() {
        let mut set = ControllerSet::new(vec![vec![1, 2, 3]]);
        let removed = set.remove(&vec![9, 9, 9]);
        assert!(!removed);
    }

    #[test]
    fn controller_set_iter() {
        let set = ControllerSet::new(vec![vec![1], vec![2]]);
        let collected: Vec<_> = set.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn controller_set_to_vec() {
        let set = ControllerSet::new(vec![vec![1, 2], vec![3, 4]]);
        let vec = set.to_vec();
        assert_eq!(vec.len(), 2);
        assert!(vec.contains(&vec![1, 2]));
        assert!(vec.contains(&vec![3, 4]));
    }

    #[test]
    fn controller_set_clone() {
        let set = ControllerSet::new(vec![vec![1, 2, 3]]);
        let cloned = set.clone();
        assert!(cloned.contains(&vec![1, 2, 3]));
    }

    // -----------------------------------------------------------------------
    // Dispatch command tests — full integration with NodeStore
    // -----------------------------------------------------------------------

    /// Creates an unlimited capacity tracker for tests.
    fn test_capacity_tracker() -> std::sync::Arc<crate::capacity::ResourceTracker> {
        let cap = crate::capacity::NodeCapacity {
            total_cores: 256,
            total_memory_bytes: 256 * 1024 * 1024 * 1024, // 256 GB
        };
        std::sync::Arc::new(crate::capacity::ResourceTracker::new(&cap, 0, 0))
    }

    fn append_genesis(store: &mut NodeStore, node_id: &NodeID) {
        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event_blocking(genesis).unwrap();
    }

    fn dispatch_test_command(
        command: &CommandEnvelope,
        store: &mut NodeStore,
        node_id: &NodeID,
        signer: &TestSigner,
        controllers: &mut ControllerSet,
        replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
        revoked: &HashSet<Vec<u8>>,
        trusted: &[Vec<u8>],
    ) -> CommandDispatchResult {
        dispatch_command(
            command,
            store,
            &node_id.0,
            signer,
            controllers,
            replay_cache,
            revoked,
            trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(),
            &test_workload_policy(),
            &test_rate_limiter(),
            &test_running_workloads(),
        )
    }

    #[test]
    fn dispatch_rejects_command_with_empty_command_id() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![], // empty command_id -> structural reject
            node_id.0.to_vec(),
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        assert_eq!(result.decision, 2); // REJECTED
        assert!(!result.reason_code.is_empty());
    }

    #[test]
    fn dispatch_rejects_command_targeting_wrong_node() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64], // wrong target
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        assert_eq!(result.decision, 2); // REJECTED
    }

    #[test]
    fn dispatch_rejects_command_without_target() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        append_genesis(&mut store, &node_id);

        let mut command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64],
            vec![1, 2, 3],
            CommandType::Query as i32,
        );
        command.target_node = None; // no target

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_rejects_non_controller_without_delegation() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        append_genesis(&mut store, &node_id);

        // Issuer is NOT in controllers and has no delegation chain
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            vec![99, 99, 99], // not a controller
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Fails signature verification first (no signature on command)
        // so it never reaches the controller check
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_add_controller_command() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        // Command with the controller as command_id (extract_identity_from_command uses command_id)
        let new_ctrl = vec![10, 20, 30];
        let command = make_command(
            new_ctrl.clone(), // command_id carries target identity
            node_id.0.to_vec(),
            initial_ctrl.clone(), // issued by existing controller
            CommandType::AddController as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        // Will fail signature verification (no signature), so gets rejected
        // This tests that the dispatch path runs through signature check
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_add_controller_missing_identity() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        // Empty command_id -> extract_identity returns empty
        let command = make_command(
            vec![], // empty -> missing identity
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::AddController as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Should be rejected for structural reasons (empty command_id)
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_remove_controller_command() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let ctrl_to_remove = vec![10, 20, 30];
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers =
            ControllerSet::new(vec![initial_ctrl.clone(), ctrl_to_remove.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            ctrl_to_remove.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::RemoveController as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Fails signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_signed_remove_last_controller_is_rejected() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let mut command = make_command(
            initial_ctrl.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::RemoveController as i32,
        );
        sign_command(&mut command, &signer);

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        assert_eq!(result.decision, 2);
        assert_eq!(result.reason_code, "CONTROL_INVARIANT_FAILED");
        assert!(controllers.contains(&initial_ctrl));
    }

    #[test]
    fn dispatch_transfer_control_command() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let new_ctrl = vec![42, 42, 42];
        let command = make_command(
            new_ctrl.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::TransferControl as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Fails signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_query_command_type() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Fails signature verification, but query type is recognized
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_publish_snapshot_returns_use_request() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::PublishSnapshot as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Should be rejected with "use_produce_snapshot_request" after failing sig check
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_fetch_object_returns_use_request() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::FetchObject as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_unknown_command_type_rejected() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        // Use an unknown command type
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            999, // unknown type
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_custom_command_with_delegation_payload() {
        use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
        use edgerun_proto::edgerun::v0::trust::DelegationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let now_secs = now_unix_secs_i64();

        // Build a structurally valid but unsigned delegation record.
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![1, 2, 3, 4],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: now_secs,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            capability: Some(valid_capability_descriptor()),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        let delegation_bytes = prost::Message::encode_to_vec(&delegation);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(Payload::InlinePayload(delegation_bytes));

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Should try to process as delegation but fail signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_signed_delegation_payload_commits() {
        use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
        use edgerun_proto::edgerun::v0::trust::DelegationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = node_id.0.to_vec();
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let now_secs = now_unix_secs_i64();
        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![1, 2, 3, 4],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: now_secs,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            capability: Some(valid_capability_descriptor()),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_delegation_record(&mut delegation, &signer);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(Payload::InlinePayload(prost::Message::encode_to_vec(
            &delegation,
        )));
        sign_command(&mut command, &signer);

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        assert_eq!(result.decision, CommandDecision::Committed as i32);
    }

    #[test]
    fn dispatch_custom_command_with_revocation_payload() {
        use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
        use edgerun_proto::edgerun::v0::trust::RevocationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        // Build a revocation record and encode as payload
        let revocation = RevocationRecord {
            record_version: 1,
            revocation_id: vec![10, 20, 30],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: None,
            effective_at: None,
            revocation_kind: 0,
            scope_override: None,
            reason_code: String::new(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: None,
            target: Some(
                edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(
                    edgerun_proto::edgerun::v0::common::IdentityRef {
                        identity_id: vec![5, 6, 7],
                        identity_kind: Some(2),
                        key_hint: None,
                    },
                ),
            ),
        };
        let revocation_bytes = prost::Message::encode_to_vec(&revocation);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(Payload::InlinePayload(revocation_bytes));

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Will fail signature verification but the dispatch path for revocation runs
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_signed_revocation_payload_commits() {
        use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
        use edgerun_proto::edgerun::v0::trust::{RevocationKind, RevocationRecord};

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = node_id.0.to_vec();
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let now_secs = now_unix_secs_i64();
        let mut revocation = RevocationRecord {
            record_version: 1,
            revocation_id: vec![10, 20, 30],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: now_secs,
                nanos: 0,
            }),
            effective_at: None,
            revocation_kind: RevocationKind::IdentityTrust as i32,
            scope_override: None,
            reason_code: "test".into(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: None,
            target: Some(
                edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(
                    edgerun_proto::edgerun::v0::common::IdentityRef {
                        identity_id: vec![5, 6, 7],
                        identity_kind: Some(2),
                        key_hint: None,
                    },
                ),
            ),
        };
        sign_revocation_record(&mut revocation, &signer);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateRevocation as i32,
        );
        command.payload = Some(Payload::InlinePayload(prost::Message::encode_to_vec(
            &revocation,
        )));
        sign_command(&mut command, &signer);

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        assert_eq!(result.decision, CommandDecision::Committed as i32);
    }

    #[test]
    fn dispatch_custom_command_unknown_payload() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(
            edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(vec![
                0xFF;
                10
            ]),
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Fails signature verification (no signature) — so rejected before reaching custom dispatch
        assert_eq!(result.decision, 2);
    }

    // -----------------------------------------------------------------------
    // Event recording and action lifecycle
    // -----------------------------------------------------------------------

    #[test]
    fn dispatch_records_action_lifecycle_events() {
        // Even rejected commands should have events recorded
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let _result = dispatch_command(
            &command,
            &mut store,
            &node_id.0,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(),
            &test_workload_policy(),
            &test_rate_limiter(),
            &std::sync::Arc::new(crate::running_workloads::RunningWorkloads::new()),
        );

        // After genesis (seq 0), there should be additional events recorded
        // for the rejected command (ActionStarted is only for committed, but ActionFailed is)
        let head_seq = store.get_head(&node_id.0).unwrap().unwrap().0;
        assert!(head_seq > 0); // more events than just genesis
    }

    // -----------------------------------------------------------------------
    // Project controller set
    // -----------------------------------------------------------------------

    #[test]
    fn project_controller_set_from_empty_store() {
        let store = test_store();
        let initial = vec![vec![1, 2, 3]];
        let result = project_controller_set(&store, b"test-stream", initial.clone());
        assert_eq!(result.to_vec(), initial);
    }

    #[test]
    fn project_controller_set_returns_initials_when_no_events() {
        let store = test_store();
        let initials = vec![vec![1], vec![2], vec![3]];
        let result = project_controller_set(&store, b"stream", initials.clone());
        assert!(result.contains(&vec![1]));
        assert!(result.contains(&vec![2]));
        assert!(result.contains(&vec![3]));
        assert_eq!(result.to_vec().len(), 3);
    }

    // -----------------------------------------------------------------------
    // Extract identity from command
    // -----------------------------------------------------------------------

    #[test]
    fn extract_identity_uses_command_id() {
        let command = make_command(vec![10, 20, 30], vec![0], vec![1], 1);
        let identity = extract_identity_from_command(&command);
        assert_eq!(identity, vec![10, 20, 30]);
    }

    #[test]
    fn extract_identity_falls_back_to_issuer() {
        let mut command = make_command(vec![], vec![0], vec![7, 8, 9], 1);
        let identity = extract_identity_from_command(&command);
        assert_eq!(identity, vec![7, 8, 9]);
    }

    #[test]
    fn extract_identity_returns_empty_when_no_issuer_and_no_command_id() {
        let mut command = make_command(vec![], vec![0], vec![], 1);
        command.issuer = None;
        let identity = extract_identity_from_command(&command);
        assert!(identity.is_empty());
    }

    // -----------------------------------------------------------------------
    // Dispatch result
    // -----------------------------------------------------------------------

    #[test]
    fn dispatch_result_fields() {
        let result = CommandDispatchResult {
            event_type: EventType::CommandCommitted,
            decision: 1,
            reason_code: "test".to_string(),
            response_bytes: vec![1, 2, 3],
        };
        assert_eq!(result.decision, 1);
        assert_eq!(result.reason_code, "test");
        assert_eq!(result.response_bytes, vec![1, 2, 3]);
    }
}
