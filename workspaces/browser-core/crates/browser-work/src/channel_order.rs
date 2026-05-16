use alloc::collections::BTreeMap;

use crate::channel::ChannelEnvelope;
use crate::preimage::HashBuilder;
use crate::protocol::{ChannelId, Hash, NodeId, WORK_WIRE_ABI_VERSION};

const ORDERED_MESSAGE_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:ordered-message";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelOrderError {
    BadAbi,
    PacketHashMismatch,
    RouteHashMismatch,
    SequenceOutOfOrder,
    PreviousHashMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedChannelEnvelope {
    pub envelope: ChannelEnvelope,
    pub sequence: u64,
    pub previous_message_hash: Hash,
}

#[derive(Clone, Debug, Default)]
pub struct ChannelOrderBook {
    streams: BTreeMap<StreamKey, StreamState>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StreamKey {
    channel_id: ChannelId,
    from: NodeId,
    to: NodeId,
}

#[derive(Clone, Debug)]
struct StreamState {
    next_sequence: u64,
    last_message_hash: Hash,
}

impl ChannelOrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept(
        &mut self,
        ordered: &OrderedChannelEnvelope,
        expected_route_hash: Hash,
        expected_packet_hash: Hash,
    ) -> Result<Hash, ChannelOrderError> {
        if ordered.envelope.abi_version != WORK_WIRE_ABI_VERSION {
            return Err(ChannelOrderError::BadAbi);
        }
        if ordered.envelope.route_hash != expected_route_hash {
            return Err(ChannelOrderError::RouteHashMismatch);
        }
        if ordered.envelope.packet_hash != expected_packet_hash {
            return Err(ChannelOrderError::PacketHashMismatch);
        }

        let key = StreamKey {
            channel_id: ordered.envelope.channel_id,
            from: ordered.envelope.from,
            to: ordered.envelope.to,
        };
        let state = self.streams.entry(key).or_insert(StreamState {
            next_sequence: 1,
            last_message_hash: [0u8; 32],
        });
        if ordered.sequence != state.next_sequence {
            return Err(ChannelOrderError::SequenceOutOfOrder);
        }
        if ordered.previous_message_hash != state.last_message_hash {
            return Err(ChannelOrderError::PreviousHashMismatch);
        }
        let message_hash = ordered_message_hash(ordered);
        state.next_sequence = state.next_sequence.saturating_add(1);
        state.last_message_hash = message_hash;
        Ok(message_hash)
    }

    pub fn wrap_and_accept(
        &mut self,
        envelope: ChannelEnvelope,
        expected_route_hash: Hash,
        expected_packet_hash: Hash,
    ) -> Result<OrderedChannelEnvelope, ChannelOrderError> {
        let sequence = self.next_sequence(envelope.channel_id, envelope.from, envelope.to);
        let previous_message_hash =
            self.last_message_hash(envelope.channel_id, envelope.from, envelope.to);
        let ordered = OrderedChannelEnvelope {
            envelope,
            sequence,
            previous_message_hash,
        };
        self.accept(&ordered, expected_route_hash, expected_packet_hash)?;
        Ok(ordered)
    }

    pub fn last_message_hash(&self, channel_id: ChannelId, from: NodeId, to: NodeId) -> Hash {
        self.streams
            .get(&StreamKey {
                channel_id,
                from,
                to,
            })
            .map(|state| state.last_message_hash)
            .unwrap_or([0; 32])
    }

    pub fn next_sequence(&self, channel_id: ChannelId, from: NodeId, to: NodeId) -> u64 {
        self.streams
            .get(&StreamKey {
                channel_id,
                from,
                to,
            })
            .map(|state| state.next_sequence)
            .unwrap_or(1)
    }
}

pub fn ordered_message_hash(ordered: &OrderedChannelEnvelope) -> Hash {
    HashBuilder::domain(ORDERED_MESSAGE_HASH_DOMAIN)
        .hash(&ordered.envelope.channel_id)
        .node_id(&ordered.envelope.from)
        .node_id(&ordered.envelope.to)
        .u64(ordered.sequence)
        .hash(&ordered.previous_message_hash)
        .hash(&ordered.envelope.route_hash)
        .hash(&ordered.envelope.packet_hash)
        .finish()
}
