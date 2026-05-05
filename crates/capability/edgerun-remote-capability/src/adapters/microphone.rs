//! Microphone device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_core::protocol::capability::CapabilityInvocation;
use edgerun_core::protocol::capability_runtime::CapabilitySessionEvent;
use edgerun_encoding::byteorder::{read_i64_le, read_u16_le, read_u32_le};
use edgerun_microphone::{
    AudioCapture, AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat,
};

use crate::adapters::common::{decode_byte_field, encode_byte_field, stream_oriented_error};
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

/// Binary-encode microphone capture for remote transport.
pub fn encode_microphone_capture(capture: &AudioCapture) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 4 + 8 + 4 + capture.bytes.len());
    out.extend_from_slice(&capture.sample_rate_hz.to_le_bytes());
    out.extend_from_slice(&capture.channels.to_le_bytes());
    let format = match capture.format {
        MicrophoneSampleFormat::PcmS16Le => 1u32,
        MicrophoneSampleFormat::PcmS24Le => 2,
        MicrophoneSampleFormat::PcmS32Le => 3,
        MicrophoneSampleFormat::Float32Le => 4,
        MicrophoneSampleFormat::Other(v) => v | 0x8000_0000,
    };
    out.extend_from_slice(&format.to_le_bytes());
    out.extend_from_slice(&capture.started_at_unix_ms.to_le_bytes());
    encode_byte_field(&capture.bytes, &mut out);
    out
}

/// Binary-decode microphone capture from remote transport.
pub fn decode_microphone_capture(bytes: &[u8]) -> Result<AudioCapture, CapabilityError> {
    if bytes.len() < 22 {
        return Err(CapabilityError::InvalidRequest(
            "remote microphone payload too short",
        ));
    }
    let sample_rate_hz = read_u32_le(bytes, 0);
    let channels = read_u16_le(bytes, 4);
    let raw_format = read_u32_le(bytes, 6);
    let started_at_unix_ms = read_i64_le(bytes, 10);
    let mut cursor = 18;
    let capture_bytes = decode_byte_field(bytes, &mut cursor)?;
    if bytes.len() != cursor {
        return Err(CapabilityError::InvalidRequest(
            "remote microphone payload length does not match encoded byte count",
        ));
    }
    let format = match raw_format {
        1 => MicrophoneSampleFormat::PcmS16Le,
        2 => MicrophoneSampleFormat::PcmS24Le,
        3 => MicrophoneSampleFormat::PcmS32Le,
        4 => MicrophoneSampleFormat::Float32Le,
        other if other & 0x8000_0000 != 0 => MicrophoneSampleFormat::Other(other & 0x7fff_ffff),
        other => MicrophoneSampleFormat::Other(other),
    };
    Ok(AudioCapture {
        sample_rate_hz,
        channels,
        format,
        bytes: capture_bytes,
        started_at_unix_ms,
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
