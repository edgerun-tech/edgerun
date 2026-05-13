use alloc::vec::Vec;

use crate::channel::{CHANNEL_KIND_WEBSOCKET, ChannelEnvelope, RouteBinding};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::encode_channel_envelope_for_route;
use crate::frame_codec::channel_envelope_bytes;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_binding::current_unix_ms;
use crate::route_table::RouteState;
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, Default)]
pub struct WsWorkChannel {
    routes: RouteState,
    outbound_frames: Vec<WsFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WsFrame {
    pub to: NodeId,
    pub route_hash: Hash,
    pub packet_hash: Hash,
    pub packet_bytes: Vec<u8>,
    pub envelope_bytes: Vec<u8>,
}

impl WsWorkChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain_outbound_frames(&mut self) -> Vec<WsFrame> {
        self.outbound_frames.drain(..).collect()
    }

    pub fn drain_outbound_envelope_bytes(&mut self) -> Vec<Vec<u8>> {
        self.outbound_frames
            .drain(..)
            .map(|frame| frame.envelope_bytes)
            .collect()
    }

    pub fn inject_inbound_envelope(&mut self, envelope: ChannelEnvelope) {
        self.routes.push_inbox(envelope);
    }

    pub fn ordered_inbound(
        &mut self,
        order: &mut ChannelOrderBook,
        envelope: ChannelEnvelope,
    ) -> Result<OrderedChannelEnvelope, WorkChannelError> {
        let from = envelope.from;
        let to = envelope.to;
        let sequence = order.next_sequence(envelope.channel_id, from, to);
        let previous_message_hash = order.last_message_hash(envelope.channel_id, from, to);
        let ordered = OrderedChannelEnvelope {
            envelope,
            sequence,
            previous_message_hash,
        };
        order
            .accept(&ordered, ordered.envelope.route_hash)
            .map_err(WorkChannelError::Order)?;
        Ok(ordered)
    }
}

impl WorkChannel for WsWorkChannel {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError> {
        if route.endpoint.kind != CHANNEL_KIND_WEBSOCKET {
            return Err(WorkChannelError::RouteInvalid);
        }
        let hash = self
            .routes
            .insert_live_route(route, current_unix_ms())
            .ok_or(WorkChannelError::RouteInvalid)?;
        Ok(hash)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
        self.routes.remove_route(node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        self.routes.route_hash_for(node_id, current_unix_ms())
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let route = self
            .routes
            .route_for_send(&to, current_unix_ms())
            .ok_or(WorkChannelError::RouteMissing)?;
        let encoded = encode_channel_envelope_for_route(route, from, to, packet)
            .map_err(|_| WorkChannelError::PacketHashFailed)?;
        let envelope = encoded.envelope;
        let envelope_bytes =
            channel_envelope_bytes(&envelope).map_err(|_| WorkChannelError::PacketHashFailed)?;
        self.outbound_frames.push(WsFrame {
            to,
            route_hash: envelope.route_hash,
            packet_hash: encoded.packet.hash,
            packet_bytes: encoded.packet.as_bytes().to_vec(),
            envelope_bytes,
        });
        self.routes.push_inbox(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.routes.drain_inbox(node_id)
    }
}
