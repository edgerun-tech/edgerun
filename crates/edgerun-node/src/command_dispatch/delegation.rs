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

    // Store the delegation as an object so it's cryptographically linked
    // to the signed CommandCommitted event via payload_object
    let delegation_bytes = prost::Message::encode_to_vec(delegation);
    let object_ref = match store.put_object(&delegation_bytes, 0, &[]) {
        Ok(r) => r,
        Err(e) => {
            edgerun_log::warn!("failed to store delegation object: {}", e);
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "storage_failed", Vec::new(), None);
        }
    };

    // Also store in the index for efficient lookup
    let delegation_id_hex = edgerun_core::util::bytes_to_hex(&delegation.delegation_id);
    let issuer_hex = delegation.issuer.as_ref().map(|i| edgerun_core::util::bytes_to_hex(&i.identity_id)).unwrap_or_default();
    let recipient_hex = delegation.recipient.as_ref().map(|r| edgerun_core::util::bytes_to_hex(&r.identity_id)).unwrap_or_default();
    let expires_at = delegation.expires_at.as_ref().map(|t| t.seconds);
    let capability_bytes = delegation.capability.as_ref()
        .map(|c| prost::Message::encode_to_vec(c))
        .unwrap_or_default();

    if let Err(e) = store.store_delegation(&delegation_id_hex, &issuer_hex, &recipient_hex, &edgerun_core::util::bytes_to_hex(&capability_bytes), expires_at) {
        edgerun_log::warn!("failed to index delegation: {}", e);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "index_failed", Vec::new(), None);
    }

    edgerun_log::info!("delegation recorded");

    // Manually produce events with the delegation object as payload_object
    let command_ref = command_ref_from(command);
    let delegations = delegation_refs_from(command);

    record_action_event(store, stream_id, signer, command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted);

    let _event_seq = append_signed_event(
        store, stream_id, signer,
        EventType::CommandCommitted, 1,
        Some(object_ref),
        vec![command_ref],
        delegations,
    );

    record_action_event(store, stream_id, signer, command,
        2, // ACTION_STATUS_COMPLETED
        EventType::ActionCompleted);

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

    // Store the revocation as an object so it's cryptographically linked
    // to the signed CommandCommitted event via payload_object
    let revocation_bytes = prost::Message::encode_to_vec(revocation);
    let object_ref = match store.put_object(&revocation_bytes, 0, &[]) {
        Ok(r) => r,
        Err(e) => {
            edgerun_log::warn!("failed to store revocation object: {}", e);
            return record_and_respond(command, store, stream_id, signer, controllers,
                false, "storage_failed", Vec::new(), None);
        }
    };

    // Also store in the index for efficient lookup
    if let Err(e) = store.store_revocation(&revocation_id_hex, &issuer_hex, &target_type, &target_hex, effective_at) {
        edgerun_log::warn!("failed to index revocation: {}", e);
        return record_and_respond(command, store, stream_id, signer, controllers,
            false, "index_failed", Vec::new(), None);
    }

    edgerun_log::info!("revocation recorded");

    // Manually produce events with the revocation object as payload_object
    let command_ref = command_ref_from(command);
    let delegations = delegation_refs_from(command);

    record_action_event(store, stream_id, signer, command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted);

    let _event_seq = append_signed_event(
        store, stream_id, signer,
        EventType::CommandCommitted, 1,
        Some(object_ref),
        vec![command_ref],
        delegations,
    );

    record_action_event(store, stream_id, signer, command,
        2, // ACTION_STATUS_COMPLETED
        EventType::ActionCompleted);

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

