//! Microphone device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_microphone::{
    AudioCapture, AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat,
};
use edgerun_protocols::core_protocol::protocol::capability::CapabilityInvocation;
use edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilitySessionEvent;
use edgerun_protocols::wire::RemoteAudioCapture as AudioCaptureWire;

use crate::adapters::common::stream_oriented_error;
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

fn microphone_format_to_wire(format: MicrophoneSampleFormat) -> u32 {
    match format {
        MicrophoneSampleFormat::PcmS16Le => 1,
        MicrophoneSampleFormat::PcmS24Le => 2,
        MicrophoneSampleFormat::PcmS32Le => 3,
        MicrophoneSampleFormat::Float32Le => 4,
        MicrophoneSampleFormat::Other(value) => value | 0x8000_0000,
    }
}

fn microphone_format_from_wire(format: u32) -> MicrophoneSampleFormat {
    match format {
        1 => MicrophoneSampleFormat::PcmS16Le,
        2 => MicrophoneSampleFormat::PcmS24Le,
        3 => MicrophoneSampleFormat::PcmS32Le,
        4 => MicrophoneSampleFormat::Float32Le,
        other if other & 0x8000_0000 != 0 => MicrophoneSampleFormat::Other(other & 0x7fff_ffff),
        other => MicrophoneSampleFormat::Other(other),
    }
}

/// Rkyv-encode microphone capture for remote transport.
pub fn encode_microphone_capture(capture: &AudioCapture) -> Vec<u8> {
    let wire = AudioCaptureWire {
        sample_rate_hz: capture.sample_rate_hz,
        channels: capture.channels,
        format: microphone_format_to_wire(capture.format),
        started_at_unix_ms: capture.started_at_unix_ms,
        bytes: capture.bytes.clone(),
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("microphone capture payload must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode microphone capture from remote transport.
pub fn decode_microphone_capture(bytes: &[u8]) -> Result<AudioCapture, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        AudioCaptureWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| CapabilityError::InvalidRequest("remote microphone payload is not rkyv"))?;
    Ok(AudioCapture {
        sample_rate_hz: wire.sample_rate_hz,
        channels: wire.channels,
        format: microphone_format_from_wire(wire.format),
        bytes: wire.bytes,
        started_at_unix_ms: wire.started_at_unix_ms,
    })
}

#[derive(Debug)]
pub struct MicrophoneRemoteAdapter<D> {
    pub device: D,
    pub capture_request: AudioCaptureRequest,
    next_sequence_no: u64,
}

impl<D> MicrophoneRemoteAdapter<D> {
    pub fn new(device: D, capture_request: AudioCaptureRequest) -> Self {
        Self {
            device,
            capture_request,
            next_sequence_no: 1,
        }
    }
}

impl<D> RemoteCapabilityProvider for MicrophoneRemoteAdapter<D>
where
    D: MicrophoneDevice,
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
        Ok(stream_oriented_error(invocation, "microphone"))
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<Option<CapabilitySessionEvent>, CapabilityError> {
        let capture = self.device.capture_audio(&self.capture_request)?;
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no += 1;
        Ok(Some(CapabilitySessionEvent {
            version: 1,
            session_id: session_id.to_vec(),
            sequence_no,
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            payload_object: None,
            inline_payload: encode_microphone_capture(&capture),
        }))
    }
}
