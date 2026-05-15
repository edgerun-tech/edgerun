use alloc::vec::Vec;

use crate::codec::{blake3_hash, wire_bytes, wire_from_bytes};
use crate::generated_wire::{ArchivedContactBook, ArchivedMessageObject, ArchivedThreadObject};
use crate::preimage::HashBuilder;
use crate::protocol::{Hash, NodeId, WORK_WIRE_ABI_VERSION};

const CONTACT_OBJECT_DOMAIN: &[u8] = b"edgerun:v1:work:chat:contact";
const MESSAGE_OBJECT_DOMAIN: &[u8] = b"edgerun:v1:work:chat:message";
const THREAD_ID_DOMAIN: &[u8] = b"edgerun:v1:work:chat:thread-id";
const THREAD_OBJECT_DOMAIN: &[u8] = b"edgerun:v1:work:chat:thread";
const CONTACT_BOOK_DOMAIN: &[u8] = b"edgerun:v1:work:chat:contact-book";

pub const CHAT_PUBLIC_ID_KIND_EDGERUN_NODE: u16 = 1;
pub const CHAT_PUBLIC_ID_KIND_ED25519: u16 = 2;
pub const CHAT_PUBLIC_ID_KIND_X25519: u16 = 3;

pub const CHAT_EXTERNAL_CONTACT_KIND_EMAIL: u16 = 1;
pub const CHAT_EXTERNAL_CONTACT_KIND_PHONE: u16 = 2;
pub const CHAT_EXTERNAL_CONTACT_KIND_GITHUB: u16 = 3;
pub const CHAT_EXTERNAL_CONTACT_KIND_WEB: u16 = 4;

