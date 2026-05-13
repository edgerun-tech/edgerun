use alloc::string::String;
use alloc::vec::Vec;

use rkyv::{Archive, Deserialize, Serialize};

use crate::protocol::{
    Hash, NodeId, NodeIdentity, WORK_WIRE_ABI_VERSION, WorkPacket, WorkSignature,
};

pub const CHANNEL_KIND_MEMORY: u16 = 1;
pub const CHANNEL_KIND_TCP: u16 = 2;
pub const CHANNEL_KIND_WEBSOCKET: u16 = 3;
pub const CHANNEL_KIND_WEBRTC: u16 = 4;
pub const CHANNEL_KIND_BLUETOOTH: u16 = 5;
pub const CHANNEL_KIND_SERIAL: u16 = 6;
pub const CHANNEL_KIND_WASM_HOST: u16 = 7;
pub const CHANNEL_KIND_QUIC: u16 = 8;
pub const CHANNEL_KIND_WEBTRANSPORT: u16 = 9;

pub type ChannelId = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ChannelEndpoint {
    pub abi_version: u16,
    pub channel_id: ChannelId,
    pub kind: u16,
    pub address: Vec<u8>,
    pub label: String,
}

impl ChannelEndpoint {
    pub fn new(channel_id: ChannelId, kind: u16, address: Vec<u8>, label: String) -> Self {
        Self {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id,
            kind,
            address,
            label,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
/// Runtime route binding installed by admission/runtime state.
///
/// This is derived state, not a signed node claim. Work authority still comes
/// from the signed `WorkRequest` and signed `WorkAdmission`.
pub struct RouteBinding {
    pub abi_version: u16,
    pub node: NodeIdentity,
    pub relay_node_id: NodeId,
    pub endpoint: ChannelEndpoint,
    pub roles: Vec<u16>,
    pub departments: Vec<u16>,
    pub valid_until_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ChannelEnvelope {
    pub abi_version: u16,
    pub channel_id: ChannelId,
    pub from: NodeId,
    pub to: NodeId,
    pub route_hash: Hash,
    pub packet_hash: Hash,
    pub packet: WorkPacket,
}

impl ChannelEnvelope {
    pub fn new(
        channel_id: ChannelId,
        from: NodeId,
        to: NodeId,
        route_hash: Hash,
        packet_hash: Hash,
        packet: WorkPacket,
    ) -> Self {
        Self {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id,
            from,
            to,
            route_hash,
            packet_hash,
            packet,
        }
    }

    pub fn for_route(
        route: &RouteBinding,
        route_hash: Hash,
        from: NodeId,
        to: NodeId,
        packet_hash: Hash,
        packet: WorkPacket,
    ) -> Self {
        Self::new(
            route.endpoint.channel_id,
            from,
            to,
            route_hash,
            packet_hash,
            packet,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
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
