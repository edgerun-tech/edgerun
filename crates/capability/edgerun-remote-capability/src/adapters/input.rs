//! Input device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_input::{InputDevice, InputEventKind, InputEventRecord};
use edgerun_protocols::core_protocol::protocol::capability::CapabilityInvocation;
use edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilitySessionEvent;
use edgerun_protocols::wire::{
    RemoteInputEventRecord as InputEventRecordWire, RemoteInputEvents as InputEventsWire,
};

use crate::adapters::common::stream_oriented_error;
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

fn input_event_kind_to_wire(kind: InputEventKind) -> u16 {
    match kind {
        InputEventKind::Key => 1,
        InputEventKind::RelativeMotion => 2,
        InputEventKind::AbsoluteMotion => 3,
        InputEventKind::Switch => 4,
        InputEventKind::Misc => 5,
        InputEventKind::Synchronization => 6,
        InputEventKind::Other(value) => value | 0x8000,
    }
}

fn input_event_kind_from_wire(kind: u16) -> InputEventKind {
    match kind {
        1 => InputEventKind::Key,
        2 => InputEventKind::RelativeMotion,
        3 => InputEventKind::AbsoluteMotion,
        4 => InputEventKind::Switch,
        5 => InputEventKind::Misc,
        6 => InputEventKind::Synchronization,
        other if other & 0x8000 != 0 => InputEventKind::Other(other & 0x7fff),
        other => InputEventKind::Other(other),
    }
}

/// Rkyv-encode input events for remote transport.
pub fn encode_input_events(events: &[InputEventRecord]) -> Vec<u8> {
    let wire = InputEventsWire {
        events: events
            .iter()
            .map(|event| InputEventRecordWire {
                timestamp_sec: event.timestamp_sec,
                timestamp_usec: event.timestamp_usec,
                kind: input_event_kind_to_wire(event.kind),
                code: event.code,
                value: event.value,
            })
            .collect(),
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("input event payload must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode input events from remote transport.
pub fn decode_input_events(bytes: &[u8]) -> Result<Vec<InputEventRecord>, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        InputEventsWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| CapabilityError::InvalidRequest("remote input payload is not rkyv"))?;
    Ok(wire
        .events
        .into_iter()
        .map(|event| InputEventRecord {
            timestamp_sec: event.timestamp_sec,
            timestamp_usec: event.timestamp_usec,
            kind: input_event_kind_from_wire(event.kind),
            code: event.code,
            value: event.value,
        })
        .collect())
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
