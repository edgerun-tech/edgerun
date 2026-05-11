use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::channel::{ChannelEnvelope, RouteAdvertisement};
use crate::codec::{encode_work_packet_once, ArchivedWorkPacketFrame};
use crate::memory_channel::{route_hash, route_is_available};
use crate::protocol::{Hash, NodeId, WorkPacket, WORK_WIRE_ABI_VERSION};
use crate::route_auth::verify_route_advertisement;
use crate::route_plan::{RouteRuntimeProfile, RouteSelectionPolicy};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[cfg(feature = "std")]
fn current_unix_ms() -> u64 {
    crate::std_runtime::unix_ms()
}

#[cfg(not(feature = "std"))]
fn current_unix_ms() -> u64 {
    0
}

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
    routes: BTreeMap<NodeId, RouteAdvertisement>,
    inboxes: BTreeMap<NodeId, Vec<ChannelEnvelope>>,
}

impl<T> TransportWorkChannel<T> {
    pub fn new(transport: T, policy: RouteSelectionPolicy) -> Self {
        Self {
            transport,
            policy,
            routes: BTreeMap::new(),
            inboxes: BTreeMap::new(),
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
        let envelope = ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: frame.channel_id,
            from: frame.from,
            to: frame.to,
            route_hash: frame.route_hash,
            packet_hash,
            packet,
        };
        self.inboxes.entry(envelope.to).or_default().push(envelope);
        Ok(())
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }
}

impl<T: WorkPacketTransport> WorkChannel for TransportWorkChannel<T> {
    fn add_route(&mut self, route: RouteAdvertisement) -> Result<Hash, WorkChannelError> {
        if !verify_route_advertisement(&route)
            || !route_is_available(&route, current_unix_ms())
            || !self.policy.allows(&route)
        {
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
        let route = self.routes.get(node_id)?;
        route_is_available(route, current_unix_ms()).then(|| route_hash(route))
    }

    fn send_unordered(
        &mut self,
        from: NodeId,
        to: NodeId,
        packet: WorkPacket,
    ) -> Result<ChannelEnvelope, WorkChannelError> {
        let now = current_unix_ms();
        if self.routes.get(&to).is_some_and(|route| !route_is_available(route, now)) {
            self.routes.remove(&to);
            return Err(WorkChannelError::RouteMissing);
        }
        let route = self.routes.get(&to).ok_or(WorkChannelError::RouteMissing)?;
        if !self.policy.allows(route) {
            return Err(WorkChannelError::RouteInvalid);
        }
        let encoded = encode_work_packet_once(&packet).map_err(|_| WorkChannelError::PacketHashFailed)?;
        self.transport
            .send_packet_bytes(route, encoded.as_bytes())
            .map_err(|_| WorkChannelError::DeliveryFailed)?;
        Ok(ChannelEnvelope {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id: route.endpoint.channel_id,
            from,
            to,
            route_hash: route_hash(route),
            packet_hash: encoded.hash,
            packet,
        })
    }

    fn recv_all(&mut self, node_id: NodeId) -> Vec<ChannelEnvelope> {
        let _ = self.poll_recv();
        self.inboxes.entry(node_id).or_default().drain(..).collect()
    }
}
