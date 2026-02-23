// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use edgerun_transport_core::{DiscoveryProvider, TransportEndpoint, TransportError, TransportKind};
use edgerun_types::control_plane::{
    ControlWsClientMessage, ControlWsRequestPayload, ControlWsResponsePayload,
    ControlWsServerMessage, RouteResolveRequest, RouteResolveResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

static REQUEST_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Default, Clone)]
pub struct StaticDiscoveryProvider {
    peers: Arc<RwLock<HashMap<String, Vec<TransportEndpoint>>>>,
}

impl StaticDiscoveryProvider {
    pub fn insert(&self, peer_id: impl Into<String>, endpoints: Vec<TransportEndpoint>) {
        let mut guard = self.peers.write().expect("lock poisoned");
        guard.insert(peer_id.into(), endpoints);
    }
}

#[async_trait]
impl DiscoveryProvider for StaticDiscoveryProvider {
    async fn discover(&self, peer_id: &str) -> Result<Vec<TransportEndpoint>, TransportError> {
        let guard = self.peers.read().expect("lock poisoned");
        let endpoints = guard
            .get(peer_id)
            .cloned()
            .ok_or_else(|| TransportError::NoRoute(format!("peer not found: {peer_id}")))?;
        Ok(endpoints)
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerRouteDiscovery {
    base_url: String,
}

impl SchedulerRouteDiscovery {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

fn next_request_id() -> String {
    let seq = REQUEST_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("discovery-{seq}")
}

fn control_ws_url(base: &str) -> Result<Url, TransportError> {
    let mut url = Url::parse(base).map_err(|e| TransportError::Protocol(e.to_string()))?;
    let scheme = match url.scheme() {
        "http" => "ws".to_string(),
        "https" => "wss".to_string(),
        "ws" | "wss" => url.scheme().to_string(),
        other => {
            return Err(TransportError::Protocol(format!(
                "unsupported scheduler scheme for control ws: {other}"
            )));
        }
    };
    url.set_scheme(&scheme)
        .map_err(|_| TransportError::Protocol("failed to set websocket scheme".to_string()))?;
    url.set_path("/v1/control/ws");
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("client_id", "discovery-route-resolve");
    Ok(url)
}

#[async_trait]
impl DiscoveryProvider for SchedulerRouteDiscovery {
    async fn discover(&self, peer_id: &str) -> Result<Vec<TransportEndpoint>, TransportError> {
        let ws_url = control_ws_url(&self.base_url)?;
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url.as_str())
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        let request_id = next_request_id();
        let request = ControlWsClientMessage {
            request_id: request_id.clone(),
            payload: ControlWsRequestPayload::RouteResolve(RouteResolveRequest {
                device_id: peer_id.to_string(),
            }),
        };
        let encoded =
            bincode::serialize(&request).map_err(|e| TransportError::Protocol(e.to_string()))?;
        socket
            .send(Message::Binary(encoded.into()))
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        let mut payload: Option<RouteResolveResponse> = None;
        while let Some(frame) = socket.next().await {
            let frame = frame.map_err(|e| TransportError::Protocol(e.to_string()))?;
            let Message::Binary(bytes) = frame else {
                continue;
            };
            let response = bincode::deserialize::<ControlWsServerMessage>(&bytes)
                .map_err(|e| TransportError::Protocol(e.to_string()))?;
            if response.request_id != request_id {
                continue;
            }
            if !response.ok {
                let status = response
                    .status
                    .map(|v| format!(" ({v})"))
                    .unwrap_or_default();
                let err = response
                    .error
                    .unwrap_or_else(|| format!("scheduler route resolve failed{status}"));
                return Err(TransportError::Protocol(err));
            }
            let data = response.data.ok_or_else(|| {
                TransportError::Protocol("missing route resolve payload".to_string())
            })?;
            let ControlWsResponsePayload::RouteResolve(resolved) = data else {
                return Err(TransportError::Protocol(
                    "unexpected route resolve response payload".to_string(),
                ));
            };
            payload = Some(resolved);
            break;
        }
        let payload = payload.ok_or_else(|| {
            TransportError::Protocol("scheduler closed control ws before response".to_string())
        })?;
        if !payload.found {
            return Err(TransportError::NoRoute(format!(
                "no route found for peer {peer_id}"
            )));
        }
        let Some(route) = payload.route else {
            return Err(TransportError::NoRoute(format!(
                "route payload missing for peer {peer_id}"
            )));
        };

        let mut out = Vec::new();
        for raw in route.reachable_urls {
            let Some(kind) = kind_from_uri(&raw) else {
                continue;
            };
            out.push(TransportEndpoint::new(kind, raw));
        }
        if out.is_empty() {
            return Err(TransportError::NoRoute(format!(
                "no usable endpoints found for peer {peer_id}"
            )));
        }
        Ok(out)
    }
}

fn kind_from_uri(uri: &str) -> Option<TransportKind> {
    if uri.starts_with("quic://") {
        return Some(TransportKind::Quic);
    }
    if uri.starts_with("ws://") || uri.starts_with("wss://") {
        return Some(TransportKind::WebSocket);
    }
    if uri.starts_with("wg://") || uri.starts_with("wireguard://") {
        return Some(TransportKind::WireGuard);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn static_provider_returns_inserted_endpoints() {
        let provider = StaticDiscoveryProvider::default();
        provider.insert(
            "peer-1",
            vec![TransportEndpoint::new(
                TransportKind::Quic,
                "quic://peer-1.example:4433",
            )],
        );
        let endpoints = provider.discover("peer-1").await.expect("discover");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].kind, TransportKind::Quic);
    }
}
