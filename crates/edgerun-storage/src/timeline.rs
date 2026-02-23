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

pub use proto::{
    InteractionPayloadV1, TimelineActorTypeV1, TimelineEventEnvelopeV1, TimelineEventTypeV1,
};

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

#[derive(Debug, Clone, Default)]
pub struct TimelineStatus {
    pub schema_version: u32,
    pub events_total: u64,
    pub unique_run_ids: u64,
    pub unique_job_ids: u64,
    pub unique_session_ids: u64,
    pub last_event_id: String,
    pub last_seq: u64,
    pub last_ts_unix_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineQueryFilter {
    pub event_type: Option<TimelineEventTypeV1>,
    pub run_id: Option<String>,
    pub job_id: Option<String>,
    pub session_id: Option<String>,
    pub actor_id: Option<String>,
    pub payload_type: Option<String>,
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
        validate_envelope(envelope)?;
        let mut staged = envelope.clone();
        let all_rows = self.engine.query_segmented_journal_raw(&self.segment)?;
        for row in &all_rows {
            let existing = decode_envelope(&row.event.payload)?;
            if existing.event_id == envelope.event_id {
                if same_logical_event(&existing, envelope) {
                    return Ok(row.offset);
                }
                return Err(StorageError::InvalidSealPolicy(format!(
                    "duplicate event_id with conflicting envelope: {}",
                    envelope.event_id
                )));
            }
        }
        if let Some(last) = all_rows.last() {
            staged.prev_event_hash = last.event_hash.to_vec();
        }
        staged.seq = all_rows.len() as u64 + 1;
        staged.event_hash = compute_envelope_hash(&staged);
        self.append_raw_event(&staged)
    }

    pub fn query(
        &mut self,
        limit: usize,
        cursor_offset: u64,
        filter: TimelineQueryFilter,
    ) -> Result<TimelineQueryResult, StorageError> {
        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut filtered = Vec::new();
        let mut next_cursor_offset = None;
        for (idx, row) in rows_all.iter().enumerate().skip(cursor_offset as usize) {
            let envelope = decode_envelope(&row.event.payload)?;
            if let Some(expected) = filter.event_type {
                if envelope.event_type != expected as i32 {
                    continue;
                }
            }
            if let Some(expected) = filter.run_id.as_ref() {
                if &envelope.run_id != expected {
                    continue;
                }
            }
            if let Some(expected) = filter.job_id.as_ref() {
                if &envelope.job_id != expected {
                    continue;
                }
            }
            if let Some(expected) = filter.session_id.as_ref() {
                if &envelope.session_id != expected {
                    continue;
                }
            }
            if let Some(expected) = filter.actor_id.as_ref() {
                if &envelope.actor_id != expected {
                    continue;
                }
            }
            if let Some(expected) = filter.payload_type.as_ref() {
                if &envelope.payload_type != expected {
                    continue;
                }
            }
            filtered.push(TimelineQueryRow {
                offset: row.offset,
                event_hash: hex::encode(row.event_hash),
                envelope,
            });
            if filtered.len() >= limit {
                if idx + 1 < rows_all.len() {
                    next_cursor_offset = Some((idx + 1) as u64);
                }
                break;
            }
        }
        Ok(TimelineQueryResult {
            events: filtered,
            next_cursor_offset,
        })
    }

