use std::collections::{HashMap, HashSet};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::*;
use crate::protocol::*;
use crate::std_runtime::framing::{ack, read_work_packet, unix_ms, write_work_packet};

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

struct AdmissionState {
    sequence: u64,
    relays: HashMap<NodeId, RelayRecord>,
    nodes: HashMap<NodeId, NodeRecord>,
    assignments: HashMap<NodeId, RelayEndpoint>,
    seen_requests: HashSet<Hash>,
    user_balances: HashMap<PublicKey, u64>,
}

#[derive(Clone)]
struct RelayRecord {
    endpoint: RelayEndpoint,
    last_heartbeat_unix_ms: u64,
    connection_hash: Hash,
}

struct NodeRecord {
    node: NodeIdentity,
    assigned_relay: RelayEndpoint,
    admitted: bool,
    last_heartbeat_unix_ms: u64,
}

impl InMemoryAdmissionController {
    pub fn new(admission_key: Ed25519SigningKey, config: AdmissionConfig) -> Self {
        let admission_identity = node_identity_from_key(&admission_key, NODE_ROLE_ADMISSION);
        Self {
            inner: Arc::new(Mutex::new(AdmissionState {
                sequence: 0,
                relays: HashMap::new(),
                nodes: HashMap::new(),
                assignments: HashMap::new(),
                seen_requests: HashSet::new(),
                user_balances: HashMap::new(),
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
        self.inner.lock().expect("admission state poisoned").user_balances.insert(user, balance);
    }

    pub fn serve<A: ToSocketAddrs>(&self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let stream = stream?;
            let controller = self.clone();
            thread::spawn(move || {
                let _ = controller.handle_connection(stream);
            });
        }
        Ok(())
    }

    pub fn handle_connection(&self, mut stream: TcpStream) -> io::Result<()> {
        let peer = stream.peer_addr().ok();
        let first = read_work_packet(&mut stream)?;
        let connection_hash = connection_hash(peer, unix_ms());
        let mut registered_node = None;

        if let WorkPacket::NodeAvailable(available) = first {
            let node_id = available.node.node_id;
            let response = self.handle_node_available(available, connection_hash, peer)?;
            if matches!(response, WorkPacket::RelayPeerList(_) | WorkPacket::RelayAssignment(_)) {
                registered_node = Some(node_id);
            }
            write_work_packet(&mut stream, &response)?;
        } else {
            write_work_packet(&mut stream, &ack(false, 400, "first packet must be signed NodeAvailable"))?;
            return Ok(());
        }

        loop {
            let packet = match read_work_packet(&mut stream) {
                Ok(packet) => packet,
                Err(_) => break,
            };
            let response = self.handle_packet(packet, registered_node)?;
            write_work_packet(&mut stream, &response)?;
        }

        if let Some(node_id) = registered_node {
            self.drop_connection(node_id, connection_hash);
        }
        Ok(())
    }

    pub fn handle_packet(&self, packet: WorkPacket, registered_node: Option<NodeId>) -> io::Result<WorkPacket> {
        match packet {
            WorkPacket::NodeHeartbeat(heartbeat) => {
                let ok = self.handle_heartbeat(heartbeat, registered_node);
                Ok(ack(ok, if ok { 200 } else { 401 }, if ok { "heartbeat" } else { "invalid heartbeat" }))
            }
            WorkPacket::NetworkMessage(message) => self.handle_network_message(message),
            WorkPacket::WorkRequest(request) => self.handle_work_request(request),
            WorkPacket::WorkReceipt(receipt) => {
                if verify_work_receipt(&receipt) {
                    Ok(ack(true, 202, "receipt accepted"))
                } else {
                    Ok(ack(false, 401, "invalid receipt"))
                }
            }
            _ => Ok(ack(false, 400, "unsupported packet for established connection")),
        }
    }

    fn handle_node_available(&self, available: NodeAvailable, connection_hash: Hash, peer: Option<SocketAddr>) -> io::Result<WorkPacket> {
        if !verify_node_available(&available) {
            return Ok(ack(false, 401, "invalid availability signature"));
        }
        match available.node.role {
            NODE_ROLE_RELAY => self.admit_relay(available, connection_hash, peer),
            _ => self.assign_relay(available),
        }
    }

    fn admit_relay(&self, available: NodeAvailable, connection_hash: Hash, peer: Option<SocketAddr>) -> io::Result<WorkPacket> {
        let host = if available.listen_host.is_empty() {
            peer.map(|addr| addr.ip().to_string()).unwrap_or_default()
        } else {
            available.listen_host.clone()
        };
        let endpoint = RelayEndpoint { relay_node_id: available.node.node_id, host, port: available.listen_port };
        let mut state = self.inner.lock().expect("admission state poisoned");
        state.relays.insert(available.node.node_id, RelayRecord { endpoint: endpoint.clone(), last_heartbeat_unix_ms: available.unix_ms, connection_hash });
        let relays = state.relays.values().map(|relay| relay.endpoint.clone()).collect::<Vec<_>>();
        Ok(WorkPacket::RelayPeerList(RelayPeerList { abi_version: WORK_WIRE_ABI_VERSION, assigned_to: available.node.node_id, relays }))
    }

    fn assign_relay(&self, available: NodeAvailable) -> io::Result<WorkPacket> {
        let mut state = self.inner.lock().expect("admission state poisoned");
        let Some(relay) = state.relays.values().next().cloned() else {
            return Ok(ack(false, 503, "no relay available"));
        };
        state.sequence += 1;
        let assignment = RelayAssignment {
            abi_version: WORK_WIRE_ABI_VERSION,
            node_id: available.node.node_id,
            relay: relay.endpoint.clone(),
            assigned_by: self.admission_identity.clone(),
            sequence: state.sequence,
            valid_until_unix_ms: unix_ms().saturating_add(60_000),
            signature: empty_signature(),
        };
        let assignment = sign_relay_assignment(&self.admission_key, assignment);
        state.assignments.insert(available.node.node_id, relay.endpoint.clone());
        state.nodes.insert(available.node.node_id, NodeRecord { node: available.node, assigned_relay: relay.endpoint, admitted: false, last_heartbeat_unix_ms: unix_ms() });
        Ok(WorkPacket::RelayAssignment(assignment))
    }

    fn handle_heartbeat(&self, heartbeat: NodeHeartbeat, registered_node: Option<NodeId>) -> bool {
        if !verify_node_heartbeat(&heartbeat) {
            return false;
        }
        if registered_node.is_some_and(|node_id| node_id != heartbeat.node.node_id) {
            return false;
        }
        let mut state = self.inner.lock().expect("admission state poisoned");
        if let Some(relay) = state.relays.get_mut(&heartbeat.node.node_id) {
            relay.last_heartbeat_unix_ms = heartbeat.unix_ms;
            relay.connection_hash = heartbeat.connection_hash;
            return true;
        }
        if let Some(node) = state.nodes.get_mut(&heartbeat.node.node_id) {
            node.last_heartbeat_unix_ms = heartbeat.unix_ms;
            return true;
        }
        false
    }
