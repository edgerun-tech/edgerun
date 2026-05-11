use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::*;
use crate::codec::{blake3_hash, empty_signature, node_identity_from_key};
use crate::protocol::*;
use crate::route_auth::sign_route_advertisement;

const CHANNEL_ENDPOINT_ID_DOMAIN: &[u8] = b"edgerun:v1:work:channel-endpoint";

pub struct RouteAdvertisementBuilder {
    node: NodeIdentity,
    relay_node_id: NodeId,
    endpoint: ChannelEndpoint,
    roles: Vec<u16>,
    departments: Vec<u16>,
    status: u16,
    sequence: u64,
    valid_until_unix_ms: u64,
    previous_route_hash: Hash,
}

impl RouteAdvertisementBuilder {
    pub fn new(key: &Ed25519SigningKey, role: u16, endpoint: ChannelEndpoint) -> Self {
        let node = node_identity_from_key(key, role);
        Self {
            relay_node_id: node.node_id,
            node,
            endpoint,
            roles: vec![role],
            departments: Vec::new(),
            status: ROUTE_STATUS_AVAILABLE,
            sequence: 1,
            valid_until_unix_ms: u64::MAX,
            previous_route_hash: [0u8; 32],
        }
    }

    pub fn relay_node_id(mut self, relay_node_id: NodeId) -> Self {
        self.relay_node_id = relay_node_id;
        self
    }

    pub fn departments(mut self, departments: Vec<u16>) -> Self {
        self.departments = departments;
        self
    }

    pub fn roles(mut self, roles: Vec<u16>) -> Self {
        self.roles = roles;
        self
    }

    pub fn sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    pub fn valid_until_unix_ms(mut self, valid_until_unix_ms: u64) -> Self {
        self.valid_until_unix_ms = valid_until_unix_ms;
        self
    }

    pub fn previous_route_hash(mut self, previous_route_hash: Hash) -> Self {
        self.previous_route_hash = previous_route_hash;
        self
    }

    pub fn build(self, key: &Ed25519SigningKey) -> RouteAdvertisement {
        sign_route_advertisement(
            key,
            RouteAdvertisement {
                abi_version: WORK_WIRE_ABI_VERSION,
                node: self.node,
                relay_node_id: self.relay_node_id,
                endpoint: self.endpoint,
                roles: self.roles,
                departments: self.departments,
                status: self.status,
                sequence: self.sequence,
                valid_until_unix_ms: self.valid_until_unix_ms,
                previous_route_hash: self.previous_route_hash,
                signature: empty_signature(),
            },
        )
    }
}

pub fn memory_endpoint(label: impl Into<String>, seed: &[u8]) -> ChannelEndpoint {
    ChannelEndpoint::new(endpoint_channel_id(CHANNEL_KIND_MEMORY, seed), CHANNEL_KIND_MEMORY, Vec::new(), label.into())
}

pub fn tcp_endpoint(label: impl Into<String>, address: impl AsRef<[u8]>) -> ChannelEndpoint {
    endpoint_from_address(CHANNEL_KIND_TCP, label, address)
}

pub fn quic_endpoint(label: impl Into<String>, address: impl AsRef<[u8]>) -> ChannelEndpoint {
    endpoint_from_address(CHANNEL_KIND_QUIC, label, address)
}

pub fn webtransport_endpoint(label: impl Into<String>, url: impl AsRef<[u8]>) -> ChannelEndpoint {
    endpoint_from_address(CHANNEL_KIND_WEBTRANSPORT, label, url)
}

pub fn websocket_endpoint(label: impl Into<String>, url: impl AsRef<[u8]>) -> ChannelEndpoint {
    endpoint_from_address(CHANNEL_KIND_WEBSOCKET, label, url)
}

pub fn endpoint_channel_id(kind: u16, address_or_seed: &[u8]) -> ChannelId {
    let mut input = Vec::new();
    input.extend_from_slice(CHANNEL_ENDPOINT_ID_DOMAIN);
    input.push(0);
    input.extend_from_slice(&kind.to_be_bytes());
    input.extend_from_slice(&(address_or_seed.len() as u64).to_be_bytes());
    input.extend_from_slice(address_or_seed);
    blake3_hash(&input)
}

fn endpoint_from_address(
    kind: u16,
    label: impl Into<String>,
    address: impl AsRef<[u8]>,
) -> ChannelEndpoint {
    let address = address.as_ref().to_vec();
    ChannelEndpoint::new(endpoint_channel_id(kind, &address), kind, address, label.into())
}
