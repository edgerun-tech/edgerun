//! Input device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_core::protocol::capability::CapabilityInvocation;
use edgerun_core::protocol::capability_runtime::CapabilitySessionEvent;
use edgerun_encoding::byteorder::{read_i32_le, read_i64_le, read_u16_le};
use edgerun_input::{InputDevice, InputEventKind, InputEventRecord};

use crate::adapters::common::{decode_count_u32, encode_count_u32, stream_oriented_error};
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

/// Binary-encode input events for remote transport.
pub fn encode_input_events(events: &[InputEventRecord]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + events.len() * 24);
    encode_count_u32(events.len(), &mut out);
    for event in events {
        out.extend_from_slice(&event.timestamp_sec.to_le_bytes());
        out.extend_from_slice(&event.timestamp_usec.to_le_bytes());
        let kind = match event.kind {
            InputEventKind::Key => 1u16,
            InputEventKind::RelativeMotion => 2,
            InputEventKind::AbsoluteMotion => 3,
            InputEventKind::Switch => 4,
            InputEventKind::Misc => 5,
            InputEventKind::Synchronization => 6,
            InputEventKind::Other(v) => v | 0x8000,
        };
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&event.code.to_le_bytes());
        out.extend_from_slice(&event.value.to_le_bytes());
    }
    out
}

/// Binary-decode input events from remote transport.
pub fn decode_input_events(bytes: &[u8]) -> Result<Vec<InputEventRecord>, CapabilityError> {
    let mut offset = 0usize;
    let count = decode_count_u32(
        bytes,
        &mut offset,
        "remote input payload too short for event count",
    )?;
    let expected = 4 + count * 24;
    if bytes.len() != expected {
        return Err(CapabilityError::InvalidRequest(
            "remote input payload length does not match encoded event count",
        ));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let timestamp_sec = read_i64_le(bytes, offset);
        offset += 8;
        let timestamp_usec = read_i64_le(bytes, offset);
        offset += 8;
        let raw_kind = read_u16_le(bytes, offset);
        offset += 2;
        let code = read_u16_le(bytes, offset);
        offset += 2;
        let value = read_i32_le(bytes, offset);
        offset += 4;
        let kind = match raw_kind {
            1 => InputEventKind::Key,
            2 => InputEventKind::RelativeMotion,
            3 => InputEventKind::AbsoluteMotion,
            4 => InputEventKind::Switch,
            5 => InputEventKind::Misc,
            6 => InputEventKind::Synchronization,
            other if other & 0x8000 != 0 => InputEventKind::Other(other & 0x7fff),
            other => InputEventKind::Other(other),
        };
        out.push(InputEventRecord {
            timestamp_sec,
            timestamp_usec,
            kind,
            code,
            value,
        });
    }
    Ok(out)
}

#[derive(Debug)]
pub struct InputRemoteAdapter<D> {
    pub device: D,
    pub max_events_per_poll: usize,
    next_sequence_no: u64,
}

impl<D> InputRemoteAdapter<D> {
    pub fn new(device: D, max_events_per_poll: usize) -> Self {
        Self {
            device,
            max_events_per_poll,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for InputRemoteAdapter<D>
where
    D: InputDevice,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        _inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        Ok(stream_oriented_error(invocation, "input"))
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let events = self.device.read_events(self.max_events_per_poll)?;
        if events.is_empty() {
            return Ok(None);
        }
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![
                CapabilityEventKind::Touch as i32,
                CapabilityEventKind::Text as i32,
                CapabilityEventKind::State as i32,
            ],
            payload_object: None,
            inline_payload: encode_input_events(&events),
        }))
    }
}
