use alloc::collections::VecDeque;
use alloc::vec::Vec;

use edgerun_work::{
    RouteAdvertisement, WorkPacketTransport, WorkTransportError, CHANNEL_KIND_QUIC,
};

use crate::http::http3::quic::QuicConnection;

const DEFAULT_MAX_PENDING_PACKETS: usize = 1024;
const FIRST_CLIENT_BIDI_STREAM_ID: u64 = 0;
const CLIENT_BIDI_STREAM_ID_STRIDE: u64 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingQuicWorkPacket {
    pub stream_id: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct QuicWorkTransport {
    pending: VecDeque<PendingQuicWorkPacket>,
    next_stream_id: u64,
    max_pending_packets: usize,
}

impl Default for QuicWorkTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl QuicWorkTransport {
    pub fn new() -> Self {
        Self::with_max_pending_packets(DEFAULT_MAX_PENDING_PACKETS)
    }

    pub fn with_max_pending_packets(max_pending_packets: usize) -> Self {
        Self {
            pending: VecDeque::new(),
            next_stream_id: FIRST_CLIENT_BIDI_STREAM_ID,
            max_pending_packets,
        }
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn max_pending_packets(&self) -> usize {
        self.max_pending_packets
    }

    pub fn next_stream_id(&self) -> u64 {
        self.next_stream_id
    }

    pub fn drain_pending(&mut self) -> Vec<PendingQuicWorkPacket> {
        self.pending.drain(..).collect()
    }

    pub async fn flush(&mut self, connection: &mut QuicConnection) -> Result<usize, WorkTransportError> {
        let mut sent = 0usize;
        while let Some(packet) = self.pending.pop_front() {
            if let Err(_error) = connection
                .send_stream_data(packet.stream_id, &packet.bytes, true)
                .await
            {
                self.pending.push_front(packet);
                return Err(WorkTransportError::DeliveryFailed);
            }
            sent = sent.saturating_add(1);
        }
        Ok(sent)
    }

    fn allocate_stream_id(&mut self) -> u64 {
        let stream_id = self.next_stream_id;
        self.next_stream_id = self
            .next_stream_id
            .saturating_add(CLIENT_BIDI_STREAM_ID_STRIDE);
        stream_id
    }
}

impl WorkPacketTransport for QuicWorkTransport {
    fn send_packet_bytes(
        &mut self,
        route: &RouteAdvertisement,
        packet_bytes: &[u8],
    ) -> Result<(), WorkTransportError> {
        if route.endpoint.kind != CHANNEL_KIND_QUIC {
            return Err(WorkTransportError::UnsupportedRoute);
        }
        if self.pending.len() >= self.max_pending_packets {
            return Err(WorkTransportError::Backpressure);
        }
        let stream_id = self.allocate_stream_id();
        self.pending.push_back(PendingQuicWorkPacket {
            stream_id,
            bytes: packet_bytes.to_vec(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::Ed25519SigningKey;
    use edgerun_work::{
        empty_signature, quic_endpoint, websocket_endpoint, sign_route_advertisement,
        ChannelEndpoint, NodeIdentity, RouteAdvertisement, WORK_WIRE_ABI_VERSION,
        NODE_ROLE_MESSAGE, ROUTE_STATUS_AVAILABLE,
    };

    fn route(endpoint: ChannelEndpoint) -> RouteAdvertisement {
        let key = Ed25519SigningKey::from_bytes(&[90u8; 32]);
        let identity = edgerun_work::node_identity_from_key(&key, NODE_ROLE_MESSAGE);
        sign_route_advertisement(
            &key,
            RouteAdvertisement {
                abi_version: WORK_WIRE_ABI_VERSION,
                node: NodeIdentity {
                    node_id: identity.node_id,
                    role: identity.role,
                    public_key: identity.public_key,
                },
                relay_node_id: identity.node_id,
                endpoint,
                roles: alloc::vec![NODE_ROLE_MESSAGE],
                departments: alloc::vec![],
                status: ROUTE_STATUS_AVAILABLE,
                sequence: 1,
                valid_until_unix_ms: u64::MAX,
                previous_route_hash: [0u8; 32],
                signature: empty_signature(),
            },
        )
    }

    #[test]
    fn quic_transport_queues_quic_packet_bytes() {
        let mut transport = QuicWorkTransport::new();
        let route = route(quic_endpoint("quic", b"127.0.0.1:4433"));

        transport
            .send_packet_bytes(&route, b"packet")
            .expect("quic route accepted");

        assert_eq!(transport.pending_len(), 1);
        let pending = transport.drain_pending();
        assert_eq!(pending[0].stream_id, 0);
        assert_eq!(pending[0].bytes, b"packet");
        assert_eq!(transport.next_stream_id(), 4);
    }

    #[test]
    fn quic_transport_rejects_non_quic_route() {
        let mut transport = QuicWorkTransport::new();
        let route = route(websocket_endpoint("ws", b"wss://relay.edgerun.test/ws"));

        assert_eq!(
            transport.send_packet_bytes(&route, b"packet"),
            Err(WorkTransportError::UnsupportedRoute)
        );
    }

    #[test]
    fn quic_transport_applies_backpressure() {
        let mut transport = QuicWorkTransport::with_max_pending_packets(1);
        let route = route(quic_endpoint("quic", b"127.0.0.1:4433"));

        transport
            .send_packet_bytes(&route, b"first")
            .expect("first packet accepted");
        assert_eq!(
            transport.send_packet_bytes(&route, b"second"),
            Err(WorkTransportError::Backpressure)
        );
    }
}
