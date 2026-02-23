// SPDX-License-Identifier: GPL-2.0-only
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use prost::Message;

use crate::durability::DurabilityLevel;
use crate::event::{ActorId, Event as StorageEvent, StreamId};
use crate::{StorageEngine, StorageError};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/edgerun.timeline.v1.rs"));
}

pub use proto::{InteractionPayloadV1, TimelineActorTypeV1, TimelineEventEnvelopeV1, TimelineEventTypeV1};

static TIMELINE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct TimelineQueryRow {
    pub offset: u64,
    pub event_hash: String,
    pub envelope: TimelineEventEnvelopeV1,
}

#[derive(Debug, Clone)]
pub struct TimelineQueryResult {
    pub events: Vec<TimelineQueryRow>,
    pub next_cursor_offset: Option<u64>,
}

pub struct StorageBackedTimeline {
    engine: StorageEngine,
    segment: String,
    stream_id: StreamId,
    actor_id: ActorId,
}

impl StorageBackedTimeline {
    pub fn open_writer(data_dir: PathBuf, segment: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        Ok(Self {
            engine,
            segment: segment.to_string(),
            stream_id: stream_id_for_seed("edgerun-timeline-stream"),
            actor_id: actor_id_for_seed("edgerun-timeline-actor"),
        })
    }

    pub fn open_reader(data_dir: PathBuf, segment: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        Ok(Self {
            engine,
            segment: segment.to_string(),
            stream_id: stream_id_for_seed("edgerun-timeline-stream"),
            actor_id: actor_id_for_seed("edgerun-timeline-actor"),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_envelope(
        run_id: String,
        job_id: String,
        session_id: String,
        actor_type: TimelineActorTypeV1,
        actor_id: String,
        event_type: TimelineEventTypeV1,
        payload_type: String,
        payload: Vec<u8>,
    ) -> TimelineEventEnvelopeV1 {
        let ts_unix_ms = now_unix_ms();
        let event_id = format!(
            "tl-{}-{}-{}",
            ts_unix_ms,
            std::process::id(),
            TIMELINE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let mut envelope = TimelineEventEnvelopeV1 {
            schema_version: 1,
            event_id,
            seq: 0,
            ts_unix_ms,
            run_id,
            job_id,
            session_id,
            actor_type: actor_type as i32,
            actor_id,
            event_type: event_type as i32,
            payload_type,
            payload: payload.clone(),
            payload_hash_blake3: blake3::hash(&payload).as_bytes().to_vec(),
            prev_event_hash: Vec::new(),
            event_hash: Vec::new(),
        };
        envelope.event_hash = compute_envelope_hash(&envelope);
        envelope
    }

    pub fn publish(&mut self, envelope: &TimelineEventEnvelopeV1) -> Result<u64, StorageError> {
        let mut staged = envelope.clone();
        let all_rows = self.engine.query_segmented_journal_raw(&self.segment)?;
        if let Some(last) = all_rows.last() {
            staged.prev_event_hash = last.event_hash.to_vec();
            staged.seq = all_rows.len() as u64 + 1;
        } else {
            staged.seq = 1;
        }
        staged.event_hash = compute_envelope_hash(&staged);
        self.append_raw_event(&staged)
    }

    pub fn query(
        &mut self,
        limit: usize,
        cursor_offset: u64,
        event_type: Option<TimelineEventTypeV1>,
    ) -> Result<TimelineQueryResult, StorageError> {
        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut filtered = Vec::new();
        for row in rows_all.iter().skip(cursor_offset as usize) {
            let envelope = decode_envelope(&row.event.payload)?;
            if let Some(expected) = event_type {
                if envelope.event_type != expected as i32 {
                    continue;
                }
            }
            filtered.push(TimelineQueryRow {
                offset: row.offset,
                event_hash: hex::encode(row.event_hash),
                envelope,
            });
            if filtered.len() >= limit {
                break;
            }
        }
        let next_cursor_offset = if filtered.len() == limit {
            Some(cursor_offset + filtered.len() as u64)
        } else {
            None
        };
        Ok(TimelineQueryResult {
            events: filtered,
            next_cursor_offset,
        })
    }

    fn append_raw_event(&mut self, envelope: &TimelineEventEnvelopeV1) -> Result<u64, StorageError> {
        let payload = envelope.encode_to_vec();
        let event = StorageEvent::new(self.stream_id.clone(), self.actor_id.clone(), payload);
        self.engine.append_event_to_segmented_journal(
            &self.segment,
            &event,
            8 * 1024 * 1024,
            DurabilityLevel::AckDurable,
        )
    }
}

fn decode_envelope(bytes: &[u8]) -> Result<TimelineEventEnvelopeV1, StorageError> {
    TimelineEventEnvelopeV1::decode(bytes).map_err(|e| {
        StorageError::InvalidSealPolicy(format!("invalid timeline envelope protobuf payload: {e}"))
    })
}

fn compute_envelope_hash(envelope: &TimelineEventEnvelopeV1) -> Vec<u8> {
    let mut canonical = envelope.clone();
    canonical.event_hash.clear();
    let bytes = canonical.encode_to_vec();
    blake3::hash(&bytes).as_bytes().to_vec()
}

fn stream_id_for_seed(seed: &str) -> StreamId {
    let digest = blake3::hash(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    StreamId::from_bytes(bytes)
}

fn actor_id_for_seed(seed: &str) -> ActorId {
    let digest = blake3::hash(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[16..32]);
    ActorId::from_bytes(bytes)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
