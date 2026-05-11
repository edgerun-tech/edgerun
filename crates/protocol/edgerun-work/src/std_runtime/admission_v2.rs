use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::{ChannelEndpoint, CHANNEL_KIND_TCP};
use crate::codec::packet_hash;
use crate::identity::node_identity_from_key;
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::request_auth::verify_work_request;
use crate::signing::{
    empty_signature, sign_relay_assignment, sign_work_admission, verify_network_message,
    verify_node_available, verify_node_heartbeat, verify_work_receipt,
};
use crate::std_runtime::framing::{ack, read_work_packet, unix_ms, write_work_packet};

const ADMISSION_ID_DOMAIN: &[u8] = b"edgerun:v1:work:admission-id";
const RELAY_CHANNEL_ID_DOMAIN: &[u8] = b"edgerun:v1:work:relay-channel-id";
const RELAY_ROUTE_COMMITMENT_DOMAIN: &[u8] = b"edgerun:v1:work:relay-route-commitment";
const ADMISSION_CONNECTION_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:admission-connection";

#[derive(Clone)]
pub struct InMemoryAdmissionController {
    inner: Arc<Mutex<AdmissionState>>,
    admission_key: Ed25519SigningKey,
    admission_identity: NodeIdentity,
    dao_id: PublicKey,
    policy_hash: Hash,
    heartbeat_grace: Duration,
}

#[derive(Clone)]
pub struct AdmissionConfig {
    pub dao_id: PublicKey,
    pub policy_hash: Hash,
    pub heartbeat_grace_secs: u64,
}

#[derive(Clone, Debug, Default)]
pub struct AdmissionReplayState {
    seen_request_ids: HashMap<Hash, u64>,
    highest_user_sequence: HashMap<PublicKey, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdmissionReplayError {
    DuplicateRequestId,
    NonIncreasingUserSequence { last_seen: u64, received: u64 },
}

impl AdmissionReplayState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate(&self, request: &WorkRequest) -> Result<(), AdmissionReplayError> {
        if self.seen_request_ids.contains_key(&request.request_id) {
            return Err(AdmissionReplayError::DuplicateRequestId);
        }
        if let Some(last_seen) = self.highest_user_sequence.get(&request.user) {
            if request.user_sequence <= *last_seen {
                return Err(AdmissionReplayError::NonIncreasingUserSequence {
                    last_seen: *last_seen,
                    received: request.user_sequence,
                });
            }
        }
        Ok(())
    }

    pub fn record_checked(&mut self, request: &WorkRequest) -> Result<(), AdmissionReplayError> {
        self.validate(request)?;
        self.seen_request_ids
            .insert(request.request_id, request.valid_until_unix_ms);
        self.highest_user_sequence
            .insert(request.user, request.user_sequence);
        Ok(())
    }

    pub fn prune_expired(&mut self, now_unix_ms: u64) -> usize {
        let before = self.seen_request_ids.len();
        self.seen_request_ids
            .retain(|_, valid_until_unix_ms| *valid_until_unix_ms >= now_unix_ms);
        before.saturating_sub(self.seen_request_ids.len())
    }

    pub fn seen_request_count(&self) -> usize {
        self.seen_request_ids.len()
    }

    pub fn highest_sequence_for(&self, user: &PublicKey) -> Option<u64> {
        self.highest_user_sequence.get(user).copied()
    }
}

struct AdmissionState {
    seq: u64,
    relays: HashMap<NodeId, RelayRecord>,
    nodes: HashMap<NodeId, NodeRecord>,
    replay: AdmissionReplayState,
    balances: HashMap<PublicKey, u64>,
}

#[derive(Clone)]
struct RelayRecord {
    endpoint: RelayEndpoint,
    last_ms: u64,
    connection_hash: Hash,
}

struct NodeRecord {
    node: NodeIdentity,
    relay: RelayEndpoint,
    admitted: bool,
    last_ms: u64,
}

