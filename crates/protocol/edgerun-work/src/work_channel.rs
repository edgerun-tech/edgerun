use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::channel_order::{ChannelOrderBook, ChannelOrderError, OrderedChannelEnvelope};
use crate::memory_channel::{MemoryChannelEngine, MemoryChannelError};
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_binding::route_hash;
use crate::route_table::RouteStateSendError;
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
        }
    }
}

impl From<RouteStateSendError> for WorkChannelError {
    fn from(error: RouteStateSendError) -> Self {
        match error {
            RouteStateSendError::RouteMissing => Self::RouteMissing,
            RouteStateSendError::PacketHashFailed => Self::PacketHashFailed,
        }
    }
}

pub trait WorkChannel {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError>;
    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding>;
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
        let route_hash = envelope.route_hash;
        order
            .accept_envelope(envelope, route_hash)
            .map_err(WorkChannelError::Order)
    }
}

impl<T: WorkChannel> OrderedWorkChannel for T {}

impl WorkChannel for MemoryChannelEngine {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError> {
        MemoryChannelEngine::add_route(self, route).map_err(Into::into)
    }

    fn remove_route(&mut self, node_id: NodeId) -> Option<RouteBinding> {
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
