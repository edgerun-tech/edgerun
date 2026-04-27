#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use edgerun_capabilities::{
    capability_descriptor, constraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityError, CapabilityEventKind, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MicrophoneSampleFormat {
    #[default]
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
    use alloc::format;
    use alloc::vec;

    // ----- MicrophoneSampleFormat -----

    #[test]
    fn sample_format_equality() {
        assert_eq!(
            MicrophoneSampleFormat::PcmS16Le,
            MicrophoneSampleFormat::PcmS16Le
        );
        assert_ne!(
            MicrophoneSampleFormat::PcmS16Le,
            MicrophoneSampleFormat::PcmS32Le
        );
    }

    #[test]
    fn sample_format_debug() {
        assert_eq!(
            format!("{:?}", MicrophoneSampleFormat::PcmS16Le),
            "PcmS16Le"
        );
        assert_eq!(
            format!("{:?}", MicrophoneSampleFormat::Float32Le),
            "Float32Le"
        );
        assert_eq!(
            format!("{:?}", MicrophoneSampleFormat::Other(42)),
            "Other(42)"
        );
    }

    #[test]
    fn sample_format_other_carries_value() {
        let a = MicrophoneSampleFormat::Other(0x1e);
        let b = MicrophoneSampleFormat::Other(0x1e);
        let c = MicrophoneSampleFormat::Other(0x1f);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn sample_format_copy() {
        let a = MicrophoneSampleFormat::PcmS24Le;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- MicrophoneInfo -----

    #[test]
    fn microphone_info_equality() {
        let info = MicrophoneInfo {
            provider: "alsa".into(),
            device_name: "Built-in Mic".into(),
            instance_id: "hw:0,0".into(),
            channels: 2,
            sample_rate_hz: 48_000,
            format: MicrophoneSampleFormat::PcmS16Le,
            hardware_aec: true,
            hardware_noise_suppression: false,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn microphone_info_debug_format() {
        let info = MicrophoneInfo {
            provider: "test".into(),
            device_name: "Mic".into(),
            instance_id: "hw:1,0".into(),
            channels: 1,
            sample_rate_hz: 16_000,
            format: MicrophoneSampleFormat::PcmS16Le,
            hardware_aec: false,
            hardware_noise_suppression: false,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("Mic"));
        assert!(debug.contains("hw:1,0"));
    }

    #[test]
    fn microphone_info_with_all_hw_features() {
        let info = MicrophoneInfo {
            provider: "alsa".into(),
            device_name: "Array Mic".into(),
            instance_id: "hw:2,0".into(),
            channels: 4,
            sample_rate_hz: 48_000,
            format: MicrophoneSampleFormat::PcmS32Le,
            hardware_aec: true,
            hardware_noise_suppression: true,
        };
        assert!(info.hardware_aec);
        assert!(info.hardware_noise_suppression);
        assert_eq!(info.channels, 4);
    }

    // ----- AudioCaptureRequest -----

    #[test]
    fn capture_request_equality() {
        let req = AudioCaptureRequest {
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 2,
            format: MicrophoneSampleFormat::PcmS16Le,
        };
        let cloned = req.clone();
        assert_eq!(req, cloned);
    }

    #[test]
    fn capture_request_debug_format() {
        let req = AudioCaptureRequest {
            duration_ms: 50,
            sample_rate_hz: 16_000,
            channels: 1,
            format: MicrophoneSampleFormat::PcmS32Le,
        };
        let debug = format!("{:?}", req);
        assert!(debug.contains("50"));
        assert!(debug.contains("16000"));
    }

    // ----- AudioCapture -----

    #[test]
    fn audio_capture_equality() {
        let cap = AudioCapture {
            sample_rate_hz: 48_000,
            channels: 2,
            format: MicrophoneSampleFormat::PcmS16Le,
            bytes: vec![0, 1, 2, 3],
            started_at_unix_ms: 1_000_000,
        };
        let cloned = cap.clone();
        assert_eq!(cap, cloned);
    }

    #[test]
    fn audio_capture_bytes_differ() {
        let a = AudioCapture {
            sample_rate_hz: 48_000,
            channels: 1,
            format: MicrophoneSampleFormat::PcmS16Le,
            bytes: vec![0],
            started_at_unix_ms: 0,
        };
        let b = AudioCapture {
            sample_rate_hz: 48_000,
            channels: 1,
            format: MicrophoneSampleFormat::PcmS16Le,
            bytes: vec![1],
            started_at_unix_ms: 0,
        };
        assert_ne!(a, b);
    }

    // ----- default_microphone_descriptor -----

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
    fn descriptor_has_query_and_capture() {
        let descriptor = default_microphone_descriptor("test", "id");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Capture as i32)));
    }

    #[test]
    fn descriptor_has_require_user_presence_constraint() {
        let descriptor = default_microphone_descriptor("test", "id");
        assert!(descriptor
            .default_constraints
            .iter()
            .any(|c| c.kind == CapabilityConstraintKind::RequireUserPresence as i32));
    }

    #[test]
    fn descriptor_uses_provider_and_instance() {
        let descriptor = default_microphone_descriptor("my-mic", "mic-0");
        assert_eq!(descriptor.provider_name, "my-mic");
        assert_eq!(descriptor.provider_instance_id, "mic-0");
    }

    // ----- validate_audio_capture_request -----

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

    #[test]
    fn capture_request_rejects_zero_sample_rate() {
        let err = validate_audio_capture_request(&AudioCaptureRequest {
            duration_ms: 100,
            sample_rate_hz: 0,
            channels: 2,
            format: MicrophoneSampleFormat::PcmS16Le,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("audio sample rate must be greater than zero")
        );
    }

    #[test]
    fn capture_request_rejects_zero_channels() {
        let err = validate_audio_capture_request(&AudioCaptureRequest {
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 0,
            format: MicrophoneSampleFormat::PcmS16Le,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("audio capture must request at least one channel")
        );
    }

    #[test]
    fn capture_request_valid_succeeds() {
        let req = AudioCaptureRequest {
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 2,
            format: MicrophoneSampleFormat::PcmS16Le,
        };
        assert!(validate_audio_capture_request(&req).is_ok());
    }

    #[test]
    fn capture_request_valid_with_minimum_values() {
        let req = AudioCaptureRequest {
            duration_ms: 1,
            sample_rate_hz: 1,
            channels: 1,
            format: MicrophoneSampleFormat::PcmS16Le,
        };
        assert!(validate_audio_capture_request(&req).is_ok());
    }
}
