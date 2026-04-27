//! Microphone device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityEventKind};
use edgerun_microphone::{
    AudioCapture, AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat,
};
use edgerun_proto::edgerun::v0::capability::CapabilityInvocation;
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionEvent, CapabilitySessionOpen,
};

use crate::adapters::common::stream_oriented_error;
use crate::protocol::{
    accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult,
};

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
    out.extend_from_slice(&(capture.bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&capture.bytes);
    out
}

/// Binary-decode microphone capture from remote transport.
pub fn decode_microphone_capture(bytes: &[u8]) -> Result<AudioCapture, CapabilityError> {
    if bytes.len() < 22 {
        return Err(CapabilityError::InvalidRequest(
            "remote microphone payload too short",
        ));
    }
    let sample_rate_hz = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let channels = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    let raw_format = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let started_at_unix_ms = i64::from_le_bytes(bytes[10..18].try_into().unwrap());
    let len = u32::from_le_bytes(bytes[18..22].try_into().unwrap()) as usize;
    if bytes.len() != 22 + len {
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
        bytes: bytes[22..].to_vec(),
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

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        Ok(accept_session_open_unchecked(open))
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
