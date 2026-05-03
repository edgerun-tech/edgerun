//! Exchange runtime bridge to the existing protocol event stream.
//!
//! This is intentionally not a new event sink. It wires exchange domain events
//! through the already-existing object store + EventEnvelope stream writer:
//!
//! 1. `encode_exchange_event(event)`
//! 2. store payload bytes as `OBJECT_KIND_PAYLOAD`
//! 3. `build_exchange_event_envelope(...)`
//! 4. append/sign through `NodeStore::append_signed_event_blocking(...)`
//! 5. rebuild projections from decoded stream payload objects

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::addr_of;

use edgerun_core::protocol::Digest;
use edgerun_core::util::now_prost_timestamp;
use edgerun_exchange::{
    build_exchange_event_envelope, decode_exchange_event, encode_exchange_event,
    project_order, ExchangeEvent, ExchangeOrderProjection,
};
use edgerun_hardware_signing::MeshSigner;
use edgerun_proto::edgerun::v0::common::ObjectKind;
use edgerun_rt::sync::Mutex;
use edgerun_storage::NodeStore;

pub struct ExchangeStreamRuntime {
    pub store: Arc<NodeStore>,
    pub signer: Arc<dyn MeshSigner + Send + Sync>,
    pub stream_id: Vec<u8>,
    pub recipients: Vec<Vec<u8>>,
}

static mut GLOBAL_EXCHANGE_STREAM_RUNTIME: Option<Mutex<ExchangeStreamRuntime>> = None;

#[inline(always)]
fn global_exchange_stream_runtime_ptr() -> *const Option<Mutex<ExchangeStreamRuntime>> {
    addr_of!(GLOBAL_EXCHANGE_STREAM_RUNTIME)
}

/// Configure exchange API/runtime to commit exchange lifecycle events to the
/// existing node event stream.
pub fn init_exchange_stream_runtime(
    store: Arc<NodeStore>,
    signer: Arc<dyn MeshSigner + Send + Sync>,
    stream_id: Vec<u8>,
) {
    let recipients = vec![signer.node_id().0.to_vec()];
    init_exchange_stream_runtime_with_recipients(store, signer, stream_id, recipients);
}

/// Same as `init_exchange_stream_runtime`, but allows callers to include extra
/// encrypted object recipients for replicated/escrowed payload access.
pub fn init_exchange_stream_runtime_with_recipients(
    store: Arc<NodeStore>,
    signer: Arc<dyn MeshSigner + Send + Sync>,
    stream_id: Vec<u8>,
    recipients: Vec<Vec<u8>>,
) {
    let recipients = if recipients.is_empty() {
        vec![signer.node_id().0.to_vec()]
    } else {
        recipients
    };

    unsafe {
        GLOBAL_EXCHANGE_STREAM_RUNTIME = Some(Mutex::new(ExchangeStreamRuntime {
            store,
            signer,
            stream_id,
            recipients,
        }));
    }
}

pub fn with_exchange_stream_runtime<R>(f: impl FnOnce(&ExchangeStreamRuntime) -> R) -> Option<R> {
    unsafe {
        (*global_exchange_stream_runtime_ptr()).as_ref().map(|runtime| {
            let guard = runtime.lock();
            f(&guard)
        })
    }
}

/// Append one exchange event through the existing signed event stream.
///
/// Returns `Ok(None)` when the exchange stream runtime has not been configured,
/// so local/in-memory development still works. Production callers should call
/// `init_exchange_stream_runtime*` before serving exchange routes.
pub fn append_exchange_event_to_stream(event: &ExchangeEvent) -> Result<Option<u64>, String> {
    with_exchange_stream_runtime(|runtime| append_exchange_event_to_runtime(runtime, event))
        .unwrap_or(Ok(None))
}

pub fn append_exchange_events_to_stream(events: &[ExchangeEvent]) -> Result<Vec<u64>, String> {
    let mut out = Vec::new();
    for event in events {
        if let Some(seq) = append_exchange_event_to_stream(event)? {
            out.push(seq);
        }
    }
    Ok(out)
}

fn append_exchange_event_to_runtime(
    runtime: &ExchangeStreamRuntime,
    event: &ExchangeEvent,
) -> Result<Option<u64>, String> {
    let encoded = encode_exchange_event(event);

    let payload_object = runtime
        .store
        .put_object(
            &encoded.payload_bytes,
            ObjectKind::Payload as i32,
            &runtime.recipients,
        )
        .map_err(|e| format!("exchange_payload_object_store_failed: {e}"))?;

    let (seq, prev_event_hash) = match runtime.store.get_head(&runtime.stream_id) {
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
        runtime.stream_id.clone(),
        seq,
        prev_event_hash,
        payload_object,
    );
    envelope.recorded_at = Some(now_prost_timestamp());

    runtime
        .store
        .append_signed_event_blocking(envelope, runtime.signer.as_ref())
        .map_err(|e| format!("exchange_stream_append_failed: {e}"))?;

    Ok(Some(seq))
}

/// Rebuild an order projection from exchange events decoded out of the existing
/// stream payload objects.
pub fn project_exchange_order_from_stream(order_id: &str) -> Result<Option<ExchangeOrderProjection>, String> {
    with_exchange_stream_runtime(|runtime| project_exchange_order_from_runtime(runtime, order_id))
        .unwrap_or(Ok(None))
}

fn project_exchange_order_from_runtime(
    runtime: &ExchangeStreamRuntime,
    order_id: &str,
) -> Result<Option<ExchangeOrderProjection>, String> {
    let Some((head_seq, _head_hash)) = runtime
        .store
        .get_head(&runtime.stream_id)
        .map_err(|e| format!("exchange_stream_head_failed: {e}"))?
    else {
        return Ok(None);
    };

    let mut events = Vec::new();
    for seq in 0..=head_seq.max(0) as u64 {
        let Some((envelope, Some(payload_bytes))) = runtime
            .store
            .get_event_with_payload(&runtime.stream_id, seq)
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