    pub fn status(&self) -> Result<TimelineStatus, StorageError> {
        use std::collections::HashSet;

        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut run_ids: HashSet<String> = HashSet::new();
        let mut job_ids: HashSet<String> = HashSet::new();
        let mut session_ids: HashSet<String> = HashSet::new();
        let mut events_total = 0u64;
        let mut last_event_id = String::new();
        let mut last_seq = 0u64;
        let mut last_ts_unix_ms = 0u64;

        for row in rows_all {
            let envelope = decode_envelope(&row.event.payload)?;
            events_total = events_total.saturating_add(1);
            if !envelope.run_id.is_empty() {
                run_ids.insert(envelope.run_id.clone());
            }
            if !envelope.job_id.is_empty() {
                job_ids.insert(envelope.job_id.clone());
            }
            if !envelope.session_id.is_empty() {
                session_ids.insert(envelope.session_id.clone());
            }
            last_event_id = envelope.event_id;
            last_seq = envelope.seq;
            last_ts_unix_ms = envelope.ts_unix_ms;
        }

        Ok(TimelineStatus {
            schema_version: 1,
            events_total,
            unique_run_ids: run_ids.len() as u64,
            unique_job_ids: job_ids.len() as u64,
            unique_session_ids: session_ids.len() as u64,
            last_event_id,
            last_seq,
            last_ts_unix_ms,
        })
    }

