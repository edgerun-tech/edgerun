use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::*;
use crate::codec::blake3_hash;
use crate::identity::node_identity_from_key;
use crate::protocol::*;
use crate::route_auth::sign_route_advertisement;
use crate::signing::{empty_signature, verify_relay_assignment};

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

pub fn storage_route_from_relay_assignment(
    storage_key: &Ed25519SigningKey,
    assignment: &RelayAssignment,
    endpoint: ChannelEndpoint,
) -> Result<RouteAdvertisement, WorkProtocolError> {
    if !verify_relay_assignment(assignment) {
        return Err(WorkProtocolError::InvalidSignature);
    }
    let storage = node_identity_from_key(storage_key, NODE_ROLE_STORAGE);
    if assignment.node_id != storage.node_id {
        return Err(WorkProtocolError::WrongRelay);
    }
    Ok(RouteAdvertisementBuilder::new(storage_key, NODE_ROLE_STORAGE, endpoint)
        .relay_node_id(assignment.relay.relay_node_id)
        .departments(vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL])
        .valid_until_unix_ms(assignment.valid_until_unix_ms)
        .build(storage_key))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::{empty_signature, sign_relay_assignment};

    #[test]
    fn storage_route_is_bound_to_assigned_relay() {
        let admission_key = Ed25519SigningKey::from_bytes(&[1u8; 32]);
        let relay_key = Ed25519SigningKey::from_bytes(&[2u8; 32]);
        let storage_key = Ed25519SigningKey::from_bytes(&[3u8; 32]);
        let relay = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
        let storage = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);
        let admission = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
        let assignment = sign_relay_assignment(
            &admission_key,
            RelayAssignment {
                abi_version: WORK_WIRE_ABI_VERSION,
                node_id: storage.node_id,
                relay: RelayEndpoint {
                    relay_node_id: relay.node_id,
                    host: "127.0.0.1".into(),
                    port: 9000,
                },
                assigned_by: admission,
                sequence: 1,
                valid_until_unix_ms: u64::MAX,
                signature: empty_signature(),
            },
        );
        let route = storage_route_from_relay_assignment(
            &storage_key,
            &assignment,
            tcp_endpoint("storage", "127.0.0.1:9001"),
        )
        .expect("storage route");
        assert_eq!(route.node.node_id, storage.node_id);
        assert_eq!(route.relay_node_id, relay.node_id);
        assert_eq!(route.departments, vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL]);
    }
}
