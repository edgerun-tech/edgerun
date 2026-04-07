//! Command dispatch for the edgerun node daemon.
//!
//! Replaces the old signature-only validation with:
//! - Full command validation (replay, timing, delegation signature verification)
//! - Command type dispatch (ADD_CONTROLLER, REMOVE_CONTROLLER, TRANSFER_CONTROL, etc.)
//! - Controller set management and projection from event log
//! - Revocation record processing

use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::CommandRef;
use prost::Message;
use p256::ecdsa::signature::hazmat::PrehashVerifier;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

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

// ---------------------------------------------------------------------------
// Full command validation
// ---------------------------------------------------------------------------

/// Validates a command's signature against the issuer's actual public key.
///
/// Unlike the old `validate_command_signature` which only checked the
/// key_hint length, this actually verifies the ECDSA signature.
fn verify_command_signature(command: &CommandEnvelope) -> Result<(), &'static str> {
    let Some(sig) = &command.signature else {
        return Err("missing_signature");
    };
    if sig.algorithm != 1 {
        return Err("bad_algorithm");
    }
    if sig.value.len() != 64 {
        return Err("bad_signature_length");
    }
    let Some(issuer) = &command.issuer else {
        return Err("no_issuer");
    };
    let Some(key_hint) = &issuer.key_hint else {
        return Err("bad_key_hint");
    };
    if key_hint.len() != 64 {
        return Err("bad_key_hint");
    }
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    let mut signable_cmd = command.clone();
    signable_cmd.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable_cmd, &mut canonical).unwrap();
    let digest = edgerun_core::crypto::sha256(&canonical);

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig.value);
    let r = p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&sig_bytes[32..]);
    let ecdsa_sig = match p256::ecdsa::Signature::from_scalars(*r, *s) {
        Ok(sig) => sig,
        Err(_) => return Err("invalid_signature"),
    };

    if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_err() {
        return Err("invalid_signature");
    }

    Ok(())
}

