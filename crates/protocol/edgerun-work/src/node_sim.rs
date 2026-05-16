use alloc::vec;
use alloc::vec::Vec;

use crate::channel::ChannelEnvelope;
use crate::channel_order::{ChannelOrderBook, ChannelOrderError, OrderedChannelEnvelope};
use crate::codec::{blake3_hash, packet_bytes};
use crate::identity::node_identity_from_key;
use crate::memory_channel::{MemoryChannelEngine, MemoryChannelError};
use crate::protocol::*;
use crate::route_binding::route_hash;
use crate::route_builder::{memory_endpoint, route_binding};
use crate::signing::{sign_network_message_payload, simple_network_message_id};
use edgerun_crypto::Ed25519SigningKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeSimError {
    Route(MemoryChannelError),
    Order(ChannelOrderError),
    PacketHash,
}

pub struct SimNode {
    pub key: Ed25519SigningKey,
    pub identity: NodeIdentity,
    pub order: ChannelOrderBook,
    pub sequence: u64,
}

impl SimNode {
    pub fn from_seed(seed: u8, role: u16) -> Self {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let identity = node_identity_from_key(&key, role);
        Self {
            key,
            identity,
            order: ChannelOrderBook::new(),
            sequence: 0,
        }
    }

    pub fn bind_memory_route(
        &self,
        relay_node_id: NodeId,
        departments: Vec<u16>,
    ) -> crate::channel::RouteBinding {
        route_binding(
            &self.key,
            self.identity.role,
            memory_endpoint("memory", &self.identity.node_id),
            relay_node_id,
            departments,
            u64::MAX,
        )
    }

    pub fn bind_memory_capability(&self, relay_node_id: NodeId) -> crate::channel::RouteBinding {
        route_binding(
            &self.key,
            NODE_ROLE_CAPABILITY,
            memory_endpoint("capability", &self.identity.node_id),
            relay_node_id,
            vec![DEPARTMENT_CAPABILITY],
            u64::MAX,
        )
    }

    pub fn message_to(
        &mut self,
        to: NodeId,
        via_relay: NodeId,
        department: u16,
        work_type: u16,
        payload: Vec<u8>,
    ) -> WorkPacket {
        self.sequence = self.sequence.saturating_add(1);
        let payload_hash = blake3_hash(&payload);
        let message_id =
            simple_network_message_id(&self.identity.node_id, &to, self.sequence, &payload_hash);
        WorkPacket::NetworkMessage(sign_network_message_payload(
            &self.key,
            message_id,
            [0u8; 32],
            self.identity.node_id,
            to,
            via_relay,
            department,
            work_type,
            self.sequence,
            payload,
        ))
    }

    pub fn accept_ordered(
        &mut self,
        ordered: &OrderedChannelEnvelope,
        expected_route_hash: Hash,
    ) -> Result<Hash, NodeSimError> {
        self.order
            .accept(ordered, expected_route_hash)
            .map_err(NodeSimError::Order)
    }
}

pub fn deliver_ordered(
    channel: &mut MemoryChannelEngine,
    sender_order: &mut ChannelOrderBook,
    from: NodeId,
    to: NodeId,
    packet: WorkPacket,
) -> Result<OrderedChannelEnvelope, NodeSimError> {
    let envelope = channel
        .deliver(from, to, packet)
        .map_err(NodeSimError::Route)?;
    let route_hash = envelope.route_hash;
    sender_order
        .accept_envelope(envelope, route_hash)
        .map_err(NodeSimError::Order)
}

pub fn envelope_packet_hash(envelope: &ChannelEnvelope) -> Result<Hash, NodeSimError> {
    packet_bytes(&envelope.packet)
        .map(|bytes| blake3_hash(&bytes))
        .map_err(|_| NodeSimError::PacketHash)
}

pub fn expected_route_hash(channel: &MemoryChannelEngine, node_id: &NodeId) -> Option<Hash> {
    channel.route_for(node_id).map(route_hash)
}
