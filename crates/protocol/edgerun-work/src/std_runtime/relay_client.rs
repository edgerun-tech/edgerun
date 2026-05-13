use std::io;
use std::net::{TcpStream, ToSocketAddrs};

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::ChannelEndpoint;
use crate::codec::blake3_hash;
use crate::identity::node_identity_from_key;
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::signing::{
    empty_signature, sign_network_message_payload, sign_node_available, sign_node_heartbeat,
};
use crate::std_runtime::framing::{read_work_packet, unix_ms, write_work_packet};

const CLIENT_CONNECTION_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:client-connection";
const CLIENT_MESSAGE_ID_DOMAIN: &[u8] = b"edgerun:v1:work:client-message-id";

pub struct WorkClient {
    stream: TcpStream,
    key: Ed25519SigningKey,
    pub identity: NodeIdentity,
    sequence: u64,
    connection_hash: Hash,
}

impl WorkClient {
    pub fn connect<A: ToSocketAddrs>(
        addr: A,
        key: Ed25519SigningKey,
        role: u16,
    ) -> io::Result<(Self, WorkPacket)> {
        Self::connect_with_relay_endpoint(addr, key, role, None)
    }

    pub fn connect_with_relay_endpoint<A: ToSocketAddrs>(
        addr: A,
        key: Ed25519SigningKey,
        role: u16,
        relay_endpoint: Option<ChannelEndpoint>,
    ) -> io::Result<(Self, WorkPacket)> {
        let mut stream = TcpStream::connect(addr)?;
        let identity = node_identity_from_key(&key, role);
        let available = sign_node_available(
            &key,
            NodeAvailable {
                abi_version: WORK_WIRE_ABI_VERSION,
                node: identity.clone(),
                relay_endpoint,
                sequence: 1,
                unix_ms: unix_ms(),
                heartbeat_secs: DEFAULT_HEARTBEAT_SECS,
                log_head: [0u8; 32],
                signature: empty_signature(),
            },
        );
        write_work_packet(&mut stream, &WorkPacket::NodeAvailable(available))?;
        let response = read_work_packet(&mut stream)?;
        let connection_hash = HashBuilder::domain(CLIENT_CONNECTION_HASH_DOMAIN)
            .node_id(&identity.node_id)
            .u64(unix_ms())
            .finish();
        Ok((
            Self {
                stream,
                key,
                identity,
                sequence: 1,
                connection_hash,
            },
            response,
        ))
    }

    pub fn send(&mut self, packet: &WorkPacket) -> io::Result<WorkPacket> {
        write_work_packet(&mut self.stream, packet)?;
        read_work_packet(&mut self.stream)
    }

    pub fn heartbeat(&mut self) -> io::Result<WorkPacket> {
        self.sequence += 1;
        let heartbeat = sign_node_heartbeat(
            &self.key,
            NodeHeartbeat {
                abi_version: WORK_WIRE_ABI_VERSION,
                node: self.identity.clone(),
                sequence: self.sequence,
                unix_ms: unix_ms(),
                connection_hash: self.connection_hash,
                log_head: [0u8; 32],
                signature: empty_signature(),
            },
        );
        self.send(&WorkPacket::NodeHeartbeat(heartbeat))
    }

    pub fn signed_network_message(
        &mut self,
        to: NodeId,
        via_relay: NodeId,
        department: u16,
        work_type: u16,
        payload: Vec<u8>,
    ) -> NetworkMessage {
        self.sequence += 1;
        let payload_hash = blake3_hash(&payload);
        let message_id = HashBuilder::domain(CLIENT_MESSAGE_ID_DOMAIN)
            .node_id(&self.identity.node_id)
            .node_id(&to)
            .u64(self.sequence)
            .hash(&payload_hash)
            .finish();
        sign_network_message_payload(
            &self.key,
            message_id,
            [0u8; 32],
            self.identity.node_id,
            to,
            via_relay,
            department,
            work_type,
            self.sequence,
            payload,
        )
    }
}