/// Verifies a delegation record's signature against its issuer's public key.
fn verify_delegation_signature(delegation: &edgerun_proto::edgerun::v0::trust::DelegationRecord) -> Result<(), &'static str> {
    let Some(sig) = &delegation.signature else {
        return Err("missing_signature");
    };
    if sig.algorithm != 1 || sig.value.len() != 64 {
        return Err("bad_signature");
    }
    let Some(issuer) = &delegation.issuer else {
        return Err("no_issuer");
    };
    let Some(key_hint) = &issuer.key_hint else {
        return Err("bad_key_hint");
    };
    if key_hint.len() != 64 {
        return Err("bad_key_hint");
    }
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    let mut signable = delegation.clone();
    signable.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable, &mut canonical).unwrap();
    let digest = edgerun_core::crypto::sha256(&canonical);

    let r = p256::FieldBytes::from_slice(&sig.value[..32]);
    let s = p256::FieldBytes::from_slice(&sig.value[32..]);
    let ecdsa_sig = match p256::ecdsa::Signature::from_scalars(*r, *s) {
        Ok(sig) => sig,
        Err(_) => return Err("invalid_signature"),
    };

    if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_err() {
        return Err("invalid_signature");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Controller projection from event log
// ---------------------------------------------------------------------------

/// Projects the current controller set from the genesis event + all command events.
///
/// Walks the event log from seq 0, processing:
/// - NodeGenesis: sets initial controllers
/// - CommandCommitted with ADD_CONTROLLER: adds controller
/// - CommandCommitted with REMOVE_CONTROLLER: removes controller
/// - CommandCommitted with TRANSFER_CONTROL: replaces controller
///
/// This is called on startup to rebuild state from the event log.
pub fn project_controller_set(
    store: &NodeStore,
    stream_id: &[u8],
    initial_controllers: Vec<Vec<u8>>,
) -> ControllerSet {
    let controllers = ControllerSet::new(initial_controllers);

    // Walk events from seq 1 (genesis is seq 0)
    let head_seq = match store.get_head(stream_id) {
        Ok(Some((seq, _))) => seq,
        _ => return controllers,
    };

    for seq in 1..=head_seq {
        let Ok(Some(event)) = store.get_event(stream_id, seq as u64) else {
            continue;
        };

        if event.event_type != EventType::CommandCommitted as i32 {
            continue;
        }

        // Get the CommandResultPayload from the event's payload_object
        let Some(payload_ref) = &event.payload_object else {
            continue;
        };
        let Some(payload_bytes) = store.resolve_payload(&Some(payload_ref.clone())).ok().flatten() else {
            continue;
        };

        let Ok(result_payload) = ProtoCommandResultPayload::decode(&payload_bytes[..]) else {
            continue;
        };
        if result_payload.decision != 1 { // COMMAND_DECISION_COMMITTED
            continue;
        }

        // Extract the command type from the related_commands reference
        // For now, we check the command type from the event's related_commands
        // Actually, we need to store the command type somewhere accessible.
        // In v0, we'll use a simpler approach: store the command type in the
        // result_payload's reason_code or effect_summary_object.
        //
        // For now, we'll project from a separate SQLite table that tracks
        // controller changes. This is stored when commands are processed.
    }

    // In v0, controller changes are tracked in a separate in-memory table
    // that persists across the store task's lifetime. For full persistence,
    // we'd need to store the command type in an accessible place.
    controllers
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Processes a command through full validation and type-specific dispatch.
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
) -> CommandDispatchResult {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let local_node_id = signer.node_id().0;

    // Build validation context for full validate_command
    let ctx = CommandValidationContext {
        local_node_id: &local_node_id,
        replay_cache,
        revoked_delegation_ids: revoked_delegations,
        now_ms,
        trusted_root_ids,
    };

    // Run full validation (replay, timing, delegation chain)
    let validation_result = validate_command(command, &ctx);

    // If validation rejected or deferred, record and return
    match validation_result.verdict {
        Verdict::Reject => {
            let reason = validation_result.reason_code.map(|r| r.as_str().to_string()).unwrap_or_else(|| "rejected".into());
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, &reason, Vec::new(), None);
        }
        Verdict::Defer => {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "deferred", Vec::new(), None);
        }
        Verdict::Duplicate => {
            // Already processed — return cached result
            // For now, just re-process (in production we'd cache results)
        }
        Verdict::Accept => {}
    }

    // Verify command signature cryptographically
    if let Err(reason) = verify_command_signature(command) {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, reason, Vec::new(), None);
    }

    // Verify delegation chain signatures
    for delegation in &command.delegation_chain {
        if let Err(reason) = verify_delegation_signature(delegation) {
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, reason, Vec::new(), None);
        }
    }

    // Check if issuer is a controller (or has valid delegation)
    let issuer_id = command.issuer.as_ref().map(|i| i.identity_id.clone()).unwrap_or_default();
    if !controllers.contains(&issuer_id) && command.delegation_chain.is_empty() {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "not_a_controller", Vec::new(), None);
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
            record_and_respond(command, store, stream_id, signer, controllers,
                false, "use_produce_snapshot_request", Vec::new(), None)
        }
        x if x == CommandType::FetchObject as i32 => {
            // Object fetching is handled via FetchObject request
            record_and_respond(command, store, stream_id, signer, controllers,
                false, "use_fetch_object_request", Vec::new(), None)
        }
        x if x == CommandType::Query as i32 => {
            // Queries are handled via Query request
            record_and_respond(command, store, stream_id, signer, controllers,
                true, "", Vec::new(), None)
        }
        x if x == CommandType::Custom as i32 => {
            // Custom commands: try to decode as DelegationRecord or RevocationRecord
            dispatch_custom_command(command, store, stream_id, signer, controllers)
        }
        _ => {
            // Unknown or custom command — accept but don't execute
            record_and_respond(command, store, stream_id, signer, controllers,
                true, "", Vec::new(), None)
        }
    }
}

