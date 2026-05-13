use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::codec::{ArchivedWorkPacketFrame, encode_work_packet_once};
use crate::protocol::{Hash, NodeId, WorkPacket};
use crate::route_auth::{current_unix_ms, route_hash};
use crate::route_plan::{RouteRuntimeProfile, RouteSelectionPolicy};
use crate::route_table::{
    RouteInboxMap, RouteMap, drain_inbox, insert_live_route_with_inbox, live_route_hash_for,
    remove_route_with_inbox, route_for_send,
};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkTransportError {
    UnsupportedRoute,
    DeliveryFailed,
    Backpressure,
    FrameTooLarge,
    InvalidFrame,
}

pub struct TransportPacketFrame {
    pub channel_id: Hash,
    pub from: NodeId,
    pub to: NodeId,
    pub route_hash: Hash,
    pub frame: ArchivedWorkPacketFrame,
}

impl core::fmt::Debug for TransportPacketFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TransportPacketFrame")
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
        route: &RouteAdvertisement,
        packet_bytes: &[u8],
    ) -> Result<(), WorkTransportError>;

    fn recv_packet_frame(&mut self) -> Result<Option<TransportPacketFrame>, WorkTransportError> {
        Ok(None)
    }
}

#[derive(Debug)]
pub struct TransportWorkChannel<T> {
    transport: T,
    policy: RouteSelectionPolicy,
    routes: RouteMap,
    inboxes: RouteInboxMap,
}

impl<T> TransportWorkChannel<T> {
    pub fn new(transport: T, policy: RouteSelectionPolicy) -> Self {
        Self {
            transport,
            policy,
            routes: RouteMap::new(),
            inboxes: RouteInboxMap::new(),
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

    pub fn poll_recv(&mut self) -> Result<usize, WorkTransportError>
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
        frame: TransportPacketFrame,
    ) -> Result<(), WorkTransportError> {
        let packet_hash = frame.frame.hash;
        let packet = frame
            .frame
            .into_packet()
            .map_err(|_| WorkTransportError::InvalidFrame)?;
        let envelope = ChannelEnvelope::new(
            frame.channel_id,
            frame.from,
            frame.to,
            frame.route_hash,
            packet_hash,
            packet,
        );
        self.inboxes.entry(envelope.to).or_default().push(envelope);
        Ok(())
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl<T: WorkPacketTransport> WorkChannel for TransportWorkChannel<T> {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !self.policy.allows(&route) {
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
        if !self.policy.allows(route) {
            return Err(WorkChannelError::RouteInvalid);
        }
        let encoded =
            encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        self.transport
            .send_packet_bytes(route, encoded.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(ChannelEnvelope::for_route(
            route,
            route_hash(route),
            from,
            to,
            encoded.hash,
            packet,
        ))
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        let _ = self.poll_recv();
        drain_inbox(&mut self.inboxes, node_id)
    }
}
