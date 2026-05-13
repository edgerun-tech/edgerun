use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteBinding};
use crate::codec::ArchivedWorkPacketFrame;
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_binding::current_unix_ms;
use crate::route_policy::{RouteRuntimeProfile, RouteSelectionPolicy};
use crate::route_table::RouteState;
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayTransportError {
    UnsupportedRoute,
    DeliveryFailed,
    Backpressure,
    FrameTooLarge,
    InvalidFrame,
}

pub struct RelayPacketFrame {
    pub channel_id: Hash,
    pub from: NodeId,
    pub to: NodeId,
    pub route_hash: Hash,
    pub frame: ArchivedWorkPacketFrame,
}

impl core::fmt::Debug for RelayPacketFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RelayPacketFrame")
            .field("channel_id", &self.channel_id)
            .field("from", &self.from)
            .field("to", &self.to)
            .field("route_hash", &self.route_hash)
            .field("packet_hash", &self.frame.hash)
            .finish()
    }
}

pub trait WorkPacketTransport {
    fn send_packet_bytes(
        &mut self,
        route: &RouteBinding,
        packet_bytes: &[u8],
    ) -> Result<(), RelayTransportError>;

    fn recv_packet_frame(&mut self) -> Result<Option<RelayPacketFrame>, RelayTransportError> {
        Ok(None)
    }
}

#[derive(Debug)]
pub struct RelayWorkChannel<T> {
    transport: T,
    policy: RouteSelectionPolicy,
    routes: RouteState,
}

impl<T> RelayWorkChannel<T> {
    pub fn new(transport: T, policy: RouteSelectionPolicy) -> Self {
        Self {
            transport,
            policy,
            routes: RouteState::new(),
        }
    }

    pub fn native(transport: T) -> Self {
        Self::new(transport, RouteSelectionPolicy::native())
    }

    pub fn browser(transport: T) -> Self {
        Self::new(transport, RouteSelectionPolicy::browser())
    }

    pub fn wasm_host(transport: T) -> Self {
        Self::new(transport, RouteSelectionPolicy::wasm_host())
    }

    pub fn for_profile(transport: T, profile: RouteRuntimeProfile) -> Self {
        Self::new(transport, RouteSelectionPolicy::for_profile(profile))
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn policy(&self) -> &RouteSelectionPolicy {
        &self.policy
    }

    pub fn poll_recv(&mut self) -> Result<usize, RelayTransportError>
    where
        T: WorkPacketTransport,
    {
        let mut accepted = 0usize;
        while let Some(frame) = self.transport.recv_packet_frame()? {
            self.accept_transport_frame(frame)?;
            accepted = accepted.saturating_add(1);
        }
        Ok(accepted)
    }

    pub fn accept_transport_frame(
        &mut self,
        frame: RelayPacketFrame,
    ) -> Result<(), RelayTransportError> {
        let packet_hash = frame.frame.hash;
        let packet = frame
            .frame
            .into_packet()
            .map_err(|_| RelayTransportError::InvalidFrame)?;
        let envelope = ChannelEnvelope::new(
            frame.channel_id,
            frame.from,
            frame.to,
            frame.route_hash,
            packet_hash,
            packet,
        );
        self.routes.push_inbox(envelope);
        Ok(())
    }

    pub fn route_count(&self) -> usize {
        self.routes.route_count()
    }
}

impl<T: WorkPacketTransport> WorkChannel for RelayWorkChannel<T> {
    fn add_route(&mut self, route: RouteBinding) -> Result<Hash, WorkChannelError> {
        if !self.policy.allows(&route) {
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
        let (route, encoded) = self
            .routes
            .encode_for_send_with_route(from, to, packet, current_unix_ms())
            .map_err(WorkChannelError::from)?;
        if !self.policy.allows(&route) {
            return Err(WorkChannelError::RouteInvalid);
        }
        self.transport
            .send_packet_bytes(&route, encoded.packet.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(encoded.envelope)
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        let _ = self.poll_recv();
        self.routes.drain_inbox(node_id)
    }
}
