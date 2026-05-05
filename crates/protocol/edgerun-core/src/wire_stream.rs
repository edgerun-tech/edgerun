//! Rkyv-only stream wire boundary.

use alloc::vec::Vec;
use core::convert::TryInto;

use crate::protocol::ProtocolRecord;
use crate::protocol::{
    protocol_wire_bytes, CommandEnvelope, CommandRef, CommandResultPayload, DelegationRef, Digest,
    EventEnvelope, EventRef, ObjectRef, RevocationRef, Signature, Timestamp,
};

const EVENT_FULL_MAGIC: &[u8; 4] = b"EREV";
const EVENT_FULL_VERSION: u8 = 1;

pub fn event_signable_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true)
}

pub fn event_full_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    encode_event_full(event)
}

pub fn decode_event_full_wire_bytes(bytes: &[u8]) -> Result<EventEnvelope, &'static str> {
    decode_event_full(bytes)
}

pub fn command_signable_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::CommandEnvelope(command.clone()), true)
}

pub fn command_full_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::CommandEnvelope(command.clone()), false)
}

pub fn command_result_wire_bytes(result: &CommandResultPayload) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::CommandResultPayload(result.clone()), false)
}

fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_i64(out: &mut Vec<u8>, value: i64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_bytes(out: &mut Vec<u8>, value: &[u8]) {
    put_u32(out, value.len() as u32);
    out.extend_from_slice(value);
}

fn put_option_i32(out: &mut Vec<u8>, value: Option<i32>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_i32(out, value);
        }
        None => put_u8(out, 0),
    }
}

fn put_option_bytes(out: &mut Vec<u8>, value: &Option<Vec<u8>>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_bytes(out, value);
        }
        None => put_u8(out, 0),
    }
}

fn put_digest(out: &mut Vec<u8>, value: &Digest) {
    put_i32(out, value.algorithm);
    put_bytes(out, &value.value);
}

fn put_option_digest(out: &mut Vec<u8>, value: &Option<Digest>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_digest(out, value);
        }
        None => put_u8(out, 0),
    }
}

fn put_signature(out: &mut Vec<u8>, value: &Signature) {
    put_i32(out, value.algorithm);
    put_bytes(out, &value.value);
}

fn put_option_signature(out: &mut Vec<u8>, value: &Option<Signature>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_signature(out, value);
        }
        None => put_u8(out, 0),
    }
}

fn put_timestamp(out: &mut Vec<u8>, value: &Timestamp) {
    put_i64(out, value.seconds);
    put_i32(out, value.nanos);
}

fn put_option_timestamp(out: &mut Vec<u8>, value: &Option<Timestamp>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_timestamp(out, value);
        }
        None => put_u8(out, 0),
    }
}

fn put_object_ref(out: &mut Vec<u8>, value: &ObjectRef) {
    put_bytes(out, &value.object_id);
    put_option_i32(out, value.object_kind);
}

fn put_option_object_ref(out: &mut Vec<u8>, value: &Option<ObjectRef>) {
    match value {
        Some(value) => {
            put_u8(out, 1);
            put_object_ref(out, value);
        }
        None => put_u8(out, 0),
    }
}

fn put_event_ref(out: &mut Vec<u8>, value: &EventRef) {
    put_bytes(out, &value.stream_id);
    put_u64(out, value.seq);
    put_option_digest(out, &value.event_hash);
}

fn put_command_ref(out: &mut Vec<u8>, value: &CommandRef) {
    put_bytes(out, &value.command_id);
    put_option_digest(out, &value.command_hash);
}

fn put_delegation_ref(out: &mut Vec<u8>, value: &DelegationRef) {
    put_bytes(out, &value.delegation_id);
    put_option_digest(out, &value.delegation_hash);
}

fn put_revocation_ref(out: &mut Vec<u8>, value: &RevocationRef) {
    put_bytes(out, &value.revocation_id);
    put_option_digest(out, &value.revocation_hash);
}

fn put_vec<T>(out: &mut Vec<u8>, values: &[T], mut put: impl FnMut(&mut Vec<u8>, &T)) {
    put_u32(out, values.len() as u32);
    for value in values {
        put(out, value);
    }
}

fn encode_event_full(event: &EventEnvelope) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(EVENT_FULL_MAGIC);
    put_u8(&mut out, EVENT_FULL_VERSION);
    put_u32(&mut out, event.envelope_version);
    put_bytes(&mut out, &event.stream_id);
    put_u64(&mut out, event.seq);
    put_option_digest(&mut out, &event.prev_event_hash);
    put_i32(&mut out, event.event_type);
    put_u32(&mut out, event.event_version);
    put_option_timestamp(&mut out, &event.recorded_at);
    put_option_timestamp(&mut out, &event.effective_at);
    put_option_object_ref(&mut out, &event.payload_object);
    put_vec(&mut out, &event.related_events, put_event_ref);
    put_vec(&mut out, &event.related_commands, put_command_ref);
    put_vec(&mut out, &event.related_objects, put_object_ref);
    put_vec(&mut out, &event.related_delegations, put_delegation_ref);
    put_vec(&mut out, &event.related_revocations, put_revocation_ref);
    put_option_object_ref(&mut out, &event.event_metadata);
    put_option_signature(&mut out, &event.signature);
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], &'static str> {
        let end = self.pos.checked_add(len).ok_or("event frame overflow")?;
        let chunk = self
            .bytes
            .get(self.pos..end)
            .ok_or("truncated event frame")?;
        self.pos = end;
        Ok(chunk)
    }

    fn u8(&mut self) -> Result<u8, &'static str> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| "invalid u32")?,
        ))
    }

    fn i32(&mut self) -> Result<i32, &'static str> {
        Ok(i32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| "invalid i32")?,
        ))
    }

    fn i64(&mut self) -> Result<i64, &'static str> {
        Ok(i64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| "invalid i64")?,
        ))
    }

    fn u64(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| "invalid u64")?,
        ))
    }

    fn bytes(&mut self) -> Result<Vec<u8>, &'static str> {
        let len = self.u32()? as usize;
        Ok(self.take(len)?.to_vec())
    }

    fn done(&self) -> bool {
        self.pos == self.bytes.len()
    }
}

