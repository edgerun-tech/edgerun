use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteAdvertisement, CHANNEL_KIND_WEBSOCKET};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::{blake3_hash, packet_bytes};
use crate::frame_codec::channel_envelope_bytes;
use crate::memory_channel::route_hash;
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, Default)]
pub struct WsWorkChannel {
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inboxes: BTreeMap<NodeId, Vec<ChannelEnvelope>>,
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
        self.inboxes.entry(envelope.to).or_default().push(envelope);
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
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !verify_route_advertisement(&route) || route.endpoint.kind != CHANNEL_KIND_WEBSOCKET {
            return Err(WorkChannelError::RouteInvalid);
        }
        let node_id = route.node.node_id;
        let hash = route_hash(&route);
        self.routes.insert(node_id, route);
        self.inboxes.entry(node_id).or_default();
        Ok(hash)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        self.inboxes.remove(&node_id);
        self.routes.remove(&node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        self.routes.get(node_id).map(route_hash)
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let route = self.routes.get(&to).ok_or(WorkChannelError::RouteMissing)?;
        let packet_bytes = packet_bytes(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        let packet_hash = blake3_hash(&packet_bytes);
        let envelope = ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash,
            packet,
        };
        let envelope_bytes = channel_envelope_bytes(&envelope)
            .map_err(|_| WorkChannelError::PacketHashFailed)?;
        self.outbound_frames.push(WsFrame {
            to,
            route_hash: envelope.route_hash,
            packet_hash,
            packet_bytes,
            envelope_bytes,
        });
        self.inboxes.entry(to).or_default().push(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.inboxes.entry(node_id).or_default().drain(..).collect()
    }
}
