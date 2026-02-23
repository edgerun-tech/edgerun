// SPDX-License-Identifier: GPL-2.0-only
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use prost::Message;

use crate::durability::DurabilityLevel;
use crate::event::{ActorId, Event as StorageEvent, StreamId};
use crate::{StorageEngine, StorageError};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/edgerun.event_bus.v1.rs"));
}

pub use proto::{
    BusEventEnvelope, EventBusPolicyV1, PolicyRuleV1, PolicyUpdateDeniedEventV1,
    PolicyUpdateRequestV1, PolicyUpdatedEventV1,
};

static EVENT_BUS_COUNTER: AtomicU64 = AtomicU64::new(1);
const EVENT_BUS_INTERNAL_PUBLISHER: &str = "event-bus";
const POLICY_UPDATE_REQUEST: &str = "policy_update_request";
const POLICY_UPDATED: &str = "policy_updated";
const POLICY_UPDATE_DENIED: &str = "policy_update_denied";

#[derive(Debug, Clone)]
pub struct BusQueryRow {
    pub offset: u64,
    pub event_hash: String,
    pub envelope: BusEventEnvelope,
}

#[derive(Debug, Clone)]
pub struct BusQueryResult {
    pub events: Vec<BusQueryRow>,
    pub next_cursor_offset: Option<u64>,
}

pub trait EventBus {
    fn publish(&mut self, envelope: &BusEventEnvelope) -> Result<u64, StorageError>;
    fn query(&mut self, limit: usize, cursor_offset: u64) -> Result<BusQueryResult, StorageError>;
}

pub struct StorageBackedEventBus {
    engine: StorageEngine,
    segment: String,
    stream_id: StreamId,
    actor_id: ActorId,
}

struct BusState {
    active_policy: Option<EventBusPolicyV1>,
    last_nonce_by_publisher: HashMap<String, u64>,
}

impl StorageBackedEventBus {
    pub fn open_writer(data_dir: PathBuf, segment: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        Ok(Self {
            engine,
            segment: segment.to_string(),
            stream_id: stream_id_for_seed("edgerun-event-bus-stream"),
            actor_id: actor_id_for_seed("edgerun-event-bus-actor"),
        })
    }

    pub fn open_reader(data_dir: PathBuf, segment: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        Ok(Self {
            engine,
            segment: segment.to_string(),
            stream_id: stream_id_for_seed("edgerun-event-bus-stream"),
            actor_id: actor_id_for_seed("edgerun-event-bus-actor"),
        })
    }

