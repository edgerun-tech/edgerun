//! Shared legacy command dispatch audit-event writing.
//!
//! This module owns the local command-result event path for `edgerun-core`
//! command streams. These events support replay and audit; admitted cross-node
//! work authority belongs in `edgerun-work`.

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::command_dispatch_result::command_ref_from;
use edgerun_hardware_signing::MeshSigner;
use edgerun_protocols::core_protocol::command::command_hash;
use edgerun_protocols::core_protocol::protocol::{CommandEnvelope, EventType, ObjectRef};
use edgerun_protocols::core_protocol::util::{bytes_to_hex, now_protocol_timestamp};
use edgerun_protocols::sign::ProtocolSigner;
use edgerun_storage::NodeStore;

use crate::protocol_signer::BorrowedMeshProtocolSigner;

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
    _reason_code: &str,
    result_object: Option<ObjectRef>,
) -> Result<CommandResultEventWrite, String> {
    append_command_result_event_with_protocol_signer(
        command,
        store,
        stream_id,
        &BorrowedMeshProtocolSigner::new(signer),
        committed,
        _reason_code,
        result_object,
    )
}

pub fn append_command_result_event_with_protocol_signer(
    command: &CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &(impl ProtocolSigner + ?Sized),
    committed: bool,
    _reason_code: &str,
    result_object: Option<ObjectRef>,
) -> Result<CommandResultEventWrite, String> {
    let event_type = if committed {
        EventType::CommandCommitted
    } else {
        EventType::CommandRejected
    };

    let event = crate::stream_append::append_signed_stream_event_blocking_with_protocol_signer(
        store,
        stream_id,
        signer,
        edgerun_storage::EventDraft {
            event_type: event_type as i32,
            event_version: 1,
            recorded_at: Some(now_protocol_timestamp()),
            effective_at: None,
            payload_object: None,
            related_events: Vec::new(),
            related_commands: vec![command_ref_from(command)],
            related_objects: result_object.clone().into_iter().collect(),
            related_delegations: Vec::new(),
            related_revocations: Vec::new(),
            event_metadata: None,
        },
    )
    .map_err(|e| format!("append_command_result_failed: {e}"))?;

    if let Some(target) = command.target_node.as_ref() {
        let target_hex = bytes_to_hex(&target.node_id);
        let command_hash_hex = bytes_to_hex(&command_hash(command).value);
        let command_id_hex = bytes_to_hex(&command.command_id);
        let _ = store.put_replay_entry(
            &target_hex,
            &command_hash_hex,
            &command_id_hex,
            event.seq as i64,
        );
    }

    Ok(CommandResultEventWrite {
        event_type,
        event_seq: event.seq as i64,
        response_bytes: Vec::new(),
        result_object,
    })
}
