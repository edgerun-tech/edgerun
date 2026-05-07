//! Speaker device remote adapter.

use crate::prelude::v1::*;
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityOperation,
};
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityInvocation, CapabilityResult,
};
use edgerun_protocols::wire::{
    RemoteAudioPlaybackRequest as AudioPlaybackRequestWire,
    RemoteAudioPlaybackResult as AudioPlaybackResultWire,
    RemoteSpeakerOutputLevel as SpeakerOutputLevelWire,
};
use edgerun_speaker::{
    AudioPlaybackRequest, AudioPlaybackResult, SpeakerDevice, SpeakerOutputLevel,
    SpeakerSampleFormat,
};

use crate::protocol::{RemoteCapabilityProvider, RemoteInvocationResult};

fn speaker_format_to_wire(format: SpeakerSampleFormat) -> u32 {
    match format {
        SpeakerSampleFormat::PcmS16Le => 1,
        SpeakerSampleFormat::PcmS24Le => 2,
        SpeakerSampleFormat::PcmFloat32Le => 3,
    }
}

fn speaker_format_from_wire(format: u32) -> Result<SpeakerSampleFormat, CapabilityError> {
    match format {
        1 => Ok(SpeakerSampleFormat::PcmS16Le),
        2 => Ok(SpeakerSampleFormat::PcmS24Le),
        3 => Ok(SpeakerSampleFormat::PcmFloat32Le),
        _ => Err(CapabilityError::InvalidRequest(
            "remote speaker playback request format is unknown",
        )),
    }
}

/// Rkyv-encode speaker playback request.
pub fn encode_speaker_playback_request(request: &AudioPlaybackRequest) -> Vec<u8> {
    let wire = AudioPlaybackRequestWire {
        duration_ms: request.duration_ms,
        sample_rate_hz: request.sample_rate_hz,
        channels: request.channels,
        format: speaker_format_to_wire(request.format),
        audio_bytes: request.audio_bytes.clone(),
        software_gain_percent: request.software_gain_percent,
        target_output_level_percent: request.target_output_level_percent,
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("speaker playback request must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode speaker playback request.
pub fn decode_speaker_playback_request(
    bytes: &[u8],
) -> Result<AudioPlaybackRequest, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        AudioPlaybackRequestWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| CapabilityError::InvalidRequest("remote speaker playback request is not rkyv"))?;
    Ok(AudioPlaybackRequest {
        duration_ms: wire.duration_ms,
        sample_rate_hz: wire.sample_rate_hz,
        channels: wire.channels,
        format: speaker_format_from_wire(wire.format)?,
        audio_bytes: wire.audio_bytes,
        software_gain_percent: wire.software_gain_percent,
        target_output_level_percent: wire.target_output_level_percent,
    })
}

/// Rkyv-encode speaker output level.
pub fn encode_speaker_output_level(level: &SpeakerOutputLevel) -> Vec<u8> {
    let wire = SpeakerOutputLevelWire {
        current_percent: level.current_percent,
        min_raw_value: level.min_raw_value,
        max_raw_value: level.max_raw_value,
        muted: level.muted,
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("speaker output level must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode speaker output level.
pub fn decode_speaker_output_level(bytes: &[u8]) -> Result<SpeakerOutputLevel, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        SpeakerOutputLevelWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| CapabilityError::InvalidRequest("remote speaker output level is not rkyv"))?;
    Ok(SpeakerOutputLevel {
        current_percent: wire.current_percent,
        min_raw_value: wire.min_raw_value,
        max_raw_value: wire.max_raw_value,
        muted: wire.muted,
    })
}

/// Rkyv-encode speaker playback result.
pub fn encode_speaker_playback_result(result: &AudioPlaybackResult) -> Vec<u8> {
    let wire = AudioPlaybackResultWire {
        bytes_written: result.bytes_written as u64,
        sample_rate_hz: result.sample_rate_hz,
        channels: result.channels,
        finished: result.finished,
    };
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&wire)
        .expect("speaker playback result must serialize through rkyv")
        .into_vec()
}

/// Rkyv-decode speaker playback result.
pub fn decode_speaker_playback_result(
    bytes: &[u8],
) -> Result<AudioPlaybackResult, CapabilityError> {
    let owned = bytes.to_vec();
    let wire = edgerun_protocols::wire::from_bytes::<
        AudioPlaybackResultWire,
        edgerun_protocols::wire::WireError,
    >(&owned)
    .map_err(|_| CapabilityError::InvalidRequest("remote speaker playback result is not rkyv"))?;
    Ok(AudioPlaybackResult {
        bytes_written: wire.bytes_written as usize,
        sample_rate_hz: wire.sample_rate_hz,
        channels: wire.channels,
        finished: wire.finished,
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
