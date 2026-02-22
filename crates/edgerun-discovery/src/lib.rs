// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use edgerun_transport_core::{DiscoveryProvider, TransportEndpoint, TransportError, TransportKind};
use serde::Deserialize;

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
    client: reqwest::Client,
}

impl SchedulerRouteDiscovery {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RouteResolveResponse {
    found: bool,
    route: Option<RouteResolveEntry>,
}

#[derive(Debug, Deserialize)]
struct RouteResolveEntry {
    reachable_urls: Vec<String>,
}

#[async_trait]
impl DiscoveryProvider for SchedulerRouteDiscovery {
    async fn discover(&self, peer_id: &str) -> Result<Vec<TransportEndpoint>, TransportError> {
        let url = format!("{}/v1/route/resolve/{}", self.base_url, peer_id);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(TransportError::Protocol(format!(
                "scheduler route resolve failed with status {status}"
            )));
        }

        let payload = response
            .json::<RouteResolveResponse>()
            .await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
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
