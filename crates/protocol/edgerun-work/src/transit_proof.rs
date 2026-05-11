use alloc::vec::Vec;

use crate::codec::blake3_hash;
use crate::protocol::{Hash, NodeId};

const PACKET_TRANSIT_DOMAIN: &[u8] = b"edgerun:v1:work:packet-transit";
const PACKET_TRANSIT_CHAIN_DOMAIN: &[u8] = b"edgerun:v1:work:packet-transit-chain";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketTransitHashInput {
    pub node_id: NodeId,
    pub from: NodeId,
    pub to: NodeId,
    pub channel_id: Hash,
    pub route_hash: Hash,
    pub packet_hash: Hash,
    pub sequence: u64,
    pub previous_transit_hash: Hash,
}

pub fn packet_transit_hash(input: &PacketTransitHashInput) -> Hash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PACKET_TRANSIT_DOMAIN);
    bytes.push(0);
    bytes.extend_from_slice(&input.node_id);
    bytes.extend_from_slice(&input.from);
    bytes.extend_from_slice(&input.to);
    bytes.extend_from_slice(&input.channel_id);
    bytes.extend_from_slice(&input.route_hash);
    bytes.extend_from_slice(&input.packet_hash);
    bytes.extend_from_slice(&input.sequence.to_be_bytes());
    bytes.extend_from_slice(&input.previous_transit_hash);
    blake3_hash(&bytes)
}

pub fn packet_transit_chain_hash(transit_hashes: &[Hash]) -> Hash {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PACKET_TRANSIT_CHAIN_DOMAIN);
    bytes.push(0);
    bytes.extend_from_slice(&(transit_hashes.len() as u64).to_be_bytes());
    for hash in transit_hashes {
        bytes.extend_from_slice(hash);
    }
    blake3_hash(&bytes)
}