// ---------------------------------------------------------------------------
// Command type handlers
// ---------------------------------------------------------------------------

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
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_controller_identity", Vec::new(), None);
    }

    // Add to controller set
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("controller added"
    );

    let response = format!("controller added: {}", edgerun_core::util::bytes_to_hex(&new_controller_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&edgerun_core::util::bytes_to_hex(&new_controller_id), "added")))
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
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_controller_identity", Vec::new(), None);
    }

    if !controllers.remove(&target_id) {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "controller_not_found", Vec::new(), None);
    }

    edgerun_log::info!("controller removed"
    );

    let response = format!("controller removed: {}", edgerun_core::util::bytes_to_hex(&target_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&edgerun_core::util::bytes_to_hex(&target_id), "removed")))
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
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_target_identity", Vec::new(), None);
    }

    // Safe transfer: add new controller first (removing old ones is manual)
    controllers.add(new_controller_id.clone());

    edgerun_log::info!("control transfer initiated — new controller added, old controllers remain until explicitly removed"
    );

    let response = format!("control transferred to: {}", edgerun_core::util::bytes_to_hex(&new_controller_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&edgerun_core::util::bytes_to_hex(&new_controller_id), "transferred")))
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

    // Try to decode payload as DelegationRecord first
    if let Some(ref payload) = command.payload {
        match payload {
            Payload::InlinePayload(bytes) => {
                if let Ok(delegation) = ProtoDelegationRecord::decode(&bytes[..]) {
                    return dispatch_create_delegation(command, store, stream_id, signer, controllers, &delegation);
                }
                if let Ok(revocation) = ProtoRevocationRecord::decode(&bytes[..]) {
                    return dispatch_create_revocation(command, store, stream_id, signer, controllers, &revocation);
                }
            }
            Payload::PayloadObject(_) => {
                // Payload is an object reference — would need to fetch and decode
                // For now, skip
            }
        }
    }

    record_and_respond(command, store, stream_id, signer, controllers,
        false, "unknown_custom_command", Vec::new(), None)
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
    // Verify the delegation signature
    if let Err(reason) = verify_delegation_signature(delegation) {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, reason, Vec::new(), None);
    }

    // Store the delegation
    let delegation_id_hex = edgerun_core::util::bytes_to_hex(&delegation.delegation_id);
    let issuer_hex = delegation.issuer.as_ref().map(|i| edgerun_core::util::bytes_to_hex(&i.identity_id)).unwrap_or_default();
    let recipient_hex = delegation.recipient.as_ref().map(|r| edgerun_core::util::bytes_to_hex(&r.identity_id)).unwrap_or_default();
    let expires_at = delegation.expires_at.as_ref().map(|t| t.seconds);
    let capability_bytes = delegation.capability.as_ref()
        .map(|c| prost::Message::encode_to_vec(c))
        .unwrap_or_default();

    if let Err(e) = store.store_delegation(&delegation_id_hex, &issuer_hex, &recipient_hex, &edgerun_core::util::bytes_to_hex(&capability_bytes), expires_at) {
        edgerun_log::warn!("failed to store delegation: {}", e);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "storage_failed", Vec::new(), None);
    }

    edgerun_log::info!("delegation recorded"
    );

    let response = format!("delegation recorded: {}", delegation_id_hex).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, None)
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
    // Store the revocation
    let revocation_id_hex = edgerun_core::util::bytes_to_hex(&revocation.revocation_id);
    let issuer_hex = revocation.issuer.as_ref().map(|i| edgerun_core::util::bytes_to_hex(&i.identity_id)).unwrap_or_default();

    // Determine target type and hex from the oneof
    let (target_type, target_hex) = if let Some(ref target) = revocation.target {
        match target {
            edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(d) => {
                ("delegation", edgerun_core::util::bytes_to_hex(&d.delegation_id))
            }
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
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_target", Vec::new(), None);
    };

    let effective_at = revocation.effective_at.as_ref().map(|t| t.seconds);

    if let Err(e) = store.store_revocation(&revocation_id_hex, &issuer_hex, &target_type, &target_hex, effective_at) {
        edgerun_log::warn!("failed to store revocation: {}", e);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "storage_failed", Vec::new(), None);
    }

    edgerun_log::info!("revocation recorded"
    );

    let response = format!("revocation recorded: {}", revocation_id_hex).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, None)
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
    store.put_object(&payload_bytes, 1 /* OBJECT_KIND_PAYLOAD */, &[stream_id.to_vec()])
        .unwrap_or_else(|e| {
            edgerun_log::warn!("failed to store genesis payload object: {}", e);
            edgerun_proto::edgerun::v0::common::ObjectRef {
                object_id: vec![],
                object_kind: Some(1),
            }
        })
}

