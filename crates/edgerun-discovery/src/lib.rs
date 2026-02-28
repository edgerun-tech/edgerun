// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use edgerun_p2p::RouteAdvertisementV1;
use edgerun_transport_core::{DiscoveryProvider, TransportEndpoint, TransportError};

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

#[derive(Debug, Clone, Default)]
pub struct Libp2pRouteCache {
    routes: Arc<RwLock<HashMap<String, RouteAdvertisementV1>>>,
}

impl Libp2pRouteCache {
    pub fn apply_advertisement(&self, advertisement: RouteAdvertisementV1) {
        if advertisement.peer_id.trim().is_empty() {
            return;
        }
        if advertisement.ttl_ms == 0 {
            return;
        }
        if advertisement.endpoints.is_empty() {
            return;
        }
        let mut guard = self.routes.write().expect("lock poisoned");
        guard.insert(advertisement.peer_id.clone(), advertisement);
    }

    pub fn discover_live(&self, peer_id: &str) -> Option<Vec<TransportEndpoint>> {
        let now = now_unix_ms();
        let mut guard = self.routes.write().expect("lock poisoned");
        guard.retain(|_, advertisement| !advertisement.is_expired_at(now));
        guard
            .get(peer_id)
            .map(|advertisement| advertisement.endpoints.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Libp2pFirstDiscovery {
    cache: Libp2pRouteCache,
}

impl Libp2pFirstDiscovery {
    pub fn new(cache: Libp2pRouteCache) -> Self {
        Self { cache }
    }

    pub fn route_cache(&self) -> &Libp2pRouteCache {
        &self.cache
    }
}

#[async_trait]
impl DiscoveryProvider for Libp2pFirstDiscovery {
    async fn discover(&self, peer_id: &str) -> Result<Vec<TransportEndpoint>, TransportError> {
        if let Some(endpoints) = self.cache.discover_live(peer_id) {
            if !endpoints.is_empty() {
                return Ok(endpoints);
            }
        }
        Err(TransportError::NoRoute(format!(
            "no libp2p route found for peer {peer_id}"
        )))
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
    use edgerun_transport_core::TransportKind;

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

    #[test]
    fn libp2p_cache_returns_live_advertisement() {
        let cache = Libp2pRouteCache::default();
        cache.apply_advertisement(RouteAdvertisementV1::new(
            "peer-42",
            now_unix_ms(),
            5_000,
            vec![TransportEndpoint::new(
                TransportKind::Quic,
                "quic://peer-42.example:4433",
            )],
        ));

        let endpoints = cache.discover_live("peer-42").expect("cached route");
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].kind, TransportKind::Quic);
    }

    #[test]
    fn libp2p_cache_expires_stale_advertisement() {
        let cache = Libp2pRouteCache::default();
        cache.apply_advertisement(RouteAdvertisementV1::new(
            "peer-42",
            1_000,
            1,
            vec![TransportEndpoint::new(
                TransportKind::WebSocket,
                "wss://peer-42.example/ws",
            )],
        ));

        assert!(cache.discover_live("peer-42").is_none());
    }

    #[tokio::test]
    async fn libp2p_first_discovery_fails_closed_when_route_missing() {
        let discovery = Libp2pFirstDiscovery::new(Libp2pRouteCache::default());
        let err = discovery
            .discover("peer-missing")
            .await
            .expect_err("missing route should fail");
        match err {
            TransportError::NoRoute(message) => {
                assert!(message.contains("peer-missing"));
            }
            other => panic!("expected no route error, got {other:?}"),
        }
    }
}
