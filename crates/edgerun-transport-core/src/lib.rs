// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

use async_trait::async_trait;
use bytes::Bytes;
use edgerun_types::control_plane::{
    HeartbeatRequest, WorkerAssignmentsRequest, WorkerFailureReport, WorkerReplayArtifactReport,
    WorkerResultReport,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Quic,
    WebSocket,
    WireGuard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportEndpoint {
    pub kind: TransportKind,
    pub uri: String,
    #[serde(default)]
    pub priority: u16,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl TransportEndpoint {
    pub fn new(kind: TransportKind, uri: impl Into<String>) -> Self {
        Self {
            kind,
            uri: uri.into(),
            priority: 100,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportCapabilities {
    pub multiplexed_streams: bool,
    pub reliable_ordered_delivery: bool,
    pub encrypted_channel: bool,
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("unsupported transport kind {0:?}")]
    UnsupportedKind(TransportKind),
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("no route available: {0}")]
    NoRoute(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(&'static str),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for TransportError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[async_trait]
pub trait TransportStream: Send + Sync {
    fn id(&self) -> u64;
    async fn send(&mut self, chunk: Bytes) -> Result<(), TransportError>;
    async fn recv(&mut self) -> Result<Option<Bytes>, TransportError>;
    async fn finish(&mut self) -> Result<(), TransportError>;
}

#[async_trait]
pub trait MuxedTransportSession: Send + Sync {
    fn kind(&self) -> TransportKind;
    fn capabilities(&self) -> TransportCapabilities;
    async fn open_stream(&self) -> Result<Box<dyn TransportStream>, TransportError>;
    async fn accept_stream(&self) -> Result<Box<dyn TransportStream>, TransportError>;
    async fn close(&self) -> Result<(), TransportError>;
}

#[async_trait]
pub trait TransportConnector: Send + Sync {
    fn supports_kind(&self, kind: TransportKind) -> bool;
    async fn connect(
        &self,
        endpoint: &TransportEndpoint,
    ) -> Result<Box<dyn MuxedTransportSession>, TransportError>;
}

#[async_trait]
pub trait DiscoveryProvider: Send + Sync {
    async fn discover(&self, peer_id: &str) -> Result<Vec<TransportEndpoint>, TransportError>;
}

pub trait RoutingPolicy: Send + Sync {
    fn choose_endpoint(
        &self,
        candidates: &[TransportEndpoint],
        require_multiplexing: bool,
    ) -> Result<TransportEndpoint, TransportError>;
}

#[derive(Debug, Clone)]
pub struct PreferMuxedQuicPolicy {
    pub allow_ws_fallback: bool,
}

impl Default for PreferMuxedQuicPolicy {
    fn default() -> Self {
        Self {
            allow_ws_fallback: true,
        }
    }
}

impl RoutingPolicy for PreferMuxedQuicPolicy {
    fn choose_endpoint(
        &self,
        candidates: &[TransportEndpoint],
        require_multiplexing: bool,
    ) -> Result<TransportEndpoint, TransportError> {
        if candidates.is_empty() {
            return Err(TransportError::NoRoute(
                "no transport endpoints discovered".to_string(),
            ));
        }

        let mut scored = candidates.to_vec();
        scored.sort_by_key(|ep| {
            let kind_score = match ep.kind {
                TransportKind::Quic => 0_u8,
                TransportKind::WebSocket if self.allow_ws_fallback => 1_u8,
                TransportKind::WireGuard => 2_u8,
                _ => 3_u8,
            };
            (kind_score, ep.priority)
        });

        for endpoint in scored {
            if require_multiplexing && matches!(endpoint.kind, TransportKind::WireGuard) {
                continue;
            }
            if !self.allow_ws_fallback && matches!(endpoint.kind, TransportKind::WebSocket) {
                continue;
            }
            return Ok(endpoint);
        }

        Err(TransportError::NoRoute(
            "no endpoint satisfies routing policy".to_string(),
        ))
    }
}

pub fn heartbeat_signing_message(payload: &HeartbeatRequest) -> String {
    let runtime_ids = payload.runtime_ids.join(",");
    let (max_concurrent, mem_bytes) = payload
        .capacity
        .as_ref()
        .map(|c| (c.max_concurrent.to_string(), c.mem_bytes.to_string()))
        .unwrap_or_else(|| ("".to_string(), "".to_string()));
    format!(
        "heartbeat|{}|{}|{}|{}|{}",
        payload.worker_pubkey, runtime_ids, payload.version, max_concurrent, mem_bytes
    )
}

pub fn assignments_signing_message(payload: &WorkerAssignmentsRequest) -> String {
    format!(
        "assignments|{}|{}",
        payload.worker_pubkey, payload.signed_at_unix_s
    )
}

pub fn result_signing_message(payload: &WorkerResultReport) -> String {
    let attestation_sig = payload.attestation_sig.clone().unwrap_or_default();
    let attestation_claim = payload
        .attestation_claim
        .as_ref()
        .map(canonical_attestation_claim)
        .unwrap_or_default();
    format!(
        "result|{}|{}|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.bundle_hash,
        payload.output_hash,
        payload.output_len,
        attestation_sig,
        attestation_claim
    )
}

fn canonical_attestation_claim(claim: &edgerun_types::AttestationClaim) -> String {
    format!(
        "measurement={};issued_at_unix_s={};expires_at_unix_s={};nonce={};format={};evidence={}",
        claim.measurement,
        claim.issued_at_unix_s,
        claim.expires_at_unix_s,
        claim.nonce.as_deref().unwrap_or_default(),
        claim.format.as_deref().unwrap_or_default(),
        claim.evidence.as_deref().unwrap_or_default()
    )
}

pub fn failure_signing_message(payload: &WorkerFailureReport) -> String {
    format!(
        "failure|{}|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.bundle_hash,
        payload.phase,
        payload.error_code,
        payload.error_message
    )
}

pub fn replay_signing_message(payload: &WorkerReplayArtifactReport) -> String {
    let ok_flag = if payload.artifact.ok { "1" } else { "0" };
    let artifact_output_hash = payload.artifact.output_hash.clone().unwrap_or_default();
    format!(
        "replay|{}|{}|{}|{}|{}",
        payload.worker_pubkey,
        payload.job_id,
        payload.artifact.bundle_hash,
        ok_flag,
        artifact_output_hash
    )
}

pub fn route_register_signing_message(
    owner_pubkey: &str,
    device_id: &str,
    reachable_urls: &[String],
    challenge_nonce: &str,
    signed_at_unix_s: u64,
) -> String {
    let urls = reachable_urls.join(",");
    format!(
        "edgerun:route_register:v1|{}|{}|{}|{}|{}",
        owner_pubkey, device_id, urls, challenge_nonce, signed_at_unix_s
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_prefers_quic_then_ws() {
        let policy = PreferMuxedQuicPolicy::default();
        let ws = TransportEndpoint::new(TransportKind::WebSocket, "wss://a.example/ws");
        let quic = TransportEndpoint::new(TransportKind::Quic, "quic://a.example:4433");
        let chosen = policy
            .choose_endpoint(&[ws.clone(), quic.clone()], true)
            .expect("route");
        assert_eq!(chosen.kind, TransportKind::Quic);
    }
}
