use crate::preimage::HashBuilder;
use crate::protocol::{Hash, NodeId};

const PACKET_TRANSIT_DOMAIN: &[u8] = b"edgerun:v1:work:packet-transit";
const PACKET_TRANSIT_CHAIN_DOMAIN: &[u8] = b"edgerun:v1:work:packet-transit-chain";
const RELAY_DELIVERY_OUTPUT_DOMAIN: &[u8] = b"edgerun:v1:work:relay-delivery-output";

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
    HashBuilder::domain(PACKET_TRANSIT_DOMAIN)
        .node_id(&input.node_id)
        .node_id(&input.from)
        .node_id(&input.to)
        .hash(&input.channel_id)
        .hash(&input.route_hash)
        .hash(&input.packet_hash)
        .u64(input.sequence)
        .hash(&input.previous_transit_hash)
        .finish()
}

pub fn relay_delivery_output_hash(
    transit_hash: Hash,
    forwarded_packet_hash: Hash,
    receiver_channel_proof_hash: Hash,
) -> Hash {
    HashBuilder::domain(RELAY_DELIVERY_OUTPUT_DOMAIN)
        .hash(&transit_hash)
        .hash(&forwarded_packet_hash)
        .hash(&receiver_channel_proof_hash)
        .finish()
}

pub fn packet_transit_chain_hash(transit_hashes: &[Hash]) -> Hash {
    let mut builder = HashBuilder::domain(PACKET_TRANSIT_CHAIN_DOMAIN).u64(transit_hashes.len() as u64);
    for hash in transit_hashes {
        builder = builder.hash(hash);
    }
    builder.finish()
}
