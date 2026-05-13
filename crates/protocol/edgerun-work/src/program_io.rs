use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;
use rkyv::{Archive, Deserialize, Serialize};

use crate::channel::{ChannelEndpoint, RouteAdvertisement};
use crate::codec::{aligned_copy_if_needed_for, blake3_hash};
use crate::identity::node_identity_from_key;
use crate::protocol::{
    Hash, NodeId, NodeIdentity, WorkPacket, WorkProtocolError, DEPARTMENT_COMPUTE,
    NODE_ROLE_COMPUTE, WORK_TYPE_PROGRAM_CLOSE, WORK_TYPE_PROGRAM_EVENT, WORK_TYPE_PROGRAM_OPEN,
    WORK_TYPE_PROGRAM_POLL, WORK_TYPE_PROGRAM_STDIN,
};
use crate::roles::{
    network_message_for_role, RoleContext, RoleInput, RoleOutput, WorkRole, ROLE_STATUS_ACCEPTED,
};
use crate::route_builder::RouteAdvertisementBuilder;
use crate::signing::{sign_network_message_payload, simple_network_message_id};

pub const PROGRAM_STREAM_STDIN: u16 = 0;
pub const PROGRAM_STREAM_STDOUT: u16 = 1;
pub const PROGRAM_STREAM_STDERR: u16 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramEnv {
    pub key: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramOpen {
    pub session_id: Hash,
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<ProgramEnv>,
    pub cwd: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramStdin {
    pub session_id: Hash,
    pub bytes: Vec<u8>,
    pub eof: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramClose {
    pub session_id: Hash,
    pub signal: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramPoll {
    pub session_id: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramOutput {
    pub session_id: Hash,
    pub stream: u16,
    pub bytes: Vec<u8>,
    pub eof: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramExit {
    pub session_id: Hash,
    pub code: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum ProgramIoPayload {
    Open(ProgramOpen),
    Stdin(ProgramStdin),
    Close(ProgramClose),
    Poll(ProgramPoll),
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum ProgramIoEvent {
    Output(ProgramOutput),
    Exit(ProgramExit),
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct ProgramIoEvents {
    pub events: Vec<ProgramIoEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramIoError {
    InvalidPayload,
    SessionMissing,
    SessionExists,
    ProgramRejected,
    BackendFailed,
}

impl ProgramIoError {
    pub const fn message(&self) -> &'static [u8] {
        match self {
            Self::InvalidPayload => b"invalid program io payload",
            Self::SessionMissing => b"program session missing",
            Self::SessionExists => b"program session already exists",
            Self::ProgramRejected => b"program rejected",
            Self::BackendFailed => b"program backend failed",
        }
    }
}

pub trait ProgramIoAdapter {
    fn open(&mut self, request: ProgramOpen) -> Result<Vec<ProgramIoEvent>, ProgramIoError>;
    fn stdin(&mut self, input: ProgramStdin) -> Result<Vec<ProgramIoEvent>, ProgramIoError>;
    fn close(&mut self, close: ProgramClose) -> Result<Vec<ProgramIoEvent>, ProgramIoError>;
    fn poll(&mut self, poll: ProgramPoll) -> Result<Vec<ProgramIoEvent>, ProgramIoError>;
}

pub struct ProgramIoRole<A> {
    adapter: A,
}

impl<A> ProgramIoRole<A> {
    pub const fn new(adapter: A) -> Self {
        Self { adapter }
    }

    pub fn adapter(&self) -> &A {
        &self.adapter
    }

    pub fn adapter_mut(&mut self) -> &mut A {
        &mut self.adapter
    }
}

impl<A: ProgramIoAdapter> WorkRole for ProgramIoRole<A> {
    fn role_id(&self) -> u16 {
        NODE_ROLE_COMPUTE
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_COMPUTE
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        work_type == WORK_TYPE_PROGRAM_OPEN
            || work_type == WORK_TYPE_PROGRAM_STDIN
            || work_type == WORK_TYPE_PROGRAM_CLOSE
            || work_type == WORK_TYPE_PROGRAM_POLL
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput::ignored();
        };
        let Ok(payload) = program_io_payload_from_bytes(&message.payload) else {
            return RoleOutput::rejected(ProgramIoError::InvalidPayload.message().to_vec());
        };
        let result = match payload {
            ProgramIoPayload::Open(request) if message.work_type == WORK_TYPE_PROGRAM_OPEN => {
                self.adapter.open(request)
            }
            ProgramIoPayload::Stdin(input) if message.work_type == WORK_TYPE_PROGRAM_STDIN => {
                self.adapter.stdin(input)
            }
            ProgramIoPayload::Close(close) if message.work_type == WORK_TYPE_PROGRAM_CLOSE => {
                self.adapter.close(close)
            }
            ProgramIoPayload::Poll(poll) if message.work_type == WORK_TYPE_PROGRAM_POLL => {
                self.adapter.poll(poll)
            }
            _ => Err(ProgramIoError::InvalidPayload),
        };
        let events = match result {
            Ok(events) => events,
            Err(error) => return RoleOutput::rejected(error.message().to_vec()),
        };
        if events.is_empty() {
            return RoleOutput::accepted_ack(200, "program io accepted");
        }
        let Ok(bytes) = program_io_events_bytes(&ProgramIoEvents { events }) else {
            return RoleOutput::rejected(b"program io response encode failed".to_vec());
        };
        RoleOutput {
            status: ROLE_STATUS_ACCEPTED,
            packet: None,
            bytes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramIoResponse {
    pub status: u16,
    pub packet: Option<WorkPacket>,
    pub bytes: Vec<u8>,
}

pub struct ProgramIoService<A> {
    key: Ed25519SigningKey,
    identity: NodeIdentity,
    role: ProgramIoRole<A>,
    policy_hash: Hash,
    event_sequence: u64,
}

impl<A: ProgramIoAdapter> ProgramIoService<A> {
    pub fn new(key: Ed25519SigningKey, adapter: A) -> Self {
        Self::with_policy(key, adapter, [0u8; 32])
    }

    pub fn with_policy(key: Ed25519SigningKey, adapter: A, policy_hash: Hash) -> Self {
        let identity = node_identity_from_key(&key, NODE_ROLE_COMPUTE);
        Self {
            key,
            identity,
            role: ProgramIoRole::new(adapter),
            policy_hash,
            event_sequence: 0,
        }
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    pub fn node_id(&self) -> NodeId {
        self.identity.node_id
    }

    pub fn adapter(&self) -> &A {
        self.role.adapter()
    }

    pub fn adapter_mut(&mut self) -> &mut A {
        self.role.adapter_mut()
    }

    pub fn route_advertisement(
        &self,
        endpoint: ChannelEndpoint,
        relay_node_id: NodeId,
        valid_until_unix_ms: u64,
    ) -> RouteAdvertisement {
        RouteAdvertisementBuilder::new(&self.key, NODE_ROLE_COMPUTE, endpoint)
            .relay_node_id(relay_node_id)
            .departments(vec![DEPARTMENT_COMPUTE])
            .valid_until_unix_ms(valid_until_unix_ms)
            .build(&self.key)
    }

    pub fn handle_packet(&mut self, packet: WorkPacket, now_unix_ms: u64) -> ProgramIoResponse {
        let request = match &packet {
            WorkPacket::NetworkMessage(message) => Some(message.clone()),
            _ => None,
        };
        let output = self.role.handle(
            &RoleContext {
                now_unix_ms,
                local_node: self.identity.clone(),
                policy_hash: self.policy_hash,
            },
            RoleInput {
                packet,
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        let packet = if output.status == ROLE_STATUS_ACCEPTED && !output.bytes.is_empty() {
            request.map(|message| {
                self.event_sequence = self.event_sequence.wrapping_add(1);
                self.signed_event_packet(&message, output.bytes.clone(), self.event_sequence)
            })
        } else {
            output.packet
        };
        ProgramIoResponse {
            status: output.status,
            packet,
            bytes: output.bytes,
        }
    }

    fn signed_event_packet(
        &self,
        request: &crate::protocol::NetworkMessage,
        payload: Vec<u8>,
        sequence: u64,
    ) -> WorkPacket {
        let payload_hash = blake3_hash(&payload);
        let message_id = simple_network_message_id(
            &self.identity.node_id,
            &request.from,
            sequence,
            &payload_hash,
        );
        WorkPacket::NetworkMessage(sign_network_message_payload(
            &self.key,
            message_id,
            request.message_id,
            self.identity.node_id,
            request.from,
            request.via_relay,
            DEPARTMENT_COMPUTE,
            WORK_TYPE_PROGRAM_EVENT,
            sequence,
            payload,
        ))
    }
}

pub fn program_io_payload_bytes(payload: &ProgramIoPayload) -> Result<Vec<u8>, WorkProtocolError> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(payload)
        .map(|bytes| bytes.to_vec())
        .map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn program_io_payload_from_bytes(bytes: &[u8]) -> Result<ProgramIoPayload, WorkProtocolError> {
    if let Some(aligned) = aligned_copy_if_needed_for::<ArchivedProgramIoPayload>(bytes) {
        return program_io_payload_from_aligned_bytes(aligned.as_slice());
    }
    program_io_payload_from_aligned_bytes(bytes)
}

fn program_io_payload_from_aligned_bytes(
    bytes: &[u8],
) -> Result<ProgramIoPayload, WorkProtocolError> {
    use edgerun_wire::{access, deserialize, WireError};
    let archived = access::<ArchivedProgramIoPayload, WireError>(bytes)
        .map_err(|_| WorkProtocolError::InvalidPacket)?;
    deserialize::<ProgramIoPayload, WireError>(archived)
        .map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn program_io_events_bytes(events: &ProgramIoEvents) -> Result<Vec<u8>, WorkProtocolError> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(events)
        .map(|bytes| bytes.to_vec())
        .map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn program_io_events_from_bytes(bytes: &[u8]) -> Result<ProgramIoEvents, WorkProtocolError> {
    if let Some(aligned) = aligned_copy_if_needed_for::<ArchivedProgramIoEvents>(bytes) {
        return program_io_events_from_aligned_bytes(aligned.as_slice());
    }
    program_io_events_from_aligned_bytes(bytes)
}

fn program_io_events_from_aligned_bytes(
    bytes: &[u8],
) -> Result<ProgramIoEvents, WorkProtocolError> {
    use edgerun_wire::{access, deserialize, WireError};
    let archived = access::<ArchivedProgramIoEvents, WireError>(bytes)
        .map_err(|_| WorkProtocolError::InvalidPacket)?;
    deserialize::<ProgramIoEvents, WireError>(archived)
        .map_err(|_| WorkProtocolError::InvalidPacket)
}