impl InMemoryAdmissionController {
    pub fn new(admission_key: Ed25519SigningKey, config: AdmissionConfig) -> Self {
        let admission_identity = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
        Self {
            inner: Arc::new(Mutex::new(AdmissionState {
                seq: 0,
                relays: HashMap::new(),
                nodes: HashMap::new(),
                replay: AdmissionReplayState::new(),
                balances: HashMap::new(),
            })),
            admission_key,
            admission_identity,
            dao_id: config.dao_id,
            policy_hash: config.policy_hash,
            heartbeat_grace: Duration::from_secs(config.heartbeat_grace_secs.max(1)),
        }
    }

    pub fn identity(&self) -> NodeIdentity {
        self.admission_identity.clone()
    }

    pub fn set_user_balance(&self, user: PublicKey, balance: u64) {
        self.inner
            .lock()
            .expect("admission state poisoned")
            .balances
            .insert(user, balance);
    }

    pub fn serve<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let controller = self.clone();
            thread::spawn(move || {
                if let Ok(stream) = stream {
                    let _ = controller.handle_connection(stream);
                }
            });
        }
        Ok(())
    }

    pub fn handle_connection(&self, mut stream: TcpStream) -> io::Result<()> {
        let peer = stream.peer_addr().ok();
        let connection_hash = connection_hash(peer, unix_ms());
        let first = read_work_packet(&mut stream)?;
        let Some(registered) = self.accept_first(&mut stream, first, connection_hash, peer)? else {
            return Ok(());
        };
        loop {
            let packet = match read_work_packet(&mut stream) {
                Ok(packet) => packet,
                Err(_) => break,
            };
            let reply = self.handle_packet(packet, Some(registered))?;
            write_work_packet(&mut stream, &reply)?;
        }
        self.drop_connection(registered, connection_hash);
        Ok(())
    }

    fn accept_first(
        &self,
        stream: &mut TcpStream,
        packet: WorkPacket,
        connection_hash: Hash,
        peer: Option<SocketAddr>,
    ) -> io::Result<Option<NodeId>> {
        let WorkPacket::NodeAvailable(available) = packet else {
            write_work_packet(stream, &ack(false, 400, "first packet must be NodeAvailable"))?;
            return Ok(None);
        };
        let node_id = available.node.node_id;
        let reply = self.handle_node_available(available, connection_hash, peer)?;
        let accepted = matches!(reply, WorkPacket::RelayPeerList(_) | WorkPacket::RelayAssignment(_));
        write_work_packet(stream, &reply)?;
        Ok(accepted.then_some(node_id))
    }

    pub fn handle_packet(&self, packet: WorkPacket, on_connection: Option<NodeId>) -> io::Result<WorkPacket> {
        match packet {
            WorkPacket::NodeHeartbeat(h) => {
                let ok = self.handle_heartbeat(h, on_connection);
                Ok(ack(
                    ok,
                    if ok { 200 } else { 401 },
                    if ok { "heartbeat" } else { "invalid heartbeat" },
                ))
            }
            WorkPacket::NetworkMessage(m) => self.handle_network_message(m, on_connection),
            WorkPacket::WorkRequest(r) => self.handle_work_request(r),
            WorkPacket::WorkReceipt(r) => Ok(if verify_work_receipt(&r) {
                ack(true, 202, "receipt accepted")
            } else {
                ack(false, 401, "invalid receipt")
            }),
            _ => Ok(ack(false, 400, "unsupported packet")),
        }
    }

    fn handle_node_available(
        &self,
        available: NodeAvailable,
        connection_hash: Hash,
        peer: Option<SocketAddr>,
    ) -> io::Result<WorkPacket> {
        if !verify_node_available(&available) {
            return Ok(ack(false, 401, "invalid availability signature"));
        }
        if available.node.role == NODE_ROLE_RELAY {
            self.admit_relay(available, connection_hash, peer)
        } else {
            self.assign_relay(available)
        }
    }

    fn admit_relay(
        &self,
        available: NodeAvailable,
        connection_hash: Hash,
        peer: Option<SocketAddr>,
    ) -> io::Result<WorkPacket> {
        let host = if available.listen_host.is_empty() {
            peer.map(|p| p.ip().to_string()).unwrap_or_default()
        } else {
            available.listen_host.clone()
        };
        let endpoint = RelayEndpoint {
            relay_node_id: available.node.node_id,
            host,
            port: available.listen_port,
        };
        let mut state = self.inner.lock().expect("admission state poisoned");
        state.relays.insert(
            available.node.node_id,
            RelayRecord {
                endpoint: endpoint.clone(),
                last_ms: available.unix_ms,
                connection_hash,
            },
        );
        let relays = state.relays.values().map(|r| r.endpoint.clone()).collect();
        Ok(WorkPacket::RelayPeerList(RelayPeerList {
            abi_version: WORK_WIRE_ABI_VERSION,
            assigned_to: available.node.node_id,
            relays,
        }))
    }

    fn assign_relay(&self, available: NodeAvailable) -> io::Result<WorkPacket> {
        let mut state = self.inner.lock().expect("admission state poisoned");
        let Some(relay) = state.relays.values().next().cloned() else {
            return Ok(ack(false, 503, "no relay available"));
        };
        state.seq += 1;
        let assignment = sign_relay_assignment(
            &self.admission_key,
            RelayAssignment {
                abi_version: WORK_WIRE_ABI_VERSION,
                node_id: available.node.node_id,
                relay: relay.endpoint.clone(),
                assigned_by: self.admission_identity.clone(),
                sequence: state.seq,
                valid_until_unix_ms: unix_ms().saturating_add(60_000),
                signature: empty_signature(),
            },
        );
        state.nodes.insert(
            available.node.node_id,
            NodeRecord {
                node: available.node,
                relay: relay.endpoint,
                admitted: false,
                last_ms: unix_ms(),
            },
        );
        Ok(WorkPacket::RelayAssignment(assignment))
    }

    fn handle_heartbeat(&self, heartbeat: NodeHeartbeat, on_connection: Option<NodeId>) -> bool {
        if !verify_node_heartbeat(&heartbeat) || on_connection.is_some_and(|id| id != heartbeat.node.node_id) {
            return false;
        }
        let mut state = self.inner.lock().expect("admission state poisoned");
        if let Some(relay) = state.relays.get_mut(&heartbeat.node.node_id) {
            relay.last_ms = heartbeat.unix_ms;
            relay.connection_hash = heartbeat.connection_hash;
            return true;
        }
        if let Some(node) = state.nodes.get_mut(&heartbeat.node.node_id) {
            node.last_ms = heartbeat.unix_ms;
            return true;
        }
        false
    }

    fn handle_network_message(
        &self,
        message: NetworkMessage,
        on_connection: Option<NodeId>,
    ) -> io::Result<WorkPacket> {
        if on_connection != Some(message.via_relay) {
            return Ok(ack(false, 403, "message not received on claimed relay connection"));
        }
        let mut state = self.inner.lock().expect("admission state poisoned");
        let Some(node) = state.nodes.get_mut(&message.from) else {
            return Ok(ack(false, 404, "unknown node"));
        };
        if message.via_relay != node.relay.relay_node_id {
            return Ok(ack(false, 403, "wrong relay"));
        }
        if message.to != self.admission_identity.node_id {
            return Ok(ack(false, 400, "wrong recipient"));
        }
        if !verify_network_message(&message, &node.node) {
            return Ok(ack(false, 401, "invalid message signature"));
        }
        node.admitted = true;
        Ok(ack(true, 200, "node admitted"))
    }

    fn handle_work_request(&self, request: WorkRequest) -> io::Result<WorkPacket> {
        if !verify_work_request(&request) {
            return Ok(ack(false, 401, "invalid user work request signature"));
        }
        let now = unix_ms();
        if request.valid_until_unix_ms < now {
            return Ok(ack(false, 408, "expired request"));
        }
        let request_hash = match packet_hash(&WorkPacket::WorkRequest(request.clone())) {
            Ok(hash) => hash,
            Err(_) => return Ok(ack(false, 400, "invalid request packet")),
        };
        let mut state = self.inner.lock().expect("admission state poisoned");
        state.replay.prune_expired(now);
        if let Err(error) = state.replay.validate(&request) {
            return Ok(admission_replay_ack(error));
        }
        if *state.balances.get(&request.user).unwrap_or(&0) < request.max_total_cost {
            return Ok(ack(false, 402, "insufficient work credit"));
        }
        let Some(relay) = state.relays.values().next().cloned() else {
            return Ok(ack(false, 503, "no relay available"));
        };
        state
            .replay
            .record_checked(&request)
            .expect("validated request replay state changed while locked");
        state.seq += 1;
        let sequence = state.seq;
        let (assigned_route_hash, assigned_channel) = relay_channel_commitment(&relay.endpoint);
        let admission = WorkAdmission {
            abi_version: WORK_WIRE_ABI_VERSION,
            admission_id: admission_id_for_request(self.admission_identity.node_id, request_hash, sequence),
            dao_id: self.dao_id,
            user: request.user,
            admission_node: self.admission_identity.clone(),
            request_hash,
            assigned_route_hash,
            assigned_channel,
            admitted_budget: request.max_total_cost,
            policy_hash: self.policy_hash,
            sequence,
            valid_until_unix_ms: request.valid_until_unix_ms,
            signature: empty_signature(),
        };
        Ok(WorkPacket::WorkAdmission(sign_work_admission(&self.admission_key, admission)))
    }

    fn drop_connection(&self, node_id: NodeId, connection_hash: Hash) {
        let mut state = self.inner.lock().expect("admission state poisoned");
        if state
            .relays
            .get(&node_id)
            .is_some_and(|r| r.connection_hash == connection_hash)
        {
            state.relays.remove(&node_id);
        } else {
            state.nodes.remove(&node_id);
        }
    }

    pub fn prune_dead_relays(&self) -> Vec<NodeId> {
        let now = unix_ms();
        let max_gap = self.heartbeat_grace.as_millis() as u64;
        let mut state = self.inner.lock().expect("admission state poisoned");
        let dead = state
            .relays
            .iter()
            .filter_map(|(id, r)| (now.saturating_sub(r.last_ms) > max_gap).then_some(*id))
            .collect::<Vec<_>>();
        for id in &dead {
            state.relays.remove(id);
        }
        dead
    }

    pub fn prune_dead_nodes(&self) -> Vec<NodeId> {
        let now = unix_ms();
        let max_gap = self.heartbeat_grace.as_millis() as u64;
        let mut state = self.inner.lock().expect("admission state poisoned");
        let dead = state
            .nodes
            .iter()
            .filter_map(|(id, node)| (now.saturating_sub(node.last_ms) > max_gap).then_some(*id))
            .collect::<Vec<_>>();
        for id in &dead {
            state.nodes.remove(id);
        }
        dead
    }
}

