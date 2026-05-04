//! Node-owned exchange event stream helpers.
//!
//! The node owns exactly one event stream and that stream is identified by the
//! node identity. Exchange code must not choose or pass a stream id. It only
//! produces `ExchangeEvent` values; this module stores their payload objects and
//! appends signed envelopes to the node's single stream.

use edgerun_core::protocol::Digest;
use edgerun_core::util::now_prost_timestamp;
use edgerun_exchange::{
    build_exchange_event_envelope, decode_exchange_event, encode_exchange_event, project_order,
    ExchangeEvent, ExchangeOrderProjection,
};
use edgerun_hardware_signing::MeshSigner;
use edgerun_core::protocol::ObjectKind;
use edgerun_storage::NodeStore;

fn node_stream_id(signer: &dyn MeshSigner) -> Vec<u8> {
    signer.node_id().0.to_vec()
}

/// Append one exchange event to the node's single event stream.
pub fn append_exchange_event_to_node_stream(
    store: &NodeStore,
    signer: &dyn MeshSigner,
    event: &ExchangeEvent,
) -> Result<u64, String> {
    let recipients = [signer.node_id().0.to_vec()];
    append_exchange_event_to_node_stream_with_recipients(store, signer, event, &recipients)
}

/// Append one exchange event to the node's single event stream with explicit
/// encrypted object recipients.
///
/// The stream id is always the writer node id. It is intentionally not an
/// argument, because node code must not write exchange events to arbitrary
/// streams.
pub fn append_exchange_event_to_node_stream_with_recipients(
    store: &NodeStore,
    signer: &dyn MeshSigner,
    event: &ExchangeEvent,
    recipients: &[Vec<u8>],
) -> Result<u64, String> {
    let stream_id = node_stream_id(signer);
    let recipients = if recipients.is_empty() {
        vec![signer.node_id().0.to_vec()]
    } else {
        recipients.to_vec()
    };

    let encoded = encode_exchange_event(event);
    let payload_object = store
        .put_object(
            &encoded.payload_bytes,
            ObjectKind::Payload as i32,
            &recipients,
        )
        .map_err(|e| format!("exchange_payload_object_store_failed: {e}"))?;

    let (seq, prev_event_hash) = match store.get_head(&stream_id) {
        Ok(Some((head_seq, head_hash))) => (
            (head_seq + 1) as u64,
            Some(Digest {
                algorithm: 1,
                value: head_hash,
            }),
        ),
        Ok(None) => (0, None),
        Err(e) => return Err(format!("exchange_stream_head_failed: {e}")),
    };

    let mut envelope = build_exchange_event_envelope(
        event,
        stream_id,
        seq,
        prev_event_hash,
        payload_object,
    );
    envelope.recorded_at = Some(now_prost_timestamp());

    store
        .append_signed_event_blocking(envelope, signer)
        .map_err(|e| format!("exchange_stream_append_failed: {e}"))
}

/// Append multiple exchange events to the node's single event stream.
pub fn append_exchange_events_to_node_stream(
    store: &NodeStore,
    signer: &dyn MeshSigner,
    events: &[ExchangeEvent],
) -> Result<Vec<u64>, String> {
    let mut seqs = Vec::with_capacity(events.len());
    for event in events {
        seqs.push(append_exchange_event_to_node_stream(store, signer, event)?);
    }
    Ok(seqs)
}

/// Rebuild an order projection from exchange events decoded from the node's
/// single event stream.
pub fn project_exchange_order_from_node_stream(
    store: &NodeStore,
    signer: &dyn MeshSigner,
    order_id: &str,
) -> Result<Option<ExchangeOrderProjection>, String> {
    let stream_id = node_stream_id(signer);
    let Some((head_seq, _head_hash)) = store
        .get_head(&stream_id)
        .map_err(|e| format!("exchange_stream_head_failed: {e}"))?
    else {
        return Ok(None);
    };

    let mut events = Vec::new();
    for seq in 0..=head_seq.max(0) as u64 {
        let Some((envelope, Some(payload_bytes))) = store
            .get_event_with_payload(&stream_id, seq)
            .map_err(|e| format!("exchange_stream_event_load_failed: {e}"))?
        else {
            continue;
        };

        let Some(event) = decode_exchange_event(envelope.event_type, &payload_bytes) else {
            continue;
        };

        if event.order_id() == Some(order_id) {
            events.push(event);
        }
    }

    Ok(project_order(&events, order_id))
}
