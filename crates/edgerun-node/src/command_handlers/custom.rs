//! Custom command handlers — delegation and revocation records.

use crate::command_dispatch::{
    command_ref_from, delegation_refs_from, record_and_respond, record_and_respond_with_result_object,
    store_object_or_log, CommandDispatchResult, ControllerSet,
};
use crate::command_dispatch_event::record_action_event;
use edgerun_core::result::Verdict;
use edgerun_core::util::bytes_to_hex;
use edgerun_core::validators_proto::validate_delegation_chain;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::stream::{CommandEnvelope, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{
    DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord,
};
use edgerun_storage::NodeStore;
use prost::Message;

/// Handles custom commands: delegation records and revocation records.
pub fn dispatch_custom_command(
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
pub fn dispatch_create_delegation(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    delegation: &ProtoDelegationRecord,
) -> CommandDispatchResult {
    let now_ms = edgerun_core::util::now_unix_millis_i64();
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
    let delegation_id_hex = bytes_to_hex(&delegation.delegation_id);
    let issuer_hex = delegation
        .issuer
        .as_ref()
        .map(|i| bytes_to_hex(&i.identity_id))
        .unwrap_or_default();
    let recipient_hex = delegation
        .recipient
        .as_ref()
        .map(|r| bytes_to_hex(&r.identity_id))
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
        &bytes_to_hex(&capability_bytes),
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

    // Reject if digest doesn't match (defensive)
    let object_ref_vec = match object_ref {
        edgerun_proto::edgerun::v0::common::ObjectRef { .. } => vec![object_ref],
        _ => vec![],
    };

    let event_seq = super::append_signed_event(
        store,
        stream_id,
        signer,
        EventType::CommandCommitted,
        1,
        if object_ref_vec.is_empty() {
            None
        } else {
            Some(object_ref_vec[0].clone())
        },
        vec![command_ref],
        delegations,
        vec![],
    );

    // Write replay cache entry
    if let Some(ref target) = command.target_node {
        let target_hex = bytes_to_hex(&target.node_id);
        let cmd_hash_hex = bytes_to_hex(&super::command_hash(command).value);
        if let Err(e) = store.put_replay_entry(
            &target_hex,
            &cmd_hash_hex,
            &bytes_to_hex(&command.command_id),
            event_seq.unwrap_or(0) as i64,
        ) {
            edgerun_log::warn!("failed to write replay cache entry: {}", e);
        }
    }

    // Emit action events
    record_action_event(
        store,
        stream_id,
        signer,
        command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted,
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

    let response = format!(
        "delegation recorded: {}",
        bytes_to_hex(&delegation.delegation_id)
    )
    .into_bytes();

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
        None,
    )
}

/// Handles a revocation creation command.
pub fn dispatch_create_revocation(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut ControllerSet,
    revocation: &ProtoRevocationRecord,
) -> CommandDispatchResult {
    // Validate the revocation record (signature, structure, timing)
    let now_ms = edgerun_core::util::now_unix_millis_i64();
    let validation = edgerun_core::validators_proto::validate_revocation_record(revocation, now_ms);
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

    // Only the delegation issuer (or an authorized controller) can revoke
    let issuer_id = revocation
        .issuer
        .as_ref()
        .map(|i| i.identity_id.clone())
        .unwrap_or_default();
    let command_issuer_id = command
        .issuer
        .as_ref()
        .map(|i| i.identity_id.as_slice())
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

    // Store the revocation as an object
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

    // Index the revocation
    let revocation_id_hex = bytes_to_hex(&revocation.revocation_id);
    let delegation_id_hex = bytes_to_hex(&revocation.delegation_id);
    let issuer_hex = revocation
        .issuer
        .as_ref()
        .map(|i| bytes_to_hex(&i.identity_id))
        .unwrap_or_default();

    if let Err(e) = store.store_revocation(
        &revocation_id_hex,
        &delegation_id_hex,
        &issuer_hex,
        now_ms,
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

    // Also add to in-memory revoked set (for current session)
    let mut revoked_delegations = std::collections::HashSet::new();
    revoked_delegations.insert(revocation.delegation_id.clone());

    edgerun_log::info!("revocation recorded");

    // Emit events
    let command_ref = command_ref_from(command);
    let delegations = delegation_refs_from(command);

    let object_ref_vec = match object_ref {
        edgerun_proto::edgerun::v0::common::ObjectRef { .. } => vec![object_ref],
        _ => vec![],
    };

    let event_seq = super::append_signed_event(
        store,
        stream_id,
        signer,
        EventType::CommandCommitted,
        1,
        if object_ref_vec.is_empty() {
            None
        } else {
            Some(object_ref_vec[0].clone())
        },
        vec![command_ref],
        delegations,
        vec![],
    );

    // Write replay cache entry
    if let Some(ref target) = command.target_node {
        let target_hex = bytes_to_hex(&target.node_id);
        let cmd_hash_hex = bytes_to_hex(&super::command_hash(command).value);
        if let Err(e) = store.put_replay_entry(
            &target_hex,
            &cmd_hash_hex,
            &bytes_to_hex(&command.command_id),
            event_seq.unwrap_or(0) as i64,
        ) {
            edgerun_log::warn!("failed to write replay cache entry: {}", e);
        }
    }

    // Emit action events
    record_action_event(
        store,
        stream_id,
        signer,
        command,
        1, // ACTION_STATUS_STARTED
        EventType::ActionStarted,
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

    let response = format!(
        "revocation recorded: {}",
        bytes_to_hex(&revocation.revocation_id)
    )
    .into_bytes();

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
        None,
    )
}