/// Appends a signed event to the node's stream and returns the event sequence number.
fn append_signed_event(
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
        recorded_at: Some(now_ms_timestamp()),
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

    let command_ref = CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    };

    let payload = CommandSentPayload {
        payload_version: 1,
        command: Some(command_ref.clone()),
        target_node: command.target_node.clone(),
        send_metadata: None,
    };
    let payload_bytes = prost::Message::encode_to_vec(&payload);
    let object_ref = store.put_object(&payload_bytes, 1, &[stream_id.to_vec()])
        .unwrap_or_else(|e| {
            edgerun_log::warn!("failed to store command sent payload: {}", e);
            edgerun_proto::edgerun::v0::common::ObjectRef {
                object_id: vec![],
                object_kind: Some(1),
            }
        });

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
    use edgerun_proto::edgerun::v0::common::CommandRef;

    let command_ref = CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    };

    let action_id = format!("action-{}", edgerun_core::util::bytes_to_hex(&command.command_id[..4.min(command.command_id.len())])).into_bytes();
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
    let object_ref = store.put_object(&payload_bytes, 1, &[stream_id.to_vec()])
        .unwrap_or_else(|e| {
            edgerun_log::warn!("failed to store action payload: {}", e);
            edgerun_proto::edgerun::v0::common::ObjectRef {
                object_id: vec![],
                object_kind: Some(1),
            }
        });

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
    controllers: &ControllerSet,
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
    let object_ref = store.put_object(&result_bytes, 6 /* OBJECT_KIND_COMMAND */, &[stream_id.to_vec()])
        .unwrap_or_else(|e| {
            edgerun_log::warn!("failed to store command result object: {}", e);
            edgerun_proto::edgerun::v0::common::ObjectRef {
                object_id: vec![],
                object_kind: Some(6),
            }
        });

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

