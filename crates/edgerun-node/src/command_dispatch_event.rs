//! Shared command dispatch event writing.
//!
//! This module owns the generic command-result event path that used to be
//! duplicated inside command handlers.

use crate::command_dispatch_result::{build_command_result_payload, command_ref_from};
use crate::command_result_codec::encode_command_result_payload;
use edgerun_core::protocol::Digest;
use edgerun_core::util::{bytes_to_hex, now_prost_timestamp};
use edgerun_core::wire_command::command_hash;
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::common::ObjectRef;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, EventEnvelope, EventType};
use edgerun_storage::NodeStore;

#[derive(Clone, Debug)]
pub struct CommandResultEventWrite {
    pub event_type: EventType,
    pub event_seq: i64,
    pub response_bytes: Vec<u8>,
    pub result_object: Option<ObjectRef>,
}

pub fn append_command_result_event(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    committed: bool,
    reason_code: &str,
    result_object: Option<ObjectRef>,
) -> Result<CommandResultEventWrite, String> {
    let decision = if committed {
        CommandDecision::Committed as i32
    } else {
        CommandDecision::Rejected as i32
    };
    let event_type = if committed {
        EventType::CommandCommitted
    } else {
        EventType::CommandRejected
    };

    let result_payload = build_command_result_payload(
        command,
        decision,
        reason_code,
        None,
        None,
        result_object.clone(),
    );
    let response_bytes = encode_command_result_payload(&result_payload);
    let payload_object = store
        .put_object(
            &response_bytes,
            edgerun_proto::edgerun::v0::common::ObjectKind::Command as i32,
            &[stream_id.to_vec()],
        )
        .map_err(|e| format!("command_result_object_storage_failed: {e}"))?;

    let (seq, prev_event_hash) = match store.get_head(stream_id) {
        Ok(Some((head_seq, head_hash))) => (
            (head_seq + 1) as u64,
            Some(Digest {
                algorithm: 1,
                value: head_hash,
            }),
        ),
        Ok(None) => (0, None),
        Err(e) => return Err(format!("failed_to_get_stream_head: {e}")),
    };

    let event = EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq,
        prev_event_hash,
        event_type: event_type as i32,
        event_version: 1,
        recorded_at: Some(now_prost_timestamp()),
        effective_at: None,
        payload_object: Some(payload_object),
        related_events: Vec::new(),
        related_commands: vec![command_ref_from(command)],
        related_objects: result_object.clone().into_iter().collect(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };

    store
        .append_signed_event_blocking(event, signer)
        .map_err(|e| format!("append_command_result_failed: {e}"))?;

    if let Some(target) = command.target_node.as_ref() {
        let target_hex = bytes_to_hex(&target.node_id);
        let command_hash_hex = bytes_to_hex(&command_hash(command).value);
        let command_id_hex = bytes_to_hex(&command.command_id);
        let _ = store.put_replay_entry(
            &target_hex,
            &command_hash_hex,
            &command_id_hex,
            seq as i64,
        );
    }

    Ok(CommandResultEventWrite {
        event_type,
        event_seq: seq as i64,
        response_bytes,
        result_object,
    })
}
