//! Speaker device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_encoding::byteorder::{read_i64_le, read_u16_le, read_u32_le};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_speaker::{
    AudioPlaybackRequest, AudioPlaybackResult, SpeakerDevice, SpeakerOutputLevel,
    SpeakerSampleFormat,
};

use crate::adapters::common::{decode_byte_field, encode_byte_field};
use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

/// Binary-encode speaker playback request.
pub fn encode_speaker_playback_request(request: &AudioPlaybackRequest) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 + 2 + 4 + 2 + 1 + request.audio_bytes.len());
    out.extend_from_slice(&request.duration_ms.to_le_bytes());
    out.extend_from_slice(&request.sample_rate_hz.to_le_bytes());
    out.extend_from_slice(&request.channels.to_le_bytes());
    let format = match request.format {
        SpeakerSampleFormat::PcmS16Le => 1u32,
        SpeakerSampleFormat::PcmS24Le => 2,
        SpeakerSampleFormat::PcmFloat32Le => 3,
    };
    out.extend_from_slice(&format.to_le_bytes());
    out.extend_from_slice(&request.software_gain_percent.unwrap_or(0).to_le_bytes());
    out.push(request.target_output_level_percent.unwrap_or(255));
    encode_byte_field(&request.audio_bytes, &mut out);
    out
}

/// Binary-decode speaker playback request.
pub fn decode_speaker_playback_request(
    bytes: &[u8],
) -> Result<AudioPlaybackRequest, CapabilityError> {
    if bytes.len() < 17 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback request payload too short",
        ));
    }
    let duration_ms = read_u32_le(bytes, 0);
    let sample_rate_hz = read_u32_le(bytes, 4);
    let channels = read_u16_le(bytes, 8);
    let raw_format = read_u32_le(bytes, 10);
    let raw_gain = read_u16_le(bytes, 14);
    let raw_level = bytes[16];
    let mut cursor = 17;
    let audio_bytes = decode_byte_field(bytes, &mut cursor)?;
    if bytes.len() != cursor {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback request length does not match encoded byte count",
        ));
    }
    let format = match raw_format {
        1 => SpeakerSampleFormat::PcmS16Le,
        2 => SpeakerSampleFormat::PcmS24Le,
        3 => SpeakerSampleFormat::PcmFloat32Le,
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote speaker playback request format is unknown",
            ));
        }
    };
    Ok(AudioPlaybackRequest {
        duration_ms,
        sample_rate_hz,
        channels,
        format,
        audio_bytes,
        software_gain_percent: if raw_gain == 0 { None } else { Some(raw_gain) },
        target_output_level_percent: if raw_level == 255 {
            None
        } else {
            Some(raw_level)
        },
    })
}

/// Binary-encode speaker output level.
pub fn encode_speaker_output_level(level: &SpeakerOutputLevel) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + 8 + 8 + 1);
    out.push(level.current_percent);
    out.extend_from_slice(&level.min_raw_value.to_le_bytes());
    out.extend_from_slice(&level.max_raw_value.to_le_bytes());
    out.push(match level.muted {
        Some(true) => 1,
        Some(false) => 2,
        None => 0,
    });
    out
}

/// Binary-decode speaker output level.
pub fn decode_speaker_output_level(bytes: &[u8]) -> Result<SpeakerOutputLevel, CapabilityError> {
    if bytes.len() != 18 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker output level payload length is invalid",
        ));
    }
    let muted = match bytes[17] {
        0 => None,
        1 => Some(true),
        2 => Some(false),
        _ => {
            return Err(CapabilityError::InvalidRequest(
                "remote speaker output level mute flag is invalid",
            ));
        }
    };
    Ok(SpeakerOutputLevel {
        current_percent: bytes[0],
        min_raw_value: read_i64_le(bytes, 1),
        max_raw_value: read_i64_le(bytes, 9),
        muted,
    })
}

/// Binary-encode speaker playback result.
pub fn encode_speaker_playback_result(result: &AudioPlaybackResult) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 + 2 + 1);
    out.extend_from_slice(&(result.bytes_written as u32).to_le_bytes());
    out.extend_from_slice(&result.sample_rate_hz.to_le_bytes());
    out.extend_from_slice(&result.channels.to_le_bytes());
    out.push(result.finished as u8);
    out
}

/// Binary-decode speaker playback result.
pub fn decode_speaker_playback_result(
    bytes: &[u8],
) -> Result<AudioPlaybackResult, CapabilityError> {
    if bytes.len() != 11 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback result payload length is invalid",
        ));
    }
    Ok(AudioPlaybackResult {
        bytes_written: read_u32_le(bytes, 0) as usize,
        sample_rate_hz: read_u32_le(bytes, 4),
        channels: read_u16_le(bytes, 8),
        finished: bytes[10] != 0,
    })
}

#[derive(Debug)]
pub struct SpeakerRemoteAdapter<D> {
    pub device: D,
}

impl<D> SpeakerRemoteAdapter<D> {
    pub fn new(device: D) -> Self {
        Self { device }
    }
}

impl<D> RemoteCapabilityProvider for SpeakerRemoteAdapter<D>
where
    D: SpeakerDevice,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.device.descriptor()
    }

    fn invoke(
        &mut self,
        _session_id: &[u8],
        invocation: &CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let parameters = inline_parameters.ok_or(CapabilityError::InvalidRequest(
            "speaker remote adapter requires inline playback parameters",
        ))?;
        let request = decode_speaker_playback_request(parameters)?;
        let output_level = if invocation.operation == CapabilityOperation::Control as i32 {
            let target =
                request
                    .target_output_level_percent
                    .ok_or(CapabilityError::InvalidRequest(
                        "speaker control invocation requires target output level percent",
                    ))?;
            self.device.set_output_level(target)?
        } else {
            None
        };
        let playback = if invocation.operation == CapabilityOperation::Render as i32 {
            Some(self.device.play_audio(&request)?)
        } else {
            None
        };
        let inline_payload = if let Some(playback) = playback {
            encode_speaker_playback_result(&playback)
        } else if let Some(level) = output_level {
            encode_speaker_output_level(&level)
        } else {
            Vec::new()
        };
        Ok(RemoteInvocationResult {
            result: CapabilityResult {
                result_version: 1,
                invocation_id: invocation.invocation_id.clone(),
                grant_id: invocation.grant_id.clone(),
                success: true,
                result_access_class: invocation.requested_access_class,
                produced_event_kinds: vec![CapabilityEventKind::Auditory as i32],
                payload_object: None,
                error_reason: String::new(),
                produced_at: None,
                signature: None,
            },
            inline_payload,
        })
    }
}