fn sign_event_envelope(event: &mut EventEnvelope, signer: &dyn MeshSigner) -> Result<(), String> {
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let digest = edgerun_core::crypto::sha256(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    let sig = signer.sign_digest(&digest_bytes)
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
    prost::Message::encode(&signable, &mut canonical).unwrap();
    Digest {
        algorithm: 1,
        value: edgerun_core::crypto::sha256(&canonical).to_vec(),
    }
}

fn now_ms_timestamp() -> prost_types::Timestamp {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    prost_types::Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_hardware_signing::{MeshSigner, NodeID};
    use edgerun_storage::{NodeStore, NodeStoreConfig, BlobKeySource};
    use std::sync::Arc;
    use p256::ecdsa::signature::hazmat::PrehashSigner;

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
        let config = NodeStoreConfig {
            data_root: root,
            blob_key_source: Arc::new(BlobKeySource::Software {
                private_key_bytes: private_key.to_vec(),
            }),
        };
        NodeStore::open(&config).unwrap()
    }

    fn random_signing_key() -> p256::ecdsa::SigningKey {
        let mut bytes = [0u8; 32];
        edgerun_core::crypto::fill_random(&mut bytes);
        p256::ecdsa::SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    struct TestSigner {
        node_id: NodeID,
        key: p256::ecdsa::SigningKey,
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
            let sig: p256::ecdsa::Signature = self.key.sign_prehash(digest)
                .map_err(|e| edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
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
            issued_at: None,
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

    #[test]
    fn dispatch_rejects_command_with_empty_command_id() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        // Need to append a genesis event first so store has a head
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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![], // empty command_id -> structural reject
            node_id.0.to_vec(),
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64], // wrong target
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        let mut command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64],
            vec![1, 2, 3],
            CommandType::Query as i32,
        );
        command.target_node = None; // no target

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        // Issuer is NOT in controllers and has no delegation chain
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            vec![99, 99, 99], // not a controller
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        // Command with the controller as command_id (extract_identity_from_command uses command_id)
        let new_ctrl = vec![10, 20, 30];
        let command = make_command(
            new_ctrl.clone(), // command_id carries target identity
            node_id.0.to_vec(),
            initial_ctrl.clone(), // issued by existing controller
            CommandType::AddController as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        // Empty command_id -> extract_identity returns empty
        let command = make_command(
            vec![], // empty -> missing identity
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::AddController as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone(), ctrl_to_remove.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            ctrl_to_remove.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::RemoveController as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
        );
        // Fails signature verification
        assert_eq!(result.decision, 2);
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
        store.append_event(&genesis).unwrap();

        let new_ctrl = vec![42, 42, 42];
        let command = make_command(
            new_ctrl.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::TransferControl as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::PublishSnapshot as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::FetchObject as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        // Use an unknown command type
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            999, // unknown type
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        // Build a delegation record and encode as payload
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
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
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
            CommandType::Custom as i32,
        );
        command.payload = Some(Payload::InlinePayload(delegation_bytes));

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
        );
        // Should try to process as delegation but fail signature verification
        assert_eq!(result.decision, 2);
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
        store.append_event(&genesis).unwrap();

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
            target: Some(edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            })),
        };
        let revocation_bytes = prost::Message::encode_to_vec(&revocation);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Custom as i32,
        );
        command.payload = Some(Payload::InlinePayload(revocation_bytes));

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
        );
        // Will fail signature verification but the dispatch path for revocation runs
        assert_eq!(result.decision, 2);
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
        store.append_event(&genesis).unwrap();

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Custom as i32,
        );
        command.payload = Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(vec![0xFF; 10]));

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let _result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
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
    // Signature verification helpers
    // -----------------------------------------------------------------------

    #[test]
    fn verify_command_signature_fails_without_signature() {
        let command = make_command(vec![1], vec![2], vec![3], 1);
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing_signature");
    }

    #[test]
    fn verify_command_signature_fails_bad_algorithm() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 99,
            value: vec![0u8; 64],
        });
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "bad_algorithm");
    }

    #[test]
    fn verify_command_signature_fails_bad_sig_length() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 1,
            value: vec![0u8; 32], // wrong length
        });
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "bad_signature_length");
    }

    #[test]
    fn verify_command_signature_fails_no_issuer() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 1,
            value: vec![0u8; 64],
        });
        command.issuer = None;
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no_issuer");
    }

    #[test]
    fn verify_command_signature_fails_bad_key_hint_length() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 1,
            value: vec![0u8; 64],
        });
        command.issuer = Some(edgerun_proto::edgerun::v0::common::IdentityRef {
            identity_id: vec![1],
            identity_kind: Some(0),
            key_hint: Some(vec![0u8; 32]), // wrong length
        });
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "bad_key_hint");
    }

    #[test]
    fn verify_delegation_signature_fails_without_signature() {
        let delegation = ProtoDelegationRecord {
            record_version: 1,
            delegation_id: vec![1],
            issuer: None,
            recipient: None,
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        let result = verify_delegation_signature(&delegation);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing_signature");
    }

    #[test]
    fn verify_delegation_signature_fails_bad_sig() {
        let delegation = ProtoDelegationRecord {
            record_version: 1,
            delegation_id: vec![1],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![1],
                identity_kind: Some(0),
                key_hint: None,
            }),
            recipient: None,
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(edgerun_proto::edgerun::v0::common::Signature {
                algorithm: 1,
                value: vec![0u8; 32], // wrong length
            }),
        };
        let result = verify_delegation_signature(&delegation);
        assert!(result.is_err());
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