fn admission_replay_ack(error: AdmissionReplayError) -> WorkPacket {
    match error {
        AdmissionReplayError::DuplicateRequestId => ack(false, 409, "duplicate request id"),
        AdmissionReplayError::NonIncreasingUserSequence { .. } => ack(false, 409, "stale user request sequence"),
    }
}

fn admission_id_for_request(admission_node_id: NodeId, request_hash: Hash, sequence: u64) -> Hash {
    HashBuilder::domain(ADMISSION_ID_DOMAIN)
        .node_id(&admission_node_id)
        .u64(sequence)
        .hash(&request_hash)
        .finish()
}

fn relay_channel_commitment(relay: &RelayEndpoint) -> (Hash, ChannelEndpoint) {
    let mut address = Vec::new();
    address.extend_from_slice(relay.host.as_bytes());
    address.push(b':');
    address.extend_from_slice(relay.port.to_string().as_bytes());
    let channel_id = HashBuilder::domain(RELAY_CHANNEL_ID_DOMAIN)
        .bytes(&address)
        .finish();
    let channel = ChannelEndpoint::new(channel_id, CHANNEL_KIND_TCP, address.clone(), relay.host.clone());
    let route = HashBuilder::domain(RELAY_ROUTE_COMMITMENT_DOMAIN)
        .node_id(&relay.relay_node_id)
        .hash(&channel_id)
        .bytes(&address)
        .finish();
    (route, channel)
}

fn connection_hash(peer: Option<SocketAddr>, now: u64) -> Hash {
    let peer = peer.map(|peer| peer.to_string()).unwrap_or_default();
    HashBuilder::domain(ADMISSION_CONNECTION_HASH_DOMAIN)
        .bytes(peer.as_bytes())
        .u64(now)
        .finish()
}
