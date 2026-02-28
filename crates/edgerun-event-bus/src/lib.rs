// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;

pub mod proto {
    tonic::include_proto!("edgerun.edge_internal.v1");
}

pub mod edge_internal;

static EVENT_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum OverlayNetwork {
    EdgeInternal,
    EdgeCluster,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct EventTopic {
    pub overlay: OverlayNetwork,
    pub name: String,
}

impl EventTopic {
    pub fn new(overlay: OverlayNetwork, name: impl Into<String>) -> Result<Self, EventBusError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(EventBusError::InvalidTopic(
                "topic name must not be empty".to_string(),
            ));
        }
        Ok(Self { overlay, name })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: String,
    pub topic: EventTopic,
    pub publisher: String,
    pub payload_type: String,
    pub payload: Vec<u8>,
    pub ts_unix_ms: u64,
}

#[derive(Debug, Error)]
pub enum EventBusError {
    #[error("invalid topic: {0}")]
    InvalidTopic(String),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
    #[error("topic not found: {0:?}")]
    UnknownTopic(EventTopic),
    #[error("publish failed: {0}")]
    PublishFailed(String),
}

#[derive(Clone)]
pub struct RuntimeEventBus {
    inner: Arc<RuntimeEventBusInner>,
}

struct RuntimeEventBusInner {
    channels: Mutex<HashMap<EventTopic, broadcast::Sender<EventEnvelope>>>,
    capacity: usize,
}

impl RuntimeEventBus {
    pub fn new(capacity: usize) -> Result<Self, EventBusError> {
        if capacity == 0 {
            return Err(EventBusError::InvalidTopic(
                "bus capacity must be > 0".to_string(),
            ));
        }
        Ok(Self {
            inner: Arc::new(RuntimeEventBusInner {
                channels: Mutex::new(HashMap::new()),
                capacity,
            }),
        })
    }

    pub fn with_topics(capacity: usize, topics: &[EventTopic]) -> Result<Self, EventBusError> {
        let bus = Self::new(capacity)?;
        for topic in topics {
            bus.ensure_topic(topic)?;
        }
        Ok(bus)
    }

    pub fn ensure_topic(&self, topic: &EventTopic) -> Result<(), EventBusError> {
        let mut guard = self.inner.channels.lock().expect("lock poisoned");
        guard.entry(topic.clone()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(self.inner.capacity);
            tx
        });
        Ok(())
    }

    pub fn subscribe(
        &self,
        topic: &EventTopic,
    ) -> Result<broadcast::Receiver<EventEnvelope>, EventBusError> {
        let guard = self.inner.channels.lock().expect("lock poisoned");
        let Some(tx) = guard.get(topic) else {
            return Err(EventBusError::UnknownTopic(topic.clone()));
        };
        Ok(tx.subscribe())
    }

    pub fn publish(
        &self,
        topic: &EventTopic,
        publisher: impl Into<String>,
        payload_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<EventEnvelope, EventBusError> {
        let publisher = publisher.into();
        let payload_type = payload_type.into();
        if publisher.trim().is_empty() {
            return Err(EventBusError::InvalidEnvelope(
                "publisher must not be empty".to_string(),
            ));
        }
        if payload_type.trim().is_empty() {
            return Err(EventBusError::InvalidEnvelope(
                "payload_type must not be empty".to_string(),
            ));
        }
        let envelope = EventEnvelope {
            event_id: format!(
                "evt-{}-{}-{}",
                now_unix_ms(),
                std::process::id(),
                EVENT_SEQ.fetch_add(1, Ordering::Relaxed)
            ),
            topic: topic.clone(),
            publisher,
            payload_type,
            payload,
            ts_unix_ms: now_unix_ms(),
        };
        let guard = self.inner.channels.lock().expect("lock poisoned");
        let Some(tx) = guard.get(topic) else {
            return Err(EventBusError::UnknownTopic(topic.clone()));
        };
        tx.send(envelope.clone())
            .map_err(|err| EventBusError::PublishFailed(err.to_string()))?;
        Ok(envelope)
    }
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

    #[tokio::test]
    async fn publish_is_push_subscribe_without_polling() {
        let topic = EventTopic::new(OverlayNetwork::EdgeInternal, "scheduler.worker.heartbeat")
            .expect("topic");
        let bus = RuntimeEventBus::with_topics(16, std::slice::from_ref(&topic)).expect("bus");
        let mut rx = bus.subscribe(&topic).expect("subscribe");
        let payload = b"{}".to_vec();
        let sent = bus
            .publish(&topic, "scheduler", "worker_heartbeat", payload.clone())
            .expect("publish");
        let recv = rx.recv().await.expect("recv");
        assert_eq!(recv.event_id, sent.event_id);
        assert_eq!(recv.payload, payload);
        assert_eq!(recv.topic.name, "scheduler.worker.heartbeat");
    }
}
