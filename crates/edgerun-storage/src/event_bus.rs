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
    BusEventEnvelope, BusPhaseV1, BusStatusV1, EventBusPolicyV1, HaltEventV1, PolicyRuleV1,
    PolicyUpdateDeniedEventV1, PolicyUpdateRequestV1, PolicyUpdatedEventV1, ResumeFromV1,
};

static EVENT_BUS_COUNTER: AtomicU64 = AtomicU64::new(1);
const EVENT_BUS_INTERNAL_PUBLISHER: &str = "event-bus";
const POLICY_UPDATE_REQUEST: &str = "policy_update_request";
const POLICY_UPDATED: &str = "policy_updated";
const POLICY_UPDATE_DENIED: &str = "policy_update_denied";
const HALT_EVENT: &str = "halt_event";
const RESUME_FROM: &str = "resume_from";

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

#[derive(Debug, Clone, Default)]
pub struct BusQueryFilter {
    pub publisher: Option<String>,
    pub payload_type: Option<String>,
}

pub trait EventBus {
    fn publish(&mut self, envelope: &BusEventEnvelope) -> Result<u64, StorageError>;
    fn query(
        &mut self,
        limit: usize,
        cursor_offset: u64,
        filter: BusQueryFilter,
    ) -> Result<BusQueryResult, StorageError>;
    fn status(&self) -> Result<BusStatusV1, StorageError>;
}

pub struct StorageBackedEventBus {
    engine: StorageEngine,
    segment: String,
    stream_id: StreamId,
    actor_id: ActorId,
    cached_state: Option<BusState>,
}

struct BusState {
    phase: BusPhaseV1,
    active_policy: Option<EventBusPolicyV1>,
    last_nonce_by_publisher: HashMap<String, u64>,
    event_by_id: HashMap<String, (u64, BusEventEnvelope)>,
    last_offset: Option<u64>,
    last_event_id: Option<String>,
    latest_chain_progress_event_id: Option<String>,
}

impl StorageBackedEventBus {
    pub fn open_writer(data_dir: PathBuf, segment: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        Ok(Self {
            engine,
            segment: segment.to_string(),
            stream_id: stream_id_for_seed("edgerun-event-bus-stream"),
            actor_id: actor_id_for_seed("edgerun-event-bus-actor"),
            cached_state: None,
        })
    }

