use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel_order::OrderedChannelEnvelope;
use crate::codec::{blake3_hash, empty_signature, encode_work_packet_once, node_identity_from_key, sign_work_receipt};
use crate::memory_channel::{MemoryChannelEngine, MemoryChannelError};
use crate::protocol::*;
use crate::settlement::receipt_id_for_claim;
use crate::transit_proof::{packet_transit_hash, PacketTransitHashInput};
use crate::work_channel::{WorkChannel, WorkChannelError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayRoleError {
    NotRelay,
    WrongRelay,
    NotNetworkMessage,
    DestinationRouteMissing,
    PacketSerializationFailed,
    DeliveryFailed,
    LegacyMemoryDelivery(MemoryChannelError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayDeliveryResult {
    pub delivered_to: NodeId,
    pub destination_route_hash: Hash,
    pub forwarded_packet_hash: Hash,
    pub transit_hash: Hash,
    pub receipt: WorkReceipt,
}

pub struct RelayRole {
    pub key: Ed25519SigningKey,
    pub identity: NodeIdentity,
    pub sequence: u64,
    pub price_per_message: u64,
    pub last_transit_hash: Hash,
}

impl RelayRole {
    pub fn from_seed(seed: u8, price_per_message: u64) -> Self {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let identity = node_identity_from_key(&key, NODE_ROLE_RELAY);
        Self {
            key,
            identity,
            sequence: 0,
            price_per_message,
            last_transit_hash: [0u8; 32],
        }
    }

    pub fn from_key(key: Ed25519SigningKey, price_per_message: u64) -> Self {
        let identity = node_identity_from_key(&key, NODE_ROLE_RELAY);
        Self {
            key,
            identity,
            sequence: 0,
            price_per_message,
            last_transit_hash: [0u8; 32],
        }
    }

    pub fn forward_ordered_on<C: WorkChannel>(
        &mut self,
        channel: &mut C,
        ordered: &OrderedChannelEnvelope,
        request_hash: Hash,
        admission_hash: Hash,
    ) -> Result<RelayDeliveryResult, RelayRoleError> {
        if self.identity.role != NODE_ROLE_RELAY {
            return Err(RelayRoleError::NotRelay);
        }
        let WorkPacket::NetworkMessage(message) = &ordered.envelope.packet else {
            return Err(RelayRoleError::NotNetworkMessage);
        };
        if message.via_relay != self.identity.node_id || ordered.envelope.to != self.identity.node_id {
            return Err(RelayRoleError::WrongRelay);
        }
        let destination_route_hash = channel
            .route_hash_for(&message.to)
            .ok_or(RelayRoleError::DestinationRouteMissing)?;
        let forwarded_packet = ordered.envelope.packet.clone();
        let forwarded_packet_hash = encode_work_packet_once(&forwarded_packet)
            .map(|encoded| encoded.hash)
            .map_err(|_| RelayRoleError::PacketSerializationFailed)?;
        channel
            .send_unordered(self.identity.node_id, message.to, forwarded_packet)
            .map_err(|_| RelayRoleError::DeliveryFailed)?;
        Ok(self.finish_delivery_receipt(
            message.to,
            destination_route_hash,
            forwarded_packet_hash,
            ordered,
            request_hash,
            admission_hash,
        ))
    }

    pub fn forward_ordered(
        &mut self,
        channel: &mut MemoryChannelEngine,
        ordered: &OrderedChannelEnvelope,
        request_hash: Hash,
        admission_hash: Hash,
    ) -> Result<RelayDeliveryResult, RelayRoleError> {
        self.forward_ordered_on(channel, ordered, request_hash, admission_hash)
    }

    fn finish_delivery_receipt(
        &mut self,
        delivered_to: NodeId,
        destination_route_hash: Hash,
        forwarded_packet_hash: Hash,
        ordered: &OrderedChannelEnvelope,
        request_hash: Hash,
        admission_hash: Hash,
    ) -> RelayDeliveryResult {
        self.sequence = self.sequence.saturating_add(1);
        let input_hash = ordered_message_input_hash(ordered);
        let transit_hash = packet_transit_hash(&PacketTransitHashInput {
            node_id: self.identity.node_id,
            from: ordered.envelope.from,
            to: delivered_to,
            channel_id: ordered.envelope.channel_id,
            route_hash: destination_route_hash,
            packet_hash: forwarded_packet_hash,
            sequence: self.sequence,
            previous_transit_hash: self.last_transit_hash,
        });
        self.last_transit_hash = transit_hash;
        let receipt_id = receipt_id_for_claim(
            request_hash,
            admission_hash,
            self.identity.node_id,
            input_hash,
            transit_hash,
            self.sequence,
        );
        let receipt = sign_work_receipt(
            &self.key,
            WorkReceipt {
                abi_version: WORK_WIRE_ABI_VERSION,
                receipt_id,
                request_hash,
                admission_hash,
                worker: self.identity.clone(),
                relay_node_id: self.identity.node_id,
                input_hash,
                output_hash: transit_hash,
                units_used: 1,
                total_claim: self.price_per_message,
                sequence: self.sequence,
                signature: empty_signature(),
            },
        );
        RelayDeliveryResult {
            delivered_to,
            destination_route_hash,
            forwarded_packet_hash,
            transit_hash,
            receipt,
        }
    }
}

impl From<WorkChannelError> for RelayRoleError {
    fn from(_value: WorkChannelError) -> Self {
        RelayRoleError::DeliveryFailed
    }
}

pub fn ordered_message_input_hash(ordered: &OrderedChannelEnvelope) -> Hash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&ordered.envelope.channel_id);
    bytes.extend_from_slice(&ordered.envelope.from);
    bytes.extend_from_slice(&ordered.envelope.to);
    bytes.extend_from_slice(&ordered.sequence.to_be_bytes());
    bytes.extend_from_slice(&ordered.previous_message_hash);
    bytes.extend_from_slice(&ordered.envelope.packet_hash);
    blake3_hash(&bytes)
}
