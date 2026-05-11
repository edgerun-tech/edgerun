use alloc::string::String;
use alloc::vec::Vec;

use rkyv::{Archive, Deserialize, Serialize};

use crate::protocol::{Hash, NodeId, NodeIdentity, WorkSignature};

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayEndpoint {
    pub relay_node_id: NodeId,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NodeAvailable {
    pub abi_version: u16,
    pub node: NodeIdentity,
    pub sequence: u64,
    pub unix_ms: u64,
    pub listen_host: String,
    pub listen_port: u16,
    pub heartbeat_secs: u64,
    pub log_head: Hash,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NodeHeartbeat {
    pub abi_version: u16,
    pub node: NodeIdentity,
    pub sequence: u64,
    pub unix_ms: u64,
    pub connection_hash: Hash,
    pub log_head: Hash,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayPeerList {
    pub abi_version: u16,
    pub assigned_to: NodeId,
    pub relays: Vec<RelayEndpoint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayAssignment {
    pub abi_version: u16,
    pub node_id: NodeId,
    pub relay: RelayEndpoint,
    pub assigned_by: NodeIdentity,
    pub sequence: u64,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}