    fn append_raw_event(
        &mut self,
        envelope: &TimelineEventEnvelopeV1,
    ) -> Result<u64, StorageError> {
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

fn validate_envelope(envelope: &TimelineEventEnvelopeV1) -> Result<(), StorageError> {
    if envelope.schema_version != 1 {
        return Err(StorageError::InvalidSealPolicy(format!(
            "invalid timeline envelope schema_version: expected 1, got {}",
            envelope.schema_version
        )));
    }
    if envelope.event_id.trim().is_empty() {
        return Err(StorageError::InvalidSealPolicy(
            "invalid timeline envelope: empty event_id".to_string(),
        ));
    }
    if envelope.actor_id.trim().is_empty() {
        return Err(StorageError::InvalidSealPolicy(
            "invalid timeline envelope: empty actor_id".to_string(),
        ));
    }
    if envelope.payload_type.trim().is_empty() {
        return Err(StorageError::InvalidSealPolicy(
            "invalid timeline envelope: empty payload_type".to_string(),
        ));
    }
    Ok(())
}

fn same_logical_event(a: &TimelineEventEnvelopeV1, b: &TimelineEventEnvelopeV1) -> bool {
    a.schema_version == b.schema_version
        && a.event_id == b.event_id
        && a.ts_unix_ms == b.ts_unix_ms
        && a.run_id == b.run_id
        && a.job_id == b.job_id
        && a.session_id == b.session_id
        && a.actor_type == b.actor_type
        && a.actor_id == b.actor_id
        && a.event_type == b.event_type
        && a.payload_type == b.payload_type
        && a.payload == b.payload
        && a.payload_hash_blake3 == b.payload_hash_blake3
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_envelope() -> TimelineEventEnvelopeV1 {
        let payload = InteractionPayloadV1 {
            text: "hello".to_string(),
        }
        .encode_to_vec();
        StorageBackedTimeline::build_envelope(
            "run-1".to_string(),
            "job-1".to_string(),
            "sess-1".to_string(),
            TimelineActorTypeV1::TimelineActorTypeAgent,
            "agent-1".to_string(),
            TimelineEventTypeV1::TimelineEventTypeInteractionAgentOutput,
            "interaction.v1".to_string(),
            payload,
        )
    }

    #[test]
    fn duplicate_event_id_is_idempotent_when_payload_matches() {
        let tmp = TempDir::new().expect("tempdir");
        let mut timeline =
            StorageBackedTimeline::open_writer(tmp.path().to_path_buf(), "timeline.seg")
                .expect("open");
        let mut ev = sample_envelope();
        ev.event_id = "tl-fixed".to_string();
        let first = timeline.publish(&ev).expect("first publish");
        let second = timeline.publish(&ev).expect("idempotent replay");
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_event_id_conflict_is_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let mut timeline =
            StorageBackedTimeline::open_writer(tmp.path().to_path_buf(), "timeline.seg")
                .expect("open");
        let mut ev1 = sample_envelope();
        ev1.event_id = "tl-fixed".to_string();
        let _ = timeline.publish(&ev1).expect("first publish");

        let mut ev2 = ev1.clone();
        ev2.payload = InteractionPayloadV1 {
            text: "different".to_string(),
        }
        .encode_to_vec();
        ev2.payload_hash_blake3 = blake3::hash(&ev2.payload).as_bytes().to_vec();
        let err = timeline
            .publish(&ev2)
            .expect_err("must reject conflicting duplicate id");
        assert!(err
            .to_string()
            .contains("duplicate event_id with conflicting envelope"));
    }

    #[test]
    fn publish_rejects_invalid_schema_version() {
        let tmp = TempDir::new().expect("tempdir");
        let mut timeline =
            StorageBackedTimeline::open_writer(tmp.path().to_path_buf(), "timeline.seg")
                .expect("open");
        let mut ev = sample_envelope();
        ev.schema_version = 0;
        let err = timeline.publish(&ev).expect_err("invalid schema");
        assert!(err.to_string().contains("schema_version"));
    }

    #[test]
    fn status_reports_counts_and_last_event() {
        let tmp = TempDir::new().expect("tempdir");
        let mut timeline =
            StorageBackedTimeline::open_writer(tmp.path().to_path_buf(), "timeline.seg")
                .expect("open");
        let mut ev1 = sample_envelope();
        ev1.run_id = "run-a".to_string();
        ev1.job_id = "job-a".to_string();
        ev1.session_id = "sess-a".to_string();
        let _ = timeline.publish(&ev1).expect("publish 1");

        let mut ev2 = sample_envelope();
        ev2.run_id = "run-b".to_string();
        ev2.job_id = "job-b".to_string();
        ev2.session_id = "sess-b".to_string();
        let _ = timeline.publish(&ev2).expect("publish 2");

        let status = timeline.status().expect("status");
        assert_eq!(status.schema_version, 1);
        assert_eq!(status.events_total, 2);
        assert_eq!(status.unique_run_ids, 2);
        assert_eq!(status.unique_job_ids, 2);
        assert_eq!(status.unique_session_ids, 2);
        assert_eq!(status.last_seq, 2);
        assert!(!status.last_event_id.is_empty());
    }

    #[test]
    fn query_cursor_with_filter_advances_by_underlying_stream_index() {
        let tmp = TempDir::new().expect("tempdir");
        let mut timeline =
            StorageBackedTimeline::open_writer(tmp.path().to_path_buf(), "timeline.seg")
                .expect("open");

        let mut e1 = sample_envelope();
        e1.payload_type = "kind.a".to_string();
        let _ = timeline.publish(&e1).expect("publish e1");

        let mut e2 = sample_envelope();
        e2.payload_type = "kind.b".to_string();
        let _ = timeline.publish(&e2).expect("publish e2");

        let mut e3 = sample_envelope();
        e3.payload_type = "kind.a".to_string();
        let _ = timeline.publish(&e3).expect("publish e3");

        let mut e4 = sample_envelope();
        e4.payload_type = "kind.a".to_string();
        let _ = timeline.publish(&e4).expect("publish e4");

        let page1 = timeline
            .query(
                2,
                0,
                TimelineQueryFilter {
                    payload_type: Some("kind.a".to_string()),
                    ..TimelineQueryFilter::default()
                },
            )
            .expect("page1");
        assert_eq!(page1.events.len(), 2);
        assert_eq!(page1.events[0].envelope.payload_type, "kind.a");
        assert_eq!(page1.events[1].envelope.payload_type, "kind.a");
        assert_eq!(page1.events[0].envelope.seq, 1);
        assert_eq!(page1.events[1].envelope.seq, 3);
        assert_eq!(page1.next_cursor_offset, Some(3));

        let page2 = timeline
            .query(
                2,
                page1.next_cursor_offset.expect("cursor"),
                TimelineQueryFilter {
                    payload_type: Some("kind.a".to_string()),
                    ..TimelineQueryFilter::default()
                },
            )
            .expect("page2");
        assert_eq!(page2.events.len(), 1);
        assert_eq!(page2.events[0].envelope.seq, 4);
        assert_eq!(page2.next_cursor_offset, None);
    }
}
