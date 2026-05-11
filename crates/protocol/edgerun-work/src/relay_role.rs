use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel_order::OrderedChannelEnvelope;
use crate::codec::{blake3_hash, empty_signature, node_identity_from_key, sign_work_receipt};
use crate::memory_channel::{MemoryChannelEngine, MemoryChannelError};
use crate::protocol::*;
use crate::settlement::receipt_id_for_claim;
use crate::work_channel::{MemoryWorkChannel, WorkChannel, WorkChannelError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelayRoleError {
    NotRelay,
    WrongRelay,
    NotNetworkMessage,
    DestinationRouteMissing,
    DeliveryFailed,
    LegacyMemoryDelivery(MemoryChannelError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayDeliveryResult {
    pub delivered_to: NodeId,
    pub destination_route_hash: Hash,
    pub forwarded_packet_hash: Hash,
    pub receipt: WorkReceipt,
}

pub struct RelayRole {
    pub key: Ed25519SigningKey,
    pub identity: NodeIdentity,
    pub sequence: u64,
    pub price_per_message: u64,
}

impl RelayRole {
    pub fn from_seed(seed: u8, price_per_message: u64) -> Self {
        let key = Ed25519SigningKey::from_bytes(&[seed; 32]);
        let identity = node_identity_from_key(&key, NODE_ROLE_RELAY);
        Self { key, identity, sequence: 0, price_per_message }
    }

    pub fn from_key(key: Ed25519SigningKey, price_per_message: u64) -> Self {
        let identity = node_identity_from_key(&key, NODE_ROLE_RELAY);
        Self { key, identity, sequence: 0, price_per_message }
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
        let forwarded_packet_hash = blake3_hash(&crate::codec::packet_bytes(&forwarded_packet).unwrap_or_default());
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
        let mut adapter = MemoryWorkChannel::from_engine(channel.clone());
        let result = self.forward_ordered_on(&mut adapter, ordered, request_hash, admission_hash)?;
        *channel = adapter.into_engine();
        Ok(result)
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
        let receipt_id = receipt_id_for_claim(
            request_hash,
            admission_hash,
            self.identity.node_id,
            ordered_message_input_hash(ordered),
            forwarded_packet_hash,
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
                input_hash: ordered_message_input_hash(ordered),
                output_hash: forwarded_packet_hash,
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
