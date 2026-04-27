//! Speaker device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_proto::edgerun::v0::capability::{CapabilityInvocation, CapabilityResult};
use edgerun_proto::edgerun::v0::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionOpen,
};
use edgerun_speaker::{
    AudioPlaybackRequest, AudioPlaybackResult, SpeakerDevice, SpeakerOutputLevel,
    SpeakerSampleFormat,
};

use crate::protocol::{
    accept_session_open_unchecked, RemoteCapabilityProvider, RemoteInvocationResult,
};

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
    out.extend_from_slice(&(request.audio_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&request.audio_bytes);
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
    let duration_ms = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let sample_rate_hz = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let channels = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    let raw_format = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let raw_gain = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
    let raw_level = bytes[16];
    let audio_len_offset = 17;
    if bytes.len() < audio_len_offset + 4 {
        return Err(CapabilityError::InvalidRequest(
            "remote speaker playback request missing audio length",
        ));
    }
    let audio_len = u32::from_le_bytes(
        bytes[audio_len_offset..audio_len_offset + 4]
            .try_into()
            .unwrap(),
    ) as usize;
    if bytes.len() != audio_len_offset + 4 + audio_len {
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
            ))
        }
    };
    Ok(AudioPlaybackRequest {
        duration_ms,
        sample_rate_hz,
        channels,
        format,
        audio_bytes: bytes[audio_len_offset + 4..].to_vec(),
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
            ))
        }
    };
    Ok(SpeakerOutputLevel {
        current_percent: bytes[0],
        min_raw_value: i64::from_le_bytes(bytes[1..9].try_into().unwrap()),
        max_raw_value: i64::from_le_bytes(bytes[9..17].try_into().unwrap()),
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
        bytes_written: u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize,
        sample_rate_hz: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        channels: u16::from_le_bytes(bytes[8..10].try_into().unwrap()),
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
