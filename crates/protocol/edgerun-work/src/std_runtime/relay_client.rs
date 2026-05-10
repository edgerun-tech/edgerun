use std::io;
use std::net::{TcpStream, ToSocketAddrs};

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::*;
use crate::protocol::*;
use crate::std_runtime::framing::{read_work_packet, unix_ms, write_work_packet};

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
        listen_host: impl Into<String>,
        listen_port: u16,
    ) -> io::Result<(Self, WorkPacket)> {
        let mut stream = TcpStream::connect(addr)?;
        let identity = node_identity_from_key(&key, role);
        let available = sign_node_available(
            &key,
            NodeAvailable {
                abi_version: WORK_WIRE_ABI_VERSION,
                node: identity.clone(),
                sequence: 1,
                unix_ms: unix_ms(),
                listen_host: listen_host.into(),
                listen_port,
                heartbeat_secs: DEFAULT_HEARTBEAT_SECS,
                log_head: [0u8; 32],
                signature: empty_signature(),
            },
        );
        write_work_packet(&mut stream, &WorkPacket::NodeAvailable(available))?;
        let response = read_work_packet(&mut stream)?;
        let mut seed = Vec::new();
        seed.extend_from_slice(&identity.node_id);
        seed.extend_from_slice(&unix_ms().to_be_bytes());
        let connection_hash = blake3_hash(&seed);
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
        let mut id_input = Vec::new();
        id_input.extend_from_slice(&self.identity.node_id);
        id_input.extend_from_slice(&to);
        id_input.extend_from_slice(&self.sequence.to_be_bytes());
        id_input.extend_from_slice(&payload_hash);
        sign_network_message(
            &self.key,
            NetworkMessage {
                abi_version: WORK_WIRE_ABI_VERSION,
                message_id: blake3_hash(&id_input),
                prev_hash: [0u8; 32],
                from: self.identity.node_id,
                to,
                via_relay,
                department,
                work_type,
                sequence: self.sequence,
                payload_hash,
                payload,
                signature: empty_signature(),
            },
        )
    }
}
