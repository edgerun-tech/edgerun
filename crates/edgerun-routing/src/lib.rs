// SPDX-License-Identifier: Apache-2.0
use std::sync::Arc;

use edgerun_transport_core::{
    DiscoveryProvider, MuxedTransportSession, PreferMuxedQuicPolicy, RoutingPolicy,
    TransportConnector, TransportEndpoint, TransportError,
};

#[derive(Debug, Clone, Default)]
pub struct RoutePlanner {
    policy: PreferMuxedQuicPolicy,
}

impl RoutePlanner {
    pub fn new(policy: PreferMuxedQuicPolicy) -> Self {
        Self { policy }
    }

    pub fn plan(
        &self,
        endpoints: &[TransportEndpoint],
        require_multiplexing: bool,
    ) -> Result<TransportEndpoint, TransportError> {
        self.policy.choose_endpoint(endpoints, require_multiplexing)
    }
}

#[derive(Clone)]
pub struct TransportOrchestrator {
    policy: Arc<dyn RoutingPolicy>,
    connectors: Vec<Arc<dyn TransportConnector>>,
}

impl TransportOrchestrator {
    pub fn new(
        policy: Arc<dyn RoutingPolicy>,
        connectors: Vec<Arc<dyn TransportConnector>>,
    ) -> Self {
        Self { policy, connectors }
    }

    pub fn prefer_muxed_quic(connectors: Vec<Arc<dyn TransportConnector>>) -> Self {
        Self::new(Arc::new(PreferMuxedQuicPolicy::default()), connectors)
    }

    pub async fn connect_peer(
        &self,
        discovery: &dyn DiscoveryProvider,
        peer_id: &str,
        require_multiplexing: bool,
    ) -> Result<(TransportEndpoint, Box<dyn MuxedTransportSession>), TransportError> {
        let endpoints = discovery.discover(peer_id).await?;
        self.connect_endpoints(endpoints, require_multiplexing)
            .await
    }

    pub async fn connect_endpoints(
        &self,
        endpoints: Vec<TransportEndpoint>,
        require_multiplexing: bool,
    ) -> Result<(TransportEndpoint, Box<dyn MuxedTransportSession>), TransportError> {
        if endpoints.is_empty() {
            return Err(TransportError::NoRoute(
                "no transport endpoints discovered".to_string(),
            ));
        }
        let mut remaining = endpoints;
        let mut last_error: Option<TransportError> = None;

        while !remaining.is_empty() {
            let chosen = self
                .policy
                .choose_endpoint(&remaining, require_multiplexing)?;
            let connector_candidates = self
                .connectors
                .iter()
                .filter(|c| c.supports_kind(chosen.kind))
                .collect::<Vec<_>>();
            if connector_candidates.is_empty() {
                last_error = Some(TransportError::UnsupportedKind(chosen.kind));
                remaining.retain(|ep| ep != &chosen);
                continue;
            }

            let mut connected = None;
            for connector in connector_candidates {
                match connector.connect(&chosen).await {
                    Ok(session) => {
                        connected = Some(session);
                        break;
                    }
                    Err(err) => {
                        last_error = Some(err);
                    }
                }
            }
            if let Some(session) = connected {
                return Ok((chosen, session));
            }
            remaining.retain(|ep| ep != &chosen);
        }

        Err(last_error.unwrap_or_else(|| {
            TransportError::NoRoute("unable to connect any discovered endpoint".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use bytes::Bytes;
    use edgerun_transport_core::{DiscoveryProvider, TransportError};
    use edgerun_transport_core::{TransportCapabilities, TransportKind, TransportStream};

    use super::*;

    struct MockDiscovery {
        endpoints: Vec<TransportEndpoint>,
    }

    #[async_trait]
    impl DiscoveryProvider for MockDiscovery {
        async fn discover(&self, _peer_id: &str) -> Result<Vec<TransportEndpoint>, TransportError> {
            Ok(self.endpoints.clone())
        }
    }

    struct MockConnector {
        kind: TransportKind,
        fail: bool,
    }

    #[async_trait]
    impl TransportConnector for MockConnector {
        fn supports_kind(&self, kind: TransportKind) -> bool {
            self.kind == kind
        }

        async fn connect(
            &self,
            endpoint: &TransportEndpoint,
        ) -> Result<Box<dyn MuxedTransportSession>, TransportError> {
            if self.fail {
                return Err(TransportError::Protocol(format!(
                    "connector failed for {}",
                    endpoint.uri
                )));
            }
            Ok(Box::new(MockSession {
                kind: endpoint.kind,
            }))
        }
    }

    struct MockSession {
        kind: TransportKind,
    }

    #[async_trait]
    impl MuxedTransportSession for MockSession {
        fn kind(&self) -> TransportKind {
            self.kind
        }

        fn capabilities(&self) -> TransportCapabilities {
            TransportCapabilities {
                multiplexed_streams: true,
                reliable_ordered_delivery: true,
                encrypted_channel: true,
            }
        }

        async fn open_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
            Ok(Box::new(MockStream { id: 1 }))
        }

        async fn accept_stream(&self) -> Result<Box<dyn TransportStream>, TransportError> {
            Ok(Box::new(MockStream { id: 2 }))
        }

        async fn close(&self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    struct MockStream {
        id: u64,
    }

    #[async_trait]
    impl TransportStream for MockStream {
        fn id(&self) -> u64 {
            self.id
        }

        async fn send(&mut self, _chunk: Bytes) -> Result<(), TransportError> {
            Ok(())
        }

        async fn recv(&mut self) -> Result<Option<Bytes>, TransportError> {
            Ok(None)
        }

        async fn finish(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn planner_prefers_quic() {
        let planner = RoutePlanner::default();
        let endpoints = vec![
            TransportEndpoint::new(TransportKind::WebSocket, "wss://node/ws"),
            TransportEndpoint::new(TransportKind::Quic, "quic://node:4433"),
        ];
        let chosen = planner.plan(&endpoints, true).expect("route");
        assert_eq!(chosen.kind, TransportKind::Quic);
    }

    #[tokio::test]
    async fn orchestrator_falls_back_to_ws_when_quic_fails() {
        let discovery = MockDiscovery {
            endpoints: vec![
                TransportEndpoint::new(TransportKind::Quic, "quic://peer:4433"),
                TransportEndpoint::new(TransportKind::WebSocket, "wss://peer/ws"),
            ],
        };
        let orchestrator = TransportOrchestrator::prefer_muxed_quic(vec![
            Arc::new(MockConnector {
                kind: TransportKind::Quic,
                fail: true,
            }),
            Arc::new(MockConnector {
                kind: TransportKind::WebSocket,
                fail: false,
            }),
        ]);

        let (endpoint, session) = orchestrator
            .connect_peer(&discovery, "peer", true)
            .await
            .expect("connect");
        assert_eq!(endpoint.kind, TransportKind::WebSocket);
        assert_eq!(session.kind(), TransportKind::WebSocket);
    }
}