fn take_option<T>(
    reader: &mut Reader<'_>,
    take: impl FnOnce(&mut Reader<'_>) -> Result<T, &'static str>,
) -> Result<Option<T>, &'static str> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(take(reader)?)),
        _ => Err("invalid option tag"),
    }
}

fn take_option_i32(reader: &mut Reader<'_>) -> Result<Option<i32>, &'static str> {
    take_option(reader, |reader| reader.i32())
}

fn take_option_bytes(reader: &mut Reader<'_>) -> Result<Option<Vec<u8>>, &'static str> {
    take_option(reader, |reader| reader.bytes())
}

fn take_digest(reader: &mut Reader<'_>) -> Result<Digest, &'static str> {
    Ok(Digest {
        algorithm: reader.i32()?,
        value: reader.bytes()?,
    })
}

fn take_option_digest(reader: &mut Reader<'_>) -> Result<Option<Digest>, &'static str> {
    take_option(reader, take_digest)
}

fn take_signature(reader: &mut Reader<'_>) -> Result<Signature, &'static str> {
    Ok(Signature {
        algorithm: reader.i32()?,
        value: reader.bytes()?,
    })
}

fn take_option_signature(reader: &mut Reader<'_>) -> Result<Option<Signature>, &'static str> {
    take_option(reader, take_signature)
}

fn take_timestamp(reader: &mut Reader<'_>) -> Result<Timestamp, &'static str> {
    Ok(Timestamp {
        seconds: reader.i64()?,
        nanos: reader.i32()?,
    })
}

fn take_option_timestamp(reader: &mut Reader<'_>) -> Result<Option<Timestamp>, &'static str> {
    take_option(reader, take_timestamp)
}

fn take_object_ref(reader: &mut Reader<'_>) -> Result<ObjectRef, &'static str> {
    Ok(ObjectRef {
        object_id: reader.bytes()?,
        object_kind: take_option_i32(reader)?,
    })
}

fn take_option_object_ref(reader: &mut Reader<'_>) -> Result<Option<ObjectRef>, &'static str> {
    take_option(reader, take_object_ref)
}

fn take_event_ref(reader: &mut Reader<'_>) -> Result<EventRef, &'static str> {
    Ok(EventRef {
        stream_id: reader.bytes()?,
        seq: reader.u64()?,
        event_hash: take_option_digest(reader)?,
    })
}

fn take_command_ref(reader: &mut Reader<'_>) -> Result<CommandRef, &'static str> {
    Ok(CommandRef {
        command_id: reader.bytes()?,
        command_hash: take_option_digest(reader)?,
    })
}

fn take_delegation_ref(reader: &mut Reader<'_>) -> Result<DelegationRef, &'static str> {
    Ok(DelegationRef {
        delegation_id: reader.bytes()?,
        delegation_hash: take_option_digest(reader)?,
    })
}

fn take_revocation_ref(reader: &mut Reader<'_>) -> Result<RevocationRef, &'static str> {
    Ok(RevocationRef {
        revocation_id: reader.bytes()?,
        revocation_hash: take_option_digest(reader)?,
    })
}

fn take_vec<T>(
    reader: &mut Reader<'_>,
    mut take: impl FnMut(&mut Reader<'_>) -> Result<T, &'static str>,
) -> Result<Vec<T>, &'static str> {
    let len = reader.u32()? as usize;
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(take(reader)?);
    }
    Ok(values)
}

fn decode_event_full(bytes: &[u8]) -> Result<EventEnvelope, &'static str> {
    let mut reader = Reader::new(bytes);
    if reader.take(EVENT_FULL_MAGIC.len())? != EVENT_FULL_MAGIC {
        return Err("invalid event frame magic");
    }
    if reader.u8()? != EVENT_FULL_VERSION {
        return Err("unsupported event frame version");
    }
    let event = EventEnvelope {
        envelope_version: reader.u32()?,
        stream_id: reader.bytes()?,
        seq: reader.u64()?,
        prev_event_hash: take_option_digest(&mut reader)?,
        event_type: reader.i32()?,
        event_version: reader.u32()?,
        recorded_at: take_option_timestamp(&mut reader)?,
        effective_at: take_option_timestamp(&mut reader)?,
        payload_object: take_option_object_ref(&mut reader)?,
        related_events: take_vec(&mut reader, take_event_ref)?,
        related_commands: take_vec(&mut reader, take_command_ref)?,
        related_objects: take_vec(&mut reader, take_object_ref)?,
        related_delegations: take_vec(&mut reader, take_delegation_ref)?,
        related_revocations: take_vec(&mut reader, take_revocation_ref)?,
        event_metadata: take_option_object_ref(&mut reader)?,
        signature: take_option_signature(&mut reader)?,
    };
    if !reader.done() {
        return Err("trailing event frame bytes");
    }
    Ok(event)
}
