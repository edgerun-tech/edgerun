use alloc::vec::Vec;

use crate::channel::{CHANNEL_KIND_WEBSOCKET, ChannelEnvelope, RouteAdvertisement};
use crate::channel_order::{ChannelOrderBook, OrderedChannelEnvelope};
use crate::codec::encode_work_packet_once;
use crate::frame_codec::channel_envelope_bytes;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_auth::{current_unix_ms, route_hash};
use crate::route_table::{
    RouteInboxMap, RouteMap, drain_inbox, insert_live_route_with_inbox, live_route_hash_for,
    remove_route_with_inbox, route_for_send,
};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, Default)]
pub struct WsWorkChannel {
    routes: RouteMap,
    inboxes: RouteInboxMap,
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
        if route.endpoint.kind != CHANNEL_KIND_WEBSOCKET {
            return Err(WorkChannelError::RouteInvalid);
        }
        let hash = insert_live_route_with_inbox(
            &mut self.routes,
            &mut self.inboxes,
            route,
            current_unix_ms(),
        )
        .ok_or(WorkChannelError::RouteInvalid)?;
        Ok(hash)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        remove_route_with_inbox(&mut self.routes, &mut self.inboxes, node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        live_route_hash_for(&self.routes, node_id, current_unix_ms())
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let route = route_for_send(&mut self.routes, &to, current_unix_ms())
            .ok_or(WorkChannelError::RouteMissing)?;
        let encoded =
            encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        let envelope =
            ChannelEnvelope::for_route(route, route_hash(route), from, to, encoded.hash, packet);
        let envelope_bytes =
            channel_envelope_bytes(&envelope).map_err(|_| WorkChannelError::PacketHashFailed)?;
        self.outbound_frames.push(WsFrame {
            to,
            route_hash: envelope.route_hash,
            packet_hash: encoded.hash,
            packet_bytes: encoded.as_bytes().to_vec(),
            envelope_bytes,
        });
        self.inboxes.entry(to).or_default().push(envelope.clone());
        Ok(envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        drain_inbox(&mut self.inboxes, node_id)
    }
}
