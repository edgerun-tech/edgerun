use lifegraph_capabilities::{
    CapabilityDescriptor, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole, capability_descriptor,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeakerSampleFormat {
    PcmS16Le,
    PcmS24Le,
    PcmFloat32Le,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeakerInfo {
    pub provider: String,
    pub instance_id: String,
    pub display_name: String,
    pub default_sample_rate_hz: u32,
    pub channels: u16,
    pub supports_playback: bool,
    pub supports_output_level_control: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeakerOutputLevel {
    pub current_percent: u8,
    pub min_raw_value: i64,
    pub max_raw_value: i64,
    pub muted: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioPlaybackRequest {
    pub duration_ms: u32,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub format: SpeakerSampleFormat,
    pub audio_bytes: Vec<u8>,
    pub software_gain_percent: Option<u16>,
    pub target_output_level_percent: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioPlaybackResult {
    pub bytes_written: usize,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub finished: bool,
}

pub trait SpeakerDevice: CapabilityProvider {
    fn speaker_info(&self) -> Result<SpeakerInfo, lifegraph_capabilities::CapabilityError>;
    fn output_level(
        &self,
    ) -> Result<Option<SpeakerOutputLevel>, lifegraph_capabilities::CapabilityError> {
        Ok(None)
    }
    fn set_output_level(
        &self,
        _percent: u8,
    ) -> Result<Option<SpeakerOutputLevel>, lifegraph_capabilities::CapabilityError> {
        Err(lifegraph_capabilities::CapabilityError::Unsupported(
            "speaker output level control is not supported",
        ))
    }
    fn play_audio(
        &self,
        request: &AudioPlaybackRequest,
    ) -> Result<AudioPlaybackResult, lifegraph_capabilities::CapabilityError>;
}

pub fn default_speaker_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Output,
        &[CapabilityModality::Auditory],
        &[CapabilityEventKind::Auditory],
        &[CapabilityOperation::Query, CapabilityOperation::Render],
        Vec::new(),
    )
}

pub fn validate_audio_playback_request(
    request: &AudioPlaybackRequest,
) -> Result<(), lifegraph_capabilities::CapabilityError> {
    if request.duration_ms == 0 {
        return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
            "playback duration must be > 0",
        ));
    }
    if request.sample_rate_hz == 0 {
        return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
            "sample rate must be > 0",
        ));
    }
    if request.channels == 0 {
        return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
            "channels must be > 0",
        ));
    }
    if request.audio_bytes.is_empty() {
        return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
            "audio bytes must be non-empty",
        ));
    }
    if let Some(percent) = request.software_gain_percent {
        if percent > 400 {
            return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
                "software gain percent must be <= 400",
            ));
        }
    }
    if let Some(percent) = request.target_output_level_percent {
        if percent > 100 {
            return Err(lifegraph_capabilities::CapabilityError::InvalidRequest(
                "target output level percent must be <= 100",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_speaker_output_capability() {
        let descriptor = default_speaker_descriptor("speaker", "default");
        assert_eq!(descriptor.role, CapabilityRole::Output as i32);
        assert_eq!(
            descriptor.event_kinds,
            vec![CapabilityEventKind::Auditory as i32]
        );
    }

    #[test]
    fn playback_request_requires_duration_rate_channels_and_bytes() {
        let request = AudioPlaybackRequest {
            duration_ms: 0,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![1, 2],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        assert!(validate_audio_playback_request(&request).is_err());
    }

    #[test]
    fn playback_request_validates_gain_and_target_level() {
        let request = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![1, 2],
            software_gain_percent: Some(401),
            target_output_level_percent: Some(101),
        };
        assert!(validate_audio_playback_request(&request).is_err());
    }
}
