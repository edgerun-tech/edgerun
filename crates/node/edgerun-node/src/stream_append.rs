//! Node helpers for appending signed stream events.
//!
//! These helpers load the prior durable head, ask `edgerun-stream` to build the
//! next signed event, then append that exact envelope to storage.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_hardware_signing::MeshSigner;
use edgerun_sign::ProtocolSigner;
use edgerun_storage::NodeStore;

use crate::protocol_signer::BorrowedMeshProtocolSigner;

pub fn append_command_stream_event(
    store: &NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    event_type: edgerun_core::protocol::EventType,
    event_version: u32,
    payload_object: Option<edgerun_core::protocol::ObjectRef>,
    related_commands: Vec<edgerun_core::protocol::CommandRef>,
    related_delegations: Vec<edgerun_core::protocol::DelegationRef>,
    related_revocations: Vec<edgerun_core::protocol::RevocationRef>,
) -> Option<u64> {
    let event = append_signed_stream_event_blocking(
        store,
        stream_id,
        signer,
        edgerun_stream::EventDraft {
            event_type: event_type as i32,
            event_version,
            recorded_at: Some(edgerun_core::util::now_protocol_timestamp()),
            payload_object,
            related_commands,
            related_delegations,
            related_revocations,
            ..Default::default()
        },
    )
    .map_err(|e| {
        edgerun_log::warn!("failed to append command stream event: {e}");
        e
    })
    .ok()?;
    Some(event.seq)
}

pub fn append_signed_stream_event_blocking(
    store: &NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    draft: edgerun_stream::EventDraft,
) -> Result<edgerun_core::protocol::EventEnvelope, String> {
    append_signed_stream_event_blocking_with_protocol_signer(
        store,
        stream_id,
        &BorrowedMeshProtocolSigner::new(signer),
        draft,
    )
}

pub fn append_signed_stream_event_blocking_with_protocol_signer(
    store: &NodeStore,
    stream_id: &[u8],
    signer: &(impl ProtocolSigner + ?Sized),
    draft: edgerun_stream::EventDraft,
) -> Result<edgerun_core::protocol::EventEnvelope, String> {
    let previous = load_stream_head_event(store, stream_id)?;
    let stream_id = stream_id_array(stream_id)?;
    let event = edgerun_stream::build_signed_event(&stream_id, previous.as_ref(), draft, signer)
        .map_err(|e| format!("stream_event_build_failed: {e}"))?;
    store
        .append_event_blocking(event.clone())
        .map_err(|e| format!("stream_event_append_failed: {e}"))?;
    Ok(event)
}

pub async fn append_signed_stream_event(
    store: &NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    draft: edgerun_stream::EventDraft,
) -> Result<edgerun_core::protocol::EventEnvelope, String> {
    append_signed_stream_event_with_protocol_signer(
        store,
        stream_id,
        &BorrowedMeshProtocolSigner::new(signer),
        draft,
    )
    .await
}

pub async fn append_signed_stream_event_with_protocol_signer(
    store: &NodeStore,
    stream_id: &[u8],
    signer: &(impl ProtocolSigner + ?Sized),
    draft: edgerun_stream::EventDraft,
) -> Result<edgerun_core::protocol::EventEnvelope, String> {
    let previous = load_stream_head_event(store, stream_id)?;
    let stream_id = stream_id_array(stream_id)?;
    let event = edgerun_stream::build_signed_event(&stream_id, previous.as_ref(), draft, signer)
        .map_err(|e| format!("stream_event_build_failed: {e}"))?;
    store
        .append_event(event.clone())
        .await
        .map_err(|e| format!("stream_event_append_failed: {e}"))?;
    Ok(event)
}

fn load_stream_head_event(
    store: &NodeStore,
    stream_id: &[u8],
) -> Result<Option<edgerun_core::protocol::EventEnvelope>, String> {
    let Some((head_seq, _head_hash)) = store
        .get_head(stream_id)
        .map_err(|e| format!("stream_head_load_failed: {e}"))?
    else {
        return Ok(None);
    };
    if head_seq < 0 {
        return Err("stream_head_invalid_negative_seq".into());
    }
    store
        .get_event(stream_id, head_seq as u64)
        .map_err(|e| format!("stream_head_event_load_failed: {e}"))?
        .ok_or_else(|| format!("stream_head_event_missing: seq={head_seq}"))
        .map(Some)
}

fn stream_id_array(stream_id: &[u8]) -> Result<edgerun_stream::StreamId, String> {
    if stream_id.len() != core::mem::size_of::<edgerun_stream::StreamId>() {
        return Err(format!(
            "stream_id_invalid_len: expected={}, actual={}",
            core::mem::size_of::<edgerun_stream::StreamId>(),
            stream_id.len()
        ));
    }
    let mut out = [0u8; core::mem::size_of::<edgerun_stream::StreamId>()];
    out.copy_from_slice(stream_id);
    Ok(out)
}