    pub fn build_envelope(
        nonce: u64,
        publisher: String,
        signature: String,
        policy_id: String,
        recipients: Vec<String>,
        payload_type: String,
        payload: Vec<u8>,
    ) -> BusEventEnvelope {
        let ts_unix_ms = now_unix_ms();
        let event_id = format!(
            "evt-{}-{}-{}",
            ts_unix_ms,
            std::process::id(),
            EVENT_BUS_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        BusEventEnvelope {
            event_id,
            nonce,
            publisher,
            signature,
            policy_id,
            recipients,
            payload_type,
            payload,
            ts_unix_ms,
        }
    }

    fn decode_envelope(bytes: &[u8]) -> Result<BusEventEnvelope, StorageError> {
        BusEventEnvelope::decode(bytes).map_err(|e| {
            StorageError::InvalidSealPolicy(format!("invalid bus envelope protobuf payload: {e}"))
        })
    }

    fn decode_message<T: Message + Default>(bytes: &[u8], name: &str) -> Result<T, StorageError> {
        T::decode(bytes).map_err(|e| {
            StorageError::InvalidSealPolicy(format!("invalid protobuf {name}: {e}"))
        })
    }

    fn append_raw_event(&mut self, envelope: &BusEventEnvelope) -> Result<u64, StorageError> {
        let payload = envelope.encode_to_vec();
        let event = StorageEvent::new(self.stream_id.clone(), self.actor_id.clone(), payload);
        self.engine.append_event_to_segmented_journal(
            &self.segment,
            &event,
            8 * 1024 * 1024,
            DurabilityLevel::AckDurable,
        )
    }

    fn reconstruct_state(&self) -> Result<BusState, StorageError> {
        let all_events = self.engine.query_segmented_journal_raw(&self.segment)?;
        let mut active_policy: Option<EventBusPolicyV1> = None;
        let mut nonce_map: HashMap<String, u64> = HashMap::new();
        for row in all_events {
            let envelope = Self::decode_envelope(&row.event.payload)?;
            nonce_map
                .entry(envelope.publisher.clone())
                .and_modify(|v| *v = (*v).max(envelope.nonce))
                .or_insert(envelope.nonce);
            if envelope.payload_type == POLICY_UPDATED {
                let updated = Self::decode_message::<PolicyUpdatedEventV1>(
                    &envelope.payload,
                    "policy_updated payload",
                )?;
                let Some(policy) = updated.policy else {
                    return Err(StorageError::InvalidSealPolicy(
                        "policy_updated payload missing policy".to_string(),
                    ));
                };
                active_policy = Some(policy);
            }
        }
        Ok(BusState {
            active_policy,
            last_nonce_by_publisher: nonce_map,
        })
    }

    fn policy_allows(policy: &EventBusPolicyV1, envelope: &BusEventEnvelope) -> bool {
        policy.rules.iter().any(|rule| {
            let publisher_match = rule.publisher == "*" || rule.publisher == envelope.publisher;
            let payload_match = rule.payload_type == "*" || rule.payload_type == envelope.payload_type;
            publisher_match && payload_match
        })
    }

    fn validate_policy_update_request(
        &self,
        request: &PolicyUpdateRequestV1,
    ) -> Result<EventBusPolicyV1, String> {
        if request.schema_version != 1 {
            return Err("unsupported policy schema_version".to_string());
        }
        let Some(policy) = request.policy.clone() else {
            return Err("policy_update_request missing policy".to_string());
        };
        if policy.version == 0 {
            return Err("policy.version must be > 0".to_string());
        }
        if policy.rules.is_empty() {
            return Err("policy.rules must not be empty".to_string());
        }
        let allows_policy_updates = policy.rules.iter().any(|r| {
            (r.payload_type == POLICY_UPDATE_REQUEST || r.payload_type == "*")
                && (!r.publisher.trim().is_empty())
        });
        if !allows_policy_updates {
            return Err(
                "policy footgun: new policy would block all future policy_update_request events"
                    .to_string(),
            );
        }
        Ok(policy)
    }

    fn build_internal_outcome_event(
        nonce: u64,
        payload_type: &str,
        payload: Vec<u8>,
    ) -> BusEventEnvelope {
        StorageBackedEventBus::build_envelope(
            nonce,
            EVENT_BUS_INTERNAL_PUBLISHER.to_string(),
            "internal".to_string(),
            "policy-system".to_string(),
            vec!["*".to_string()],
            payload_type.to_string(),
            payload,
        )
    }
}

impl EventBus for StorageBackedEventBus {
    fn publish(&mut self, envelope: &BusEventEnvelope) -> Result<u64, StorageError> {
        let state = self.reconstruct_state()?;
        let expected_nonce = state
            .last_nonce_by_publisher
            .get(&envelope.publisher)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        if envelope.nonce != expected_nonce {
            return Err(StorageError::InvalidSealPolicy(format!(
                "invalid nonce for publisher '{}': expected {}, got {}",
                envelope.publisher, expected_nonce, envelope.nonce
            )));
        }

        if envelope.payload_type == POLICY_UPDATE_REQUEST {
            let offset = self.append_raw_event(envelope)?;
            let request = Self::decode_message::<PolicyUpdateRequestV1>(
                &envelope.payload,
                "policy_update_request payload",
            );
            let internal_nonce = state
                .last_nonce_by_publisher
                .get(EVENT_BUS_INTERNAL_PUBLISHER)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            let outcome = match request {
                Ok(req) => match self.validate_policy_update_request(&req) {
                    Ok(policy) => {
                        let payload = PolicyUpdatedEventV1 {
                            schema_version: 1,
                            policy: Some(policy),
                            applied_policy_id: envelope.policy_id.clone(),
                        }
                        .encode_to_vec();
                        Self::build_internal_outcome_event(internal_nonce, POLICY_UPDATED, payload)
                    }
                    Err(reason) => {
                        let payload = PolicyUpdateDeniedEventV1 {
                            schema_version: 1,
                            denied_policy_id: envelope.policy_id.clone(),
                            reason,
                        }
                        .encode_to_vec();
                        Self::build_internal_outcome_event(
                            internal_nonce,
                            POLICY_UPDATE_DENIED,
                            payload,
                        )
                    }
                },
                Err(err) => {
                    let payload = PolicyUpdateDeniedEventV1 {
                        schema_version: 1,
                        denied_policy_id: envelope.policy_id.clone(),
                        reason: err.to_string(),
                    }
                    .encode_to_vec();
                    Self::build_internal_outcome_event(internal_nonce, POLICY_UPDATE_DENIED, payload)
                }
            };
            let _ = self.append_raw_event(&outcome)?;
            return Ok(offset);
        }

        let Some(policy) = state.active_policy.as_ref() else {
            return Err(StorageError::InvalidSealPolicy(
                "event bus policy not initialized: submit policy_update_request first".to_string(),
            ));
        };
        if !Self::policy_allows(policy, envelope) {
            return Err(StorageError::InvalidSealPolicy(format!(
                "event denied by policy: publisher='{}' payload_type='{}'",
                envelope.publisher, envelope.payload_type
            )));
        }
        self.append_raw_event(envelope)
    }

