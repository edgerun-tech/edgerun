use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use edgerun_work::{
    CHANNEL_KIND_QUIC, ChannelEnvelope, Hash, NodeId, RelayPacketFrame, RelayTransportError,
    RouteBinding, WorkPacketTransport, archived_packet_frame_from_bytes,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuicInboundStreamContext {
    pub channel_id: Hash,
    pub from: NodeId,
    pub to: NodeId,
    pub route_hash: Hash,
}

impl QuicInboundStreamContext {
    pub fn from_envelope(envelope: &ChannelEnvelope) -> Self {
        Self {
            channel_id: envelope.channel_id,
            from: envelope.from,
            to: envelope.to,
            route_hash: envelope.route_hash,
        }
    }
}

#[derive(Debug)]
pub struct QuicWorkTransport {
    pending: VecDeque<PendingQuicWorkPacket>,
    received: VecDeque<RelayPacketFrame>,
    inbound_streams: BTreeMap<u64, QuicInboundStreamContext>,
    default_inbound: Option<QuicInboundStreamContext>,
    next_stream_id: u64,
    max_pending_packets: usize,
    max_received_packets: usize,
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
            received: VecDeque::new(),
            inbound_streams: BTreeMap::new(),
            default_inbound: None,
            next_stream_id: FIRST_CLIENT_BIDI_STREAM_ID,
            max_pending_packets,
            max_received_packets: max_pending_packets,
        }
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn received_len(&self) -> usize {
        self.received.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.received.is_empty()
    }

    pub fn max_pending_packets(&self) -> usize {
        self.max_pending_packets
    }

    pub fn max_received_packets(&self) -> usize {
        self.max_received_packets
    }

    pub fn set_max_received_packets(&mut self, max_received_packets: usize) {
        self.max_received_packets = max_received_packets;
    }

    pub fn next_stream_id(&self) -> u64 {
        self.next_stream_id
    }

    pub fn set_default_inbound_context(&mut self, context: QuicInboundStreamContext) {
        self.default_inbound = Some(context);
    }

    pub fn clear_default_inbound_context(&mut self) {
        self.default_inbound = None;
    }

    pub fn register_inbound_stream(&mut self, stream_id: u64, context: QuicInboundStreamContext) {
        self.inbound_streams.insert(stream_id, context);
    }

    pub fn remove_inbound_stream(&mut self, stream_id: u64) -> Option<QuicInboundStreamContext> {
        self.inbound_streams.remove(&stream_id)
    }

    pub fn drain_pending(&mut self) -> Vec<PendingQuicWorkPacket> {
        self.pending.drain(..).collect()
    }

    pub async fn flush(
        &mut self,
        connection: &mut QuicConnection,
    ) -> Result<usize, RelayTransportError> {
        let mut sent = 0usize;
        while let Some(packet) = self.pending.pop_front() {
            if let Err(_error) = connection
                .send_stream_data(packet.stream_id, &packet.bytes, true)
                .await
            {
                self.pending.push_front(packet);
                return Err(RelayTransportError::DeliveryFailed);
            }
            sent = sent.saturating_add(1);
        }
        Ok(sent)
    }

    pub async fn poll_recv_quic(
        &mut self,
        connection: &mut QuicConnection,
    ) -> Result<usize, RelayTransportError> {
        let mut accepted = 0usize;
        loop {
            let Some((stream_id, bytes, fin)) = connection
                .recv_stream_data()
                .await
                .map_err(|_| RelayTransportError::DeliveryFailed)?
            else {
                break;
            };
            if bytes.is_empty() {
                if fin {
                    self.inbound_streams.remove(&stream_id);
                }
                continue;
            }
            self.accept_quic_stream_data(stream_id, &bytes, fin)?;
            accepted = accepted.saturating_add(1);
        }
        Ok(accepted)
    }

    pub fn accept_quic_stream_data(
        &mut self,
        stream_id: u64,
        bytes: &[u8],
        fin: bool,
    ) -> Result<(), RelayTransportError> {
        if self.received.len() >= self.max_received_packets {
            return Err(RelayTransportError::Backpressure);
        }
        let context = self
            .inbound_streams
            .get(&stream_id)
            .copied()
            .or(self.default_inbound)
            .ok_or(RelayTransportError::UnsupportedRoute)?;
        let frame = archived_packet_frame_from_bytes(bytes)
            .map_err(|_| RelayTransportError::InvalidFrame)?;
        self.received.push_back(RelayPacketFrame {
            channel_id: context.channel_id,
            from: context.from,
            to: context.to,
            route_hash: context.route_hash,
            frame,
        });
        if fin {
            self.inbound_streams.remove(&stream_id);
        }
        Ok(())
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
        route: &RouteBinding,
        packet_bytes: &[u8],
    ) -> Result<(), RelayTransportError> {
        if route.endpoint.kind != CHANNEL_KIND_QUIC {
            return Err(RelayTransportError::UnsupportedRoute);
        }
        if self.pending.len() >= self.max_pending_packets {
            return Err(RelayTransportError::Backpressure);
        }
        let stream_id = self.allocate_stream_id();
        self.pending.push_back(PendingQuicWorkPacket {
            stream_id,
            bytes: packet_bytes.to_vec(),
        });
        Ok(())
    }

    fn recv_packet_frame(&mut self) -> Result<Option<RelayPacketFrame>, RelayTransportError> {
        Ok(self.received.pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::Ed25519SigningKey;
    use edgerun_work::{
        ChannelEndpoint, DEPARTMENT_MESSAGE, NODE_ROLE_MESSAGE, RouteBindingBuilder, SimNode,
        WORK_TYPE_MESSAGE_DELIVER, WorkPacket, archived_packet_frame_from_bytes,
        encode_work_packet_once, quic_endpoint, websocket_endpoint,
    };

    fn route(endpoint: ChannelEndpoint) -> RouteBinding {
        let key = Ed25519SigningKey::from_bytes(&[90u8; 32]);
        RouteBindingBuilder::new(&key, NODE_ROLE_MESSAGE, endpoint).build()
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
            Err(RelayTransportError::UnsupportedRoute)
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
            Err(RelayTransportError::Backpressure)
        );
    }

    #[test]
    fn quic_transport_accepts_received_stream_data_with_default_context() {
        let mut sender = SimNode::from_seed(91, NODE_ROLE_MESSAGE);
        let receiver = SimNode::from_seed(92, NODE_ROLE_MESSAGE);
        let packet = sender.message_to(
            receiver.identity.node_id,
            receiver.identity.node_id,
            DEPARTMENT_MESSAGE,
            WORK_TYPE_MESSAGE_DELIVER,
            b"quic inbound".to_vec(),
        );
        let encoded = encode_work_packet_once(&packet).expect("encode packet");
        let context = QuicInboundStreamContext {
            channel_id: [1u8; 32],
            from: sender.identity.node_id,
            to: receiver.identity.node_id,
            route_hash: [2u8; 32],
        };
        let mut transport = QuicWorkTransport::new();
        transport.set_default_inbound_context(context);

        transport
            .accept_quic_stream_data(0, encoded.as_bytes(), true)
            .expect("accept inbound stream data");

        assert_eq!(transport.received_len(), 1);
        let frame = transport
            .recv_packet_frame()
            .expect("recv frame result")
            .expect("queued frame");
        assert_eq!(frame.channel_id, context.channel_id);
        assert_eq!(frame.from, context.from);
        assert_eq!(frame.to, context.to);
        assert_eq!(frame.route_hash, context.route_hash);
        assert_eq!(frame.frame.hash, encoded.hash);
        assert_eq!(frame.frame.into_packet().expect("packet"), packet);
    }

    #[test]
    fn quic_transport_accepts_received_stream_data_with_registered_context() {
        let mut sender = SimNode::from_seed(93, NODE_ROLE_MESSAGE);
        let receiver = SimNode::from_seed(94, NODE_ROLE_MESSAGE);
        let packet = sender.message_to(
            receiver.identity.node_id,
            receiver.identity.node_id,
            DEPARTMENT_MESSAGE,
            WORK_TYPE_MESSAGE_DELIVER,
            b"registered context".to_vec(),
        );
        let encoded = encode_work_packet_once(&packet).expect("encode packet");
        let context = QuicInboundStreamContext {
            channel_id: [3u8; 32],
            from: sender.identity.node_id,
            to: receiver.identity.node_id,
            route_hash: [4u8; 32],
        };
        let mut transport = QuicWorkTransport::new();
        transport.register_inbound_stream(44, context);

        transport
            .accept_quic_stream_data(44, encoded.as_bytes(), true)
            .expect("accept inbound stream data");

        assert!(transport.remove_inbound_stream(44).is_none());
        let frame = transport
            .recv_packet_frame()
            .expect("recv frame result")
            .expect("queued frame");
        assert_eq!(frame.channel_id, context.channel_id);
    }

    #[test]
    fn quic_transport_rejects_inbound_without_context() {
        let mut transport = QuicWorkTransport::new();
        assert_eq!(
            transport.accept_quic_stream_data(0, b"not routed", true),
            Err(RelayTransportError::UnsupportedRoute)
        );
    }

    #[test]
    fn quic_transport_rejects_invalid_inbound_frame() {
        let mut transport = QuicWorkTransport::new();
        transport.set_default_inbound_context(QuicInboundStreamContext {
            channel_id: [1u8; 32],
            from: [2u8; 32],
            to: [3u8; 32],
            route_hash: [4u8; 32],
        });
        assert_eq!(
            transport.accept_quic_stream_data(0, b"not a work packet", true),
            Err(RelayTransportError::InvalidFrame)
        );
    }
}