pub const CHAT_MESSAGE_KIND_TEXT: u16 = 1;
pub const CHAT_MESSAGE_KIND_IMAGE: u16 = 2;
pub const CHAT_MESSAGE_KIND_FILE: u16 = 3;
pub const CHAT_MESSAGE_KIND_SYSTEM: u16 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatIndexError {
    InvalidObject,
    WrongThread,
    WrongParticipant,
    WrongSequence,
    WrongPreviousMessage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactPublicId {
    pub kind: u16,
    pub value: Vec<u8>,
    pub label: Vec<u8>,
    pub verified_at_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalContactRef {
    pub kind: u16,
    pub value: Vec<u8>,
    pub label: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactObject {
    pub abi_version: u16,
    pub contact_id: Hash,
    pub primary_node_id: NodeId,
    pub display_name: Vec<u8>,
    pub avatar_payload_hash: Hash,
    pub public_ids: Vec<ContactPublicId>,
    pub external_contacts: Vec<ExternalContactRef>,
    pub updated_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageObject {
    pub abi_version: u16,
    pub message_id: Hash,
    pub thread_id: Hash,
    pub from: NodeId,
    pub to: NodeId,
    pub sequence: u64,
    pub created_unix_ms: u64,
    pub message_kind: u16,
    pub payload_hash: Hash,
    pub payload_len: u64,
    pub sealed_payload_hash: Hash,
    pub storage_ref: Vec<u8>,
    pub previous_message_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadMessageRef {
    pub message_id: Hash,
    pub payload_hash: Hash,
    pub sequence: u64,
    pub created_unix_ms: u64,
    pub from: NodeId,
    pub message_kind: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadObject {
    pub abi_version: u16,
    pub thread_id: Hash,
    pub participant_a: NodeId,
    pub participant_b: NodeId,
    pub created_unix_ms: u64,
    pub updated_unix_ms: u64,
    pub message_count: u64,
    pub head_message_hash: Hash,
    pub tail_message_hash: Hash,
    pub entries: Vec<ThreadMessageRef>,
    pub thread_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactBookContactRef {
    pub contact_id: Hash,
    pub primary_node_id: NodeId,
    pub display_name: Vec<u8>,
    pub avatar_payload_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactBookThreadRef {
    pub thread_id: Hash,
    pub participant_a: NodeId,
    pub participant_b: NodeId,
    pub message_count: u64,
    pub head_message_hash: Hash,
    pub updated_unix_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactBook {
    pub abi_version: u16,
    pub owner: NodeId,
    pub contacts: Vec<ContactBookContactRef>,
    pub threads: Vec<ContactBookThreadRef>,
    pub updated_unix_ms: u64,
    pub contact_book_hash: Hash,
}

pub fn thread_id_for_participants(a: NodeId, b: NodeId) -> Hash {
    let (first, second) = ordered_participants(a, b);
    HashBuilder::domain(THREAD_ID_DOMAIN)
        .node_id(&first)
        .node_id(&second)
        .finish()
}

pub fn contact_object_hash(value: &ContactObject) -> Hash {
    let mut builder = HashBuilder::domain(CONTACT_OBJECT_DOMAIN)
        .node_id(&value.primary_node_id)
        .bytes(&value.display_name)
        .hash(&value.avatar_payload_hash)
        .u64(value.updated_unix_ms);
    for public_id in &value.public_ids {
        builder = builder
            .u16(public_id.kind)
            .bytes(&public_id.value)
            .bytes(&public_id.label)
            .u64(public_id.verified_at_unix_ms);
    }
    for external in &value.external_contacts {
        builder = builder
            .u16(external.kind)
            .bytes(&external.value)
            .bytes(&external.label);
    }
    builder.finish()
}

pub fn finalize_contact_object(mut value: ContactObject) -> ContactObject {
    value.abi_version = WORK_WIRE_ABI_VERSION;
    value.contact_id = contact_object_hash(&value);
    value
}

pub fn message_object_hash(value: &MessageObject) -> Hash {
    HashBuilder::domain(MESSAGE_OBJECT_DOMAIN)
        .hash(&value.thread_id)
        .node_id(&value.from)
        .node_id(&value.to)
        .u64(value.sequence)
        .u64(value.created_unix_ms)
        .u16(value.message_kind)
        .hash(&value.payload_hash)
        .u64(value.payload_len)
        .hash(&value.sealed_payload_hash)
        .bytes(&value.storage_ref)
        .hash(&value.previous_message_hash)
        .finish()
}

pub fn finalize_message_object(mut value: MessageObject) -> MessageObject {
    value.abi_version = WORK_WIRE_ABI_VERSION;
    value.message_id = message_object_hash(&value);
    value
}

pub fn empty_thread(
    participant_a: NodeId,
    participant_b: NodeId,
    now_unix_ms: u64,
) -> ThreadObject {
    let (participant_a, participant_b) = ordered_participants(participant_a, participant_b);
    let mut thread = ThreadObject {
        abi_version: WORK_WIRE_ABI_VERSION,
        thread_id: thread_id_for_participants(participant_a, participant_b),
        participant_a,
        participant_b,
        created_unix_ms: now_unix_ms,
        updated_unix_ms: now_unix_ms,
        message_count: 0,
        head_message_hash: [0u8; 32],
        tail_message_hash: [0u8; 32],
        entries: Vec::new(),
        thread_hash: [0u8; 32],
    };
    thread.thread_hash = thread_object_hash(&thread);
    thread
}

pub fn append_thread_message(
    thread: &mut ThreadObject,
    message: &MessageObject,
) -> Result<(), ChatIndexError> {
    if message.thread_id != thread.thread_id {
        return Err(ChatIndexError::WrongThread);
    }
    if !thread_has_participants(thread, message.from, message.to) {
        return Err(ChatIndexError::WrongParticipant);
    }
    if message.sequence != thread.message_count.saturating_add(1) {
        return Err(ChatIndexError::WrongSequence);
    }
    if message.previous_message_hash != thread.head_message_hash {
        return Err(ChatIndexError::WrongPreviousMessage);
    }
    if message.message_id != message_object_hash(message) {
        return Err(ChatIndexError::InvalidObject);
    }

    thread.entries.push(ThreadMessageRef {
        message_id: message.message_id,
        payload_hash: message.payload_hash,
        sequence: message.sequence,
        created_unix_ms: message.created_unix_ms,
        from: message.from,
        message_kind: message.message_kind,
    });
    thread.message_count = message.sequence;
    if thread.tail_message_hash == [0u8; 32] {
        thread.tail_message_hash = message.message_id;
    }
    thread.head_message_hash = message.message_id;
    thread.updated_unix_ms = message.created_unix_ms;
    thread.thread_hash = thread_object_hash(thread);
    Ok(())
}

pub fn thread_object_hash(value: &ThreadObject) -> Hash {
    let mut builder = HashBuilder::domain(THREAD_OBJECT_DOMAIN)
        .hash(&value.thread_id)
        .node_id(&value.participant_a)
        .node_id(&value.participant_b)
        .u64(value.created_unix_ms)
        .u64(value.updated_unix_ms)
        .u64(value.message_count)
        .hash(&value.head_message_hash)
        .hash(&value.tail_message_hash);
    for entry in &value.entries {
        builder = builder
            .hash(&entry.message_id)
            .hash(&entry.payload_hash)
            .u64(entry.sequence)
            .u64(entry.created_unix_ms)
            .node_id(&entry.from)
            .u16(entry.message_kind);
    }
    builder.finish()
}

pub fn contact_book_hash(value: &ContactBook) -> Hash {
    let mut builder = HashBuilder::domain(CONTACT_BOOK_DOMAIN)
        .node_id(&value.owner)
        .u64(value.updated_unix_ms);
    for contact in &value.contacts {
        builder = builder
            .hash(&contact.contact_id)
            .node_id(&contact.primary_node_id)
            .bytes(&contact.display_name)
            .hash(&contact.avatar_payload_hash);
    }
    for thread in &value.threads {
        builder = builder
            .hash(&thread.thread_id)
            .node_id(&thread.participant_a)
            .node_id(&thread.participant_b)
            .u64(thread.message_count)
            .hash(&thread.head_message_hash)
            .u64(thread.updated_unix_ms);
    }
    builder.finish()
}

pub fn finalize_contact_book(mut value: ContactBook) -> ContactBook {
    value.abi_version = WORK_WIRE_ABI_VERSION;
    value.contact_book_hash = contact_book_hash(&value);
    value
}

pub fn contact_book_ref_for_contact(contact: &ContactObject) -> ContactBookContactRef {
    ContactBookContactRef {
        contact_id: contact.contact_id,
        primary_node_id: contact.primary_node_id,
        display_name: contact.display_name.clone(),
        avatar_payload_hash: contact.avatar_payload_hash,
    }
}

pub fn contact_book_ref_for_thread(thread: &ThreadObject) -> ContactBookThreadRef {
    ContactBookThreadRef {
        thread_id: thread.thread_id,
        participant_a: thread.participant_a,
        participant_b: thread.participant_b,
        message_count: thread.message_count,
        head_message_hash: thread.head_message_hash,
        updated_unix_ms: thread.updated_unix_ms,
    }
}

pub fn chat_payload_hash(payload: &[u8]) -> Hash {
    blake3_hash(payload)
}

pub fn message_object_bytes(value: &MessageObject) -> Result<Vec<u8>, ChatIndexError> {
    wire_bytes(value).map_err(|_| ChatIndexError::InvalidObject)
}

pub fn message_object_from_bytes(bytes: &[u8]) -> Result<MessageObject, ChatIndexError> {
    if bytes.is_empty() {
        return Err(ChatIndexError::InvalidObject);
    }
    wire_from_bytes::<MessageObject, ArchivedMessageObject>(bytes)
        .map_err(|_| ChatIndexError::InvalidObject)
}

pub fn thread_object_bytes(value: &ThreadObject) -> Result<Vec<u8>, ChatIndexError> {
    wire_bytes(value).map_err(|_| ChatIndexError::InvalidObject)
}

pub fn thread_object_from_bytes(bytes: &[u8]) -> Result<ThreadObject, ChatIndexError> {
    if bytes.is_empty() {
        return Err(ChatIndexError::InvalidObject);
    }
    wire_from_bytes::<ThreadObject, ArchivedThreadObject>(bytes)
        .map_err(|_| ChatIndexError::InvalidObject)
}

pub fn contact_book_bytes(value: &ContactBook) -> Result<Vec<u8>, ChatIndexError> {
    wire_bytes(value).map_err(|_| ChatIndexError::InvalidObject)
}

pub fn contact_book_from_bytes(bytes: &[u8]) -> Result<ContactBook, ChatIndexError> {
    if bytes.is_empty() {
        return Err(ChatIndexError::InvalidObject);
    }
    wire_from_bytes::<ContactBook, ArchivedContactBook>(bytes)
        .map_err(|_| ChatIndexError::InvalidObject)
}

fn ordered_participants(a: NodeId, b: NodeId) -> (NodeId, NodeId) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn thread_has_participants(thread: &ThreadObject, from: NodeId, to: NodeId) -> bool {
    (from == thread.participant_a && to == thread.participant_b)
        || (from == thread.participant_b && to == thread.participant_a)
}
