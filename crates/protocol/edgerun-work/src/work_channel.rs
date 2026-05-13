use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::channel_order::{ChannelOrderBook, ChannelOrderError, OrderedChannelEnvelope};
use crate::memory_channel::{MemoryChannelEngine, MemoryChannelError};
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_auth::route_hash;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkChannelError {
    RouteInvalid,
    RouteMissing,
    DeliveryFailed,
    Order(ChannelOrderError),
    PacketHashFailed,
}

impl From<MemoryChannelError> for WorkChannelError {
    fn from(error: MemoryChannelError) -> Self {
        match error {
            MemoryChannelError::RouteInvalid => Self::RouteInvalid,
            MemoryChannelError::RouteMissing => Self::RouteMissing,
            MemoryChannelError::PacketHashFailed => Self::PacketHashFailed,
            MemoryChannelError::InboxMissing => Self::DeliveryFailed,
        }
    }
}

pub trait WorkChannel {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError>;
    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement>;
    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash>;
    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError>;
    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope>;
}

pub trait OrderedWorkChannel: WorkChannel {
    fn send_ordered(
        &mut self,
        order: &mut ChannelOrderBook,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<OrderedChannelEnvelope, WorkChannelError> {
        let envelope = self.send_unordered(from, to, packet)?;
        let previous_message_hash = order.last_message_hash(envelope.channel_id, from, to);
        let sequence = order.next_sequence(envelope.channel_id, from, to);
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

impl<T: WorkChannel> OrderedWorkChannel for T {}

impl WorkChannel for MemoryChannelEngine {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        MemoryChannelEngine::add_route(self, route).map_err(Into::into)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        MemoryChannelEngine::remove_route(self, node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        self.route_for(node_id).map(route_hash)
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        self.deliver(from, to, packet).map_err(Into::into)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        self.drain_inbox(node_id)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryWorkChannel {
    engine: MemoryChannelEngine,
}

impl MemoryWorkChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_engine(engine: MemoryChannelEngine) -> Self {
        Self { engine }
    }

    pub fn into_engine(self) -> MemoryChannelEngine {
        self.engine
    }

    pub fn engine(&self) -> &MemoryChannelEngine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut MemoryChannelEngine {
        &mut self.engine
    }
}

impl WorkChannel for MemoryWorkChannel {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        WorkChannel::add_route(&mut self.engine, route)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteAdvertisement> {
        WorkChannel::remove_route(&mut self.engine, node_id)
    }

    fn route_hash_for(&self, node_id: &NodeId) -> Option<Hash> {
        WorkChannel::route_hash_for(&self.engine, node_id)
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        WorkChannel::send_unordered(&mut self.engine, from, to, packet)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        WorkChannel::recv_all(&mut self.engine, node_id)
    }
}