    fn query(&mut self, limit: usize, cursor_offset: u64) -> Result<BusQueryResult, StorageError> {
        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let start = cursor_offset as usize;
        let end = start.saturating_add(limit).min(rows_all.len());
        let events = rows_all[start..end]
            .iter()
            .map(|row| {
                let envelope = Self::decode_envelope(&row.event.payload)?;
                Ok(BusQueryRow {
                    offset: row.offset,
                    event_hash: hex::encode(row.event_hash),
                    envelope,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        let next_cursor_offset = if end < rows_all.len() {
            Some(end as u64)
        } else {
            None
        };
        Ok(BusQueryResult {
            events,
            next_cursor_offset,
        })
    }
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

    #[test]
    fn default_deny_before_policy_init() {
        let tmp = TempDir::new().expect("tempdir");
        let mut bus = StorageBackedEventBus::open_writer(tmp.path().to_path_buf(), "events.seg")
            .expect("open");
        let ev = StorageBackedEventBus::build_envelope(
            1,
            "scheduler".to_string(),
            "sig".to_string(),
            "p1".to_string(),
            vec!["worker-a".to_string()],
            "job_created".to_string(),
            b"{\"job_id\":\"j1\"}".to_vec(),
        );
        let err = bus.publish(&ev).expect_err("must deny before init");
        assert!(err.to_string().contains("policy not initialized"));
    }

    #[test]
    fn policy_update_request_emits_outcome_and_enables_flow() {
        let tmp = TempDir::new().expect("tempdir");
        let mut bus = StorageBackedEventBus::open_writer(tmp.path().to_path_buf(), "events.seg")
            .expect("open");
        let policy_req = PolicyUpdateRequestV1 {
            schema_version: 1,
            policy: Some(EventBusPolicyV1 {
                version: 1,
                rules: vec![
                    PolicyRuleV1 {
                        publisher: "scheduler".to_string(),
                        payload_type: "job_created".to_string(),
                    },
                    PolicyRuleV1 {
                        publisher: "*".to_string(),
                        payload_type: POLICY_UPDATE_REQUEST.to_string(),
                    },
                ],
            }),
        };
        let req = StorageBackedEventBus::build_envelope(
            1,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["*".to_string()],
            POLICY_UPDATE_REQUEST.to_string(),
            policy_req.encode_to_vec(),
        );
        let _ = bus.publish(&req).expect("publish policy request");

        let ev = StorageBackedEventBus::build_envelope(
            2,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["worker-a".to_string()],
            "job_created".to_string(),
            b"{\"job_id\":\"j1\"}".to_vec(),
        );
        let _ = bus.publish(&ev).expect("publish allowed event");

        let queried = bus.query(50, 0).expect("query");
        assert!(
            queried
                .events
                .iter()
                .any(|e| e.envelope.payload_type == POLICY_UPDATED)
        );
        assert!(
            queried
                .events
                .iter()
                .any(|e| e.envelope.payload_type == "job_created")
        );
    }
}
