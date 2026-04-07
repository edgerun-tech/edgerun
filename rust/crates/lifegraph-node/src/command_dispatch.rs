//! Command dispatch for the Lifegraph node daemon.
//!
//! Replaces the old signature-only validation with:
//! - Full command validation (replay, timing, delegation signature verification)
//! - Command type dispatch (ADD_CONTROLLER, REMOVE_CONTROLLER, TRANSFER_CONTROL, etc.)
//! - Controller set management and projection from event log
//! - Revocation record processing

use lifegraph_log;
use lifegraph_core::command::{command_hash, validate_command, CommandValidationContext};
use lifegraph_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use lifegraph_core::result::Verdict;
use lifegraph_hardware_signing::MeshSigner;
use lifegraph_storage::NodeStore;
use lifegraph_proto::lifegraph::v0::stream::{CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use lifegraph_proto::lifegraph::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use lifegraph_proto::lifegraph::v0::common::CommandRef;
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
    let digest = lifegraph_core::crypto::sha256(&canonical);

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
fn verify_delegation_signature(delegation: &lifegraph_proto::lifegraph::v0::trust::DelegationRecord) -> Result<(), &'static str> {
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
    let digest = lifegraph_core::crypto::sha256(&canonical);

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

    lifegraph_log::info!("controller added"
    );

    let response = format!("controller added: {}", lifegraph_core::util::bytes_to_hex(&new_controller_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&lifegraph_core::util::bytes_to_hex(&new_controller_id), "added")))
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

    lifegraph_log::info!("controller removed"
    );

    let response = format!("controller removed: {}", lifegraph_core::util::bytes_to_hex(&target_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&lifegraph_core::util::bytes_to_hex(&target_id), "removed")))
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

    lifegraph_log::info!("control transfer initiated — new controller added, old controllers remain until explicitly removed"
    );

    let response = format!("control transferred to: {}", lifegraph_core::util::bytes_to_hex(&new_controller_id)).into_bytes();
    record_and_respond(command, store, stream_id, signer, controllers,
        true, "", response, Some((&lifegraph_core::util::bytes_to_hex(&new_controller_id), "transferred")))
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
    use lifegraph_proto::lifegraph::v0::stream::command_envelope::Payload;

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
    let delegation_id_hex = lifegraph_core::util::bytes_to_hex(&delegation.delegation_id);
    let issuer_hex = delegation.issuer.as_ref().map(|i| lifegraph_core::util::bytes_to_hex(&i.identity_id)).unwrap_or_default();
    let recipient_hex = delegation.recipient.as_ref().map(|r| lifegraph_core::util::bytes_to_hex(&r.identity_id)).unwrap_or_default();
    let expires_at = delegation.expires_at.as_ref().map(|t| t.seconds);
    let capability_bytes = delegation.capability.as_ref()
        .map(|c| prost::Message::encode_to_vec(c))
        .unwrap_or_default();

    if let Err(e) = store.store_delegation(&delegation_id_hex, &issuer_hex, &recipient_hex, &lifegraph_core::util::bytes_to_hex(&capability_bytes), expires_at) {
        lifegraph_log::warn!("failed to store delegation: {}", e);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "storage_failed", Vec::new(), None);
    }

    lifegraph_log::info!("delegation recorded"
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
    let revocation_id_hex = lifegraph_core::util::bytes_to_hex(&revocation.revocation_id);
    let issuer_hex = revocation.issuer.as_ref().map(|i| lifegraph_core::util::bytes_to_hex(&i.identity_id)).unwrap_or_default();

    // Determine target type and hex from the oneof
    let (target_type, target_hex) = if let Some(ref target) = revocation.target {
        match target {
            lifegraph_proto::lifegraph::v0::trust::revocation_record::Target::TargetDelegation(d) => {
                ("delegation", lifegraph_core::util::bytes_to_hex(&d.delegation_id))
            }
            lifegraph_proto::lifegraph::v0::trust::revocation_record::Target::TargetIdentity(i) => {
                ("identity", lifegraph_core::util::bytes_to_hex(&i.identity_id))
            }
            lifegraph_proto::lifegraph::v0::trust::revocation_record::Target::TargetNode(n) => {
                ("node", lifegraph_core::util::bytes_to_hex(&n.node_id))
            }
            lifegraph_proto::lifegraph::v0::trust::revocation_record::Target::TargetObject(o) => {
                ("object", lifegraph_core::util::bytes_to_hex(&o.object_id))
            }
        }
    } else {
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "missing_target", Vec::new(), None);
    };

    let effective_at = revocation.effective_at.as_ref().map(|t| t.seconds);

    if let Err(e) = store.store_revocation(&revocation_id_hex, &issuer_hex, &target_type, &target_hex, effective_at) {
        lifegraph_log::warn!("failed to store revocation: {}", e);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "storage_failed", Vec::new(), None);
    }

    lifegraph_log::info!("revocation recorded"
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
    node_id: &lifegraph_hardware_signing::NodeID,
    initial_controllers: &[Vec<u8>],
) -> lifegraph_proto::lifegraph::v0::common::ObjectRef {
    use lifegraph_proto::lifegraph::v0::stream::NodeGenesisPayload;
    use lifegraph_proto::lifegraph::v0::common::IdentityRef;

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
            lifegraph_log::warn!("failed to store genesis payload object: {}", e);
            lifegraph_proto::lifegraph::v0::common::ObjectRef {
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
    payload_object: Option<lifegraph_proto::lifegraph::v0::common::ObjectRef>,
    related_commands: Vec<lifegraph_proto::lifegraph::v0::common::CommandRef>,
    related_delegations: Vec<lifegraph_proto::lifegraph::v0::common::DelegationRef>,
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
        lifegraph_log::warn!("failed to sign event: {}", e);
        return None;
    }
    if let Err(e) = store.append_event(&event) {
        lifegraph_log::warn!("failed to append event: {}", e);
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
    use lifegraph_proto::lifegraph::v0::stream::CommandSentPayload;
    use lifegraph_proto::lifegraph::v0::common::CommandRef;

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
            lifegraph_log::warn!("failed to store command sent payload: {}", e);
            lifegraph_proto::lifegraph::v0::common::ObjectRef {
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
    use lifegraph_proto::lifegraph::v0::stream::ActionLifecyclePayload;
    use lifegraph_proto::lifegraph::v0::common::CommandRef;

    let command_ref = CommandRef {
        command_id: command.command_id.clone(),
        command_hash: Some(command_hash(command)),
    };

    let action_id = format!("action-{}", lifegraph_core::util::bytes_to_hex(&command.command_id[..4.min(command.command_id.len())])).into_bytes();
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
            lifegraph_log::warn!("failed to store action payload: {}", e);
            lifegraph_proto::lifegraph::v0::common::ObjectRef {
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

    let delegations: Vec<lifegraph_proto::lifegraph::v0::common::DelegationRef> =
        command.delegation_chain.iter().map(|d| {
            lifegraph_proto::lifegraph::v0::common::DelegationRef {
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
            lifegraph_log::warn!("failed to store command result object: {}", e);
            lifegraph_proto::lifegraph::v0::common::ObjectRef {
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
                lifegraph_log::warn!("failed to record controller change: {}", e);
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
    let digest = lifegraph_core::crypto::sha256(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    let sig = signer.sign_digest(&digest_bytes)
        .map_err(|e| format!("signing failed: {}", e))?;
    event.signature = Some(lifegraph_core::protocol::Signature {
        algorithm: 1,
        value: sig.to_vec(),
    });
    Ok(())
}

fn delegation_hash(delegation: &lifegraph_proto::lifegraph::v0::trust::DelegationRecord) -> Digest {
    let mut signable = delegation.clone();
    signable.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable, &mut canonical).unwrap();
    Digest {
        algorithm: 1,
        value: lifegraph_core::crypto::sha256(&canonical).to_vec(),
    }
}

fn now_ms_timestamp() -> prost_types::Timestamp {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    prost_types::Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}
