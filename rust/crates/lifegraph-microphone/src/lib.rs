use lifegraph_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicrophoneSampleFormat {
    PcmS16Le,
    PcmS24Le,
    PcmS32Le,
    Float32Le,
    Other(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MicrophoneInfo {
    pub provider: String,
    pub device_name: String,
    pub instance_id: String,
    pub channels: u16,
    pub sample_rate_hz: u32,
    pub format: MicrophoneSampleFormat,
    pub hardware_aec: bool,
    pub hardware_noise_suppression: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioCaptureRequest {
    pub duration_ms: u32,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub format: MicrophoneSampleFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioCapture {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub format: MicrophoneSampleFormat,
    pub bytes: Vec<u8>,
    pub started_at_unix_ms: i64,
}

pub trait MicrophoneDevice: CapabilityProvider {
    fn microphone_info(&self) -> Result<MicrophoneInfo, CapabilityError>;
    fn capture_audio(
        &mut self,
        request: &AudioCaptureRequest,
    ) -> Result<AudioCapture, CapabilityError>;
}

pub fn default_microphone_descriptor(provider: &str, instance_id: &str) -> CapabilityDescriptor {
    capability_descriptor(
        provider,
        instance_id,
        CapabilityRole::Input,
        &[CapabilityModality::Auditory],
        &[CapabilityEventKind::Auditory],
        &[CapabilityOperation::Query, CapabilityOperation::Capture],
        vec![constraint(CapabilityConstraintKind::RequireUserPresence)],
    )
}

pub fn validate_audio_capture_request(
    request: &AudioCaptureRequest,
) -> Result<(), CapabilityError> {
    if request.duration_ms == 0 {
        return Err(CapabilityError::InvalidRequest(
            "audio capture duration must be greater than zero",
        ));
    }
    if request.sample_rate_hz == 0 {
        return Err(CapabilityError::InvalidRequest(
            "audio sample rate must be greater than zero",
        ));
    }
    if request.channels == 0 {
        return Err(CapabilityError::InvalidRequest(
            "audio capture must request at least one channel",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_microphone_capture_capability() {
        let descriptor = default_microphone_descriptor("alsa", "hw:0,0");
        assert_eq!(descriptor.role, CapabilityRole::Input as i32);
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Auditory as i32]
        );
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Capture as i32)));
    }

    #[test]
    fn capture_request_requires_duration_rate_and_channels() {
        let err = validate_audio_capture_request(&AudioCaptureRequest {
            duration_ms: 0,
            sample_rate_hz: 0,
            channels: 0,
            format: MicrophoneSampleFormat::PcmS16Le,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("audio capture duration must be greater than zero")
        );
    }
}
