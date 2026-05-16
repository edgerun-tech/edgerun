use crate::protocol::{ChannelId, Hash, NodeId, WorkSignature, WORK_WIRE_ABI_VERSION};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelEnvelope {
    pub abi_version: u16,
    pub channel_id: ChannelId,
    pub from: NodeId,
    pub to: NodeId,
    pub route_hash: Hash,
    pub packet_hash: Hash,
}

impl ChannelEnvelope {
    pub fn new(
        channel_id: ChannelId,
        from: NodeId,
        to: NodeId,
        route_hash: Hash,
        packet_hash: Hash,
    ) -> Self {
        Self {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id,
            from,
            to,
            route_hash,
            packet_hash,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelProof {
    pub abi_version: u16,
    pub channel_id: ChannelId,
    pub relay_node_id: NodeId,
    pub from: NodeId,
    pub to: NodeId,
    pub message_hash: Hash,
    pub sequence: u64,
    pub signature: WorkSignature,
}