    pub fn open_reader(data_dir: PathBuf, segment: &str) -> Result<Self, StorageError> {
        let engine = StorageEngine::new(data_dir)?;
        Ok(Self {
            engine,
            segment: segment.to_string(),
            stream_id: stream_id_for_seed("edgerun-event-bus-stream"),
            actor_id: actor_id_for_seed("edgerun-event-bus-actor"),
            cached_state: None,
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
        T::decode(bytes)
            .map_err(|e| StorageError::InvalidSealPolicy(format!("invalid protobuf {name}: {e}")))
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
        let mut state = BusState {
            phase: BusPhaseV1::AwaitingInit,
            active_policy: None,
            last_nonce_by_publisher: HashMap::new(),
            event_by_id: HashMap::new(),
            last_offset: None,
            last_event_id: None,
            latest_chain_progress_event_id: None,
        };
        for row in all_events {
            let envelope = Self::decode_envelope(&row.event.payload)?;
            Self::apply_envelope_to_state(&mut state, row.offset, &envelope)?;
        }
        Ok(state)
    }

    fn policy_allows(policy: &EventBusPolicyV1, envelope: &BusEventEnvelope) -> bool {
        policy.rules.iter().any(|rule| {
            let publisher_match = rule.publisher == "*" || rule.publisher == envelope.publisher;
            let payload_match =
                rule.payload_type == "*" || rule.payload_type == envelope.payload_type;
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

    fn ensure_cached_state(&mut self) -> Result<(), StorageError> {
        if self.cached_state.is_none() {
            self.cached_state = Some(self.reconstruct_state()?);
        }
        Ok(())
    }

    fn apply_envelope_to_state(
        state: &mut BusState,
        offset: u64,
        envelope: &BusEventEnvelope,
    ) -> Result<(), StorageError> {
        state
            .event_by_id
            .entry(envelope.event_id.clone())
            .or_insert((offset, envelope.clone()));
        state.last_offset = Some(offset);
        state.last_event_id = Some(envelope.event_id.clone());
        if envelope.payload_type == "chain_progress" {
            state.latest_chain_progress_event_id = Some(envelope.event_id.clone());
        }
        state
            .last_nonce_by_publisher
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
            state.active_policy = Some(policy);
            if state.phase != BusPhaseV1::Halted {
                state.phase = BusPhaseV1::Running;
            }
        } else if envelope.payload_type == HALT_EVENT {
            let halt =
                Self::decode_message::<HaltEventV1>(&envelope.payload, "halt_event payload")?;
            if halt.schema_version != 1 {
                return Err(StorageError::InvalidSealPolicy(format!(
                    "invalid halt_event schema_version: expected 1, got {}",
                    halt.schema_version
                )));
            }
            state.phase = BusPhaseV1::Halted;
        } else if envelope.payload_type == RESUME_FROM {
            let resume =
                Self::decode_message::<ResumeFromV1>(&envelope.payload, "resume_from payload")?;
            if resume.schema_version != 1 {
                return Err(StorageError::InvalidSealPolicy(format!(
                    "invalid resume_from schema_version: expected 1, got {}",
                    resume.schema_version
                )));
            }
            state.phase = if state.active_policy.is_some() {
                BusPhaseV1::Running
            } else {
                BusPhaseV1::AwaitingInit
            };
        }
        Ok(())
    }
}

impl EventBus for StorageBackedEventBus {
    fn publish(&mut self, envelope: &BusEventEnvelope) -> Result<u64, StorageError> {
        self.ensure_cached_state()?;
        if envelope.event_id.trim().is_empty() {
            return Err(StorageError::InvalidSealPolicy(
                "invalid event bus envelope: empty event_id".to_string(),
            ));
        }
        if envelope.publisher.trim().is_empty() {
            return Err(StorageError::InvalidSealPolicy(
                "invalid event bus envelope: empty publisher".to_string(),
            ));
        }
        if envelope.payload_type.trim().is_empty() {
            return Err(StorageError::InvalidSealPolicy(
                "invalid event bus envelope: empty payload_type".to_string(),
            ));
        }
        let state = self
            .cached_state
            .as_ref()
            .expect("cached_state initialized by ensure_cached_state");
        if let Some((offset, existing)) = state.event_by_id.get(&envelope.event_id) {
            if existing == envelope {
                return Ok(*offset);
            }
            return Err(StorageError::InvalidSealPolicy(format!(
                "duplicate event_id with conflicting envelope: {}",
                envelope.event_id
            )));
        }

        let halted = state.phase == BusPhaseV1::Halted;
        if halted && envelope.payload_type != RESUME_FROM {
            return Err(StorageError::InvalidSealPolicy(
                "event bus halted: only resume_from events are accepted".to_string(),
            ));
        }

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
            {
                let state = self
                    .cached_state
                    .as_mut()
                    .expect("cached_state initialized by ensure_cached_state");
                Self::apply_envelope_to_state(state, offset, envelope)?;
            }
            let request = Self::decode_message::<PolicyUpdateRequestV1>(
                &envelope.payload,
                "policy_update_request payload",
            );
            let internal_nonce = self
                .cached_state
                .as_ref()
                .expect("cached_state initialized by ensure_cached_state")
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
                    Self::build_internal_outcome_event(
                        internal_nonce,
                        POLICY_UPDATE_DENIED,
                        payload,
                    )
                }
            };
            let outcome_offset = self.append_raw_event(&outcome)?;
            {
                let state = self
                    .cached_state
                    .as_mut()
                    .expect("cached_state initialized by ensure_cached_state");
                Self::apply_envelope_to_state(state, outcome_offset, &outcome)?;
            }
            return Ok(offset);
        }

        if envelope.payload_type == RESUME_FROM {
            if !halted {
                return Err(StorageError::InvalidSealPolicy(
                    "resume_from rejected: event bus is not halted".to_string(),
                ));
            }
            let resume =
                Self::decode_message::<ResumeFromV1>(&envelope.payload, "resume_from payload")?;
            if resume.schema_version != 1 {
                return Err(StorageError::InvalidSealPolicy(format!(
                    "invalid resume_from schema_version: expected 1, got {}",
                    resume.schema_version
                )));
            }
        } else if envelope.payload_type == HALT_EVENT {
            let halt =
                Self::decode_message::<HaltEventV1>(&envelope.payload, "halt_event payload")?;
            if halt.schema_version != 1 {
                return Err(StorageError::InvalidSealPolicy(format!(
                    "invalid halt_event schema_version: expected 1, got {}",
                    halt.schema_version
                )));
            }
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
        let offset = self.append_raw_event(envelope)?;
        {
            let state = self
                .cached_state
                .as_mut()
                .expect("cached_state initialized by ensure_cached_state");
            Self::apply_envelope_to_state(state, offset, envelope)?;
        }
        Ok(offset)
    }

    fn query(
        &mut self,
        limit: usize,
        cursor_offset: u64,
        filter: BusQueryFilter,
    ) -> Result<BusQueryResult, StorageError> {
        let rows_all = self.engine.query_segmented_journal_raw(&self.segment)?;
        let start = cursor_offset as usize;
        let mut events = Vec::new();
        let mut next_cursor_offset = None;
        for (idx, row) in rows_all.iter().enumerate().skip(start) {
            let envelope = Self::decode_envelope(&row.event.payload)?;
            if let Some(expected) = filter.publisher.as_ref() {
                if &envelope.publisher != expected {
                    continue;
                }
            }
            if let Some(expected) = filter.payload_type.as_ref() {
                if &envelope.payload_type != expected {
                    continue;
                }
            }
            events.push(BusQueryRow {
                offset: row.offset,
                event_hash: hex::encode(row.event_hash),
                envelope,
            });
            if events.len() >= limit {
                if idx + 1 < rows_all.len() {
                    next_cursor_offset = Some((idx + 1) as u64);
                }
                break;
            }
        }
        Ok(BusQueryResult {
            events,
            next_cursor_offset,
        })
    }

    fn status(&self) -> Result<BusStatusV1, StorageError> {
        let reconstructed;
        let state = if let Some(cached) = self.cached_state.as_ref() {
            cached
        } else {
            reconstructed = self.reconstruct_state()?;
            &reconstructed
        };
        let policy_version = state.active_policy.as_ref().map(|p| p.version).unwrap_or(0);
        Ok(BusStatusV1 {
            schema_version: 1,
            phase: state.phase as i32,
            policy_version,
            last_applied_event_id: state.last_event_id.clone().unwrap_or_default(),
            last_offset: state.last_offset.unwrap_or(0),
            latest_chain_progress_event_id: state
                .latest_chain_progress_event_id
                .clone()
                .unwrap_or_default(),
            storage_ok: true,
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

        let queried = bus.query(50, 0, BusQueryFilter::default()).expect("query");
        assert!(queried
            .events
            .iter()
            .any(|e| e.envelope.payload_type == POLICY_UPDATED));
        assert!(queried
            .events
            .iter()
            .any(|e| e.envelope.payload_type == "job_created"));
    }

    #[test]
    fn duplicate_event_id_is_idempotent_when_payload_matches() {
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

        let mut ev = StorageBackedEventBus::build_envelope(
            2,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["worker-a".to_string()],
            "job_created".to_string(),
            b"{\"job_id\":\"j1\"}".to_vec(),
        );
        ev.event_id = "evt-fixed".to_string();
        let first = bus.publish(&ev).expect("first publish");
        let second = bus.publish(&ev).expect("idempotent replay");
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_event_id_conflict_is_rejected() {
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

        let mut ev1 = StorageBackedEventBus::build_envelope(
            2,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["worker-a".to_string()],
            "job_created".to_string(),
            b"{\"job_id\":\"j1\"}".to_vec(),
        );
        ev1.event_id = "evt-fixed".to_string();
        let _ = bus.publish(&ev1).expect("first publish");

        let mut ev2 = ev1.clone();
        ev2.payload = b"{\"job_id\":\"j2\"}".to_vec();
        let err = bus
            .publish(&ev2)
            .expect_err("must reject conflicting duplicate id");
        assert!(err
            .to_string()
            .contains("duplicate event_id with conflicting envelope"));
    }

    #[test]
    fn status_reflects_policy_phase_and_version() {
        let tmp = TempDir::new().expect("tempdir");
        let mut bus = StorageBackedEventBus::open_writer(tmp.path().to_path_buf(), "events.seg")
            .expect("open");

        let s0 = bus.status().expect("status");
        assert_eq!(s0.phase, BusPhaseV1::AwaitingInit as i32);
        assert_eq!(s0.policy_version, 0);

        let policy_req = PolicyUpdateRequestV1 {
            schema_version: 1,
            policy: Some(EventBusPolicyV1 {
                version: 3,
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

        let s1 = bus.status().expect("status after init");
        assert_eq!(s1.phase, BusPhaseV1::Running as i32);
        assert_eq!(s1.policy_version, 3);
        assert!(!s1.last_applied_event_id.is_empty());
    }

    #[test]
    fn halt_and_resume_are_durable_and_enforced() {
        let tmp = TempDir::new().expect("tempdir");
        let mut bus = StorageBackedEventBus::open_writer(tmp.path().to_path_buf(), "events.seg")
            .expect("open");

        let policy_req = PolicyUpdateRequestV1 {
            schema_version: 1,
            policy: Some(EventBusPolicyV1 {
                version: 7,
                rules: vec![
                    PolicyRuleV1 {
                        publisher: "*".to_string(),
                        payload_type: POLICY_UPDATE_REQUEST.to_string(),
                    },
                    PolicyRuleV1 {
                        publisher: "scheduler".to_string(),
                        payload_type: "job_created".to_string(),
                    },
                    PolicyRuleV1 {
                        publisher: "scheduler".to_string(),
                        payload_type: HALT_EVENT.to_string(),
                    },
                    PolicyRuleV1 {
                        publisher: "scheduler".to_string(),
                        payload_type: RESUME_FROM.to_string(),
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

        let halt = StorageBackedEventBus::build_envelope(
            2,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["*".to_string()],
            HALT_EVENT.to_string(),
            HaltEventV1 {
                schema_version: 1,
                reason: "manual".to_string(),
                last_event_id: req.event_id.clone(),
            }
            .encode_to_vec(),
        );
        let _ = bus.publish(&halt).expect("halt event");
        let halted = bus.status().expect("status after halt");
        assert_eq!(halted.phase, BusPhaseV1::Halted as i32);

        let blocked = StorageBackedEventBus::build_envelope(
            3,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["worker-a".to_string()],
            "job_created".to_string(),
            b"{\"job_id\":\"j1\"}".to_vec(),
        );
        let err = bus.publish(&blocked).expect_err("must reject while halted");
        assert!(err.to_string().contains("halted"));

        let resume = StorageBackedEventBus::build_envelope(
            3,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["*".to_string()],
            RESUME_FROM.to_string(),
            ResumeFromV1 {
                schema_version: 1,
                cursor: None,
            }
            .encode_to_vec(),
        );
        let _ = bus.publish(&resume).expect("resume");
        let running = bus.status().expect("status after resume");
        assert_eq!(running.phase, BusPhaseV1::Running as i32);

        let post_resume = StorageBackedEventBus::build_envelope(
            4,
            "scheduler".to_string(),
            "sig".to_string(),
            "p-init".to_string(),
            vec!["worker-a".to_string()],
            "job_created".to_string(),
            b"{\"job_id\":\"j2\"}".to_vec(),
        );
        let _ = bus.publish(&post_resume).expect("publish after resume");
    }
}
