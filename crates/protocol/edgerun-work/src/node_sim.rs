use alloc::vec::Vec;

use crate::channel::ChannelEnvelope;
use crate::channel_order::{ChannelOrderBook, ChannelOrderError, OrderedChannelEnvelope};
use crate::codec::{blake3_hash, packet_bytes};
use crate::identity::node_identity_from_key;
use crate::memory_channel::{MemoryChannelEngine, MemoryChannelError, route_hash};
use crate::protocol::*;
use crate::route_builder::{RouteAdvertisementBuilder, memory_endpoint};
use crate::signing::{empty_signature, sign_network_message};
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

    pub fn advertise_memory_route(
        &self,
        relay_node_id: NodeId,
        departments: Vec<u16>,
    ) -> crate::channel::RouteAdvertisement {
        RouteAdvertisementBuilder::new(
            &self.key,
            self.identity.role,
            memory_endpoint("memory", &self.identity.node_id),
        )
        .relay_node_id(relay_node_id)
        .departments(departments)
        .build(&self.key)
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
        let mut id_input = Vec::new();
        id_input.extend_from_slice(&self.identity.node_id);
        id_input.extend_from_slice(&to);
        id_input.extend_from_slice(&self.sequence.to_be_bytes());
        id_input.extend_from_slice(&payload_hash);
        WorkPacket::NetworkMessage(sign_network_message(
            &self.key,
            NetworkMessage {
                abi_version: WORK_WIRE_ABI_VERSION,
                message_id: blake3_hash(&id_input),
                prev_hash: [0u8; 32],
                from: self.identity.node_id,
                to,
                via_relay,
                department,
                work_type,
                sequence: self.sequence,
                payload_hash,
                payload,
                signature: empty_signature(),
            },
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
    let previous_message_hash = sender_order.last_message_hash(envelope.channel_id, from, to);
    let sequence = sender_order.next_sequence(envelope.channel_id, from, to);
    let ordered = OrderedChannelEnvelope {
        envelope,
        sequence,
        previous_message_hash,
    };
    sender_order
        .accept(&ordered, ordered.envelope.route_hash)
        .map_err(NodeSimError::Order)?;
    Ok(ordered)
}

pub fn envelope_packet_hash(envelope: &ChannelEnvelope) -> Result<Hash, NodeSimError> {
    packet_bytes(&envelope.packet)
        .map(|bytes| blake3_hash(&bytes))
        .map_err(|_| NodeSimError::PacketHash)
}

pub fn expected_route_hash(channel: &MemoryChannelEngine, node_id: &NodeId) -> Option<Hash> {
    channel.route_for(node_id).map(route_hash)
}
