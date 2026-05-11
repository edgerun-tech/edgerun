use alloc::string::String;
use alloc::vec::Vec;

use rkyv::{Archive, Deserialize, Serialize};

use crate::protocol::{Hash, NodeId, NodeIdentity, WorkPacket, WorkSignature, WORK_WIRE_ABI_VERSION};

pub const CHANNEL_KIND_MEMORY: u16 = 1;
pub const CHANNEL_KIND_TCP: u16 = 2;
pub const CHANNEL_KIND_WEBSOCKET: u16 = 3;
pub const CHANNEL_KIND_WEBRTC: u16 = 4;
pub const CHANNEL_KIND_BLUETOOTH: u16 = 5;
pub const CHANNEL_KIND_SERIAL: u16 = 6;
pub const CHANNEL_KIND_WASM_HOST: u16 = 7;
pub const CHANNEL_KIND_QUIC: u16 = 8;
pub const CHANNEL_KIND_WEBTRANSPORT: u16 = 9;

pub const ROUTE_STATUS_AVAILABLE: u16 = 1;
pub const ROUTE_STATUS_DRAINING: u16 = 2;
pub const ROUTE_STATUS_UNAVAILABLE: u16 = 3;

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
        Self { abi_version: WORK_WIRE_ABI_VERSION, channel_id, kind, address, label }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RouteAdvertisement {
    pub abi_version: u16,
    pub node: NodeIdentity,
    pub relay_node_id: NodeId,
    pub endpoint: ChannelEndpoint,
    pub roles: Vec<u16>,
    pub departments: Vec<u16>,
    pub status: u16,
    pub sequence: u64,
    pub valid_until_unix_ms: u64,
    pub previous_route_hash: Hash,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RouteSnapshot {
    pub abi_version: u16,
    pub issued_by: NodeIdentity,
    pub sequence: u64,
    pub routes: Vec<RouteAdvertisement>,
    pub route_root: Hash,
    pub signature: WorkSignature,
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
