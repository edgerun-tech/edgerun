#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpeakerSampleFormat {
    #[default]
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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
    fn speaker_info(&self) -> Result<SpeakerInfo, edgerun_capabilities::CapabilityError>;
    fn output_level(
        &self,
    ) -> Result<Option<SpeakerOutputLevel>, edgerun_capabilities::CapabilityError> {
        Ok(None)
    }
    fn set_output_level(
        &self,
        _percent: u8,
    ) -> Result<Option<SpeakerOutputLevel>, edgerun_capabilities::CapabilityError> {
        Err(edgerun_capabilities::CapabilityError::Unsupported(
            "speaker output level control is not supported",
        ))
    }
    fn play_audio(
        &self,
        request: &AudioPlaybackRequest,
    ) -> Result<AudioPlaybackResult, edgerun_capabilities::CapabilityError>;
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
) -> Result<(), edgerun_capabilities::CapabilityError> {
    if request.duration_ms == 0 {
        return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
            "playback duration must be > 0",
        ));
    }
    if request.sample_rate_hz == 0 {
        return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
            "sample rate must be > 0",
        ));
    }
    if request.channels == 0 {
        return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
            "channels must be > 0",
        ));
    }
    if request.audio_bytes.is_empty() {
        return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
            "audio bytes must be non-empty",
        ));
    }
    if let Some(percent) = request.software_gain_percent {
        if percent > 400 {
            return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
                "software gain percent must be <= 400",
            ));
        }
    }
    if let Some(percent) = request.target_output_level_percent {
        if percent > 100 {
            return Err(edgerun_capabilities::CapabilityError::InvalidRequest(
                "target output level percent must be <= 100",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::vec;
    use edgerun_capabilities::CapabilityError;

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

    // ----- SpeakerSampleFormat -----

    #[test]
    fn sample_format_equality() {
        assert_eq!(SpeakerSampleFormat::PcmS16Le, SpeakerSampleFormat::PcmS16Le);
        assert_ne!(
            SpeakerSampleFormat::PcmS16Le,
            SpeakerSampleFormat::PcmFloat32Le
        );
    }

    #[test]
    fn sample_format_debug() {
        assert_eq!(format!("{:?}", SpeakerSampleFormat::PcmS16Le), "PcmS16Le");
        assert_eq!(format!("{:?}", SpeakerSampleFormat::PcmS24Le), "PcmS24Le");
        assert_eq!(
            format!("{:?}", SpeakerSampleFormat::PcmFloat32Le),
            "PcmFloat32Le"
        );
    }

    #[test]
    fn sample_format_copy() {
        let a = SpeakerSampleFormat::PcmS24Le;
        let b = a;
        assert_eq!(a, b);
    }

    // ----- SpeakerInfo -----

    #[test]
    fn speaker_info_equality() {
        let info = SpeakerInfo {
            provider: "alsa".into(),
            instance_id: "hw:0,0".into(),
            display_name: "Built-in Speaker".into(),
            default_sample_rate_hz: 48_000,
            channels: 2,
            supports_playback: true,
            supports_output_level_control: true,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn speaker_info_debug_format() {
        let info = SpeakerInfo {
            provider: "test".into(),
            instance_id: "spk0".into(),
            display_name: "Test Speaker".into(),
            default_sample_rate_hz: 44_100,
            channels: 1,
            supports_playback: true,
            supports_output_level_control: false,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("Test Speaker"));
        assert!(debug.contains("spk0"));
    }

    #[test]
    fn speaker_info_neq_different_values() {
        let a = SpeakerInfo {
            provider: "a".into(),
            instance_id: "0".into(),
            display_name: "A".into(),
            default_sample_rate_hz: 48_000,
            channels: 2,
            supports_playback: true,
            supports_output_level_control: false,
        };
        let b = SpeakerInfo {
            provider: "a".into(),
            instance_id: "1".into(),
            display_name: "A".into(),
            default_sample_rate_hz: 48_000,
            channels: 2,
            supports_playback: true,
            supports_output_level_control: false,
        };
        assert_ne!(a, b);
    }

    // ----- SpeakerOutputLevel -----

    #[test]
    fn output_level_equality() {
        let level = SpeakerOutputLevel {
            current_percent: 75,
            min_raw_value: 0,
            max_raw_value: 65536,
            muted: Some(false),
        };
        let cloned = level.clone();
        assert_eq!(level, cloned);
    }

    #[test]
    fn output_level_muted_none() {
        let level = SpeakerOutputLevel {
            current_percent: 50,
            min_raw_value: 0,
            max_raw_value: 100,
            muted: None,
        };
        assert!(level.muted.is_none());
    }

    #[test]
    fn output_level_muted_true() {
        let level = SpeakerOutputLevel {
            current_percent: 0,
            min_raw_value: 0,
            max_raw_value: 65536,
            muted: Some(true),
        };
        assert_eq!(level.current_percent, 0);
        assert_eq!(level.muted, Some(true));
    }

    // ----- AudioPlaybackRequest -----

    #[test]
    fn playback_request_equality() {
        let req = AudioPlaybackRequest {
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0, 1, 2, 3],
            software_gain_percent: Some(150),
            target_output_level_percent: Some(80),
        };
        let cloned = req.clone();
        assert_eq!(req, cloned);
    }

    #[test]
    fn playback_request_debug_format() {
        let req = AudioPlaybackRequest {
            duration_ms: 50,
            sample_rate_hz: 44_100,
            channels: 1,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let debug = format!("{:?}", req);
        assert!(debug.contains("50"));
        assert!(debug.contains("44100"));
    }

    #[test]
    fn playback_request_with_no_optional_fields() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 16_000,
            channels: 1,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0xff],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        assert!(req.software_gain_percent.is_none());
        assert!(req.target_output_level_percent.is_none());
    }

    // ----- AudioPlaybackResult -----

    #[test]
    fn playback_result_equality() {
        let result = AudioPlaybackResult {
            bytes_written: 1024,
            sample_rate_hz: 48_000,
            channels: 2,
            finished: true,
        };
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn playback_result_not_finished() {
        let result = AudioPlaybackResult {
            bytes_written: 512,
            sample_rate_hz: 48_000,
            channels: 2,
            finished: false,
        };
        assert!(!result.finished);
        assert_eq!(result.bytes_written, 512);
    }

    // ----- validate_audio_playback_request -----

    #[test]
    fn playback_request_rejects_zero_duration() {
        let req = AudioPlaybackRequest {
            duration_ms: 0,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let err = validate_audio_playback_request(&req).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("playback duration must be > 0")
        );
    }

    #[test]
    fn playback_request_rejects_zero_sample_rate() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 0,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let err = validate_audio_playback_request(&req).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("sample rate must be > 0")
        );
    }

    #[test]
    fn playback_request_rejects_zero_channels() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 0,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let err = validate_audio_playback_request(&req).unwrap_err();
        assert_eq!(err, CapabilityError::InvalidRequest("channels must be > 0"));
    }

    #[test]
    fn playback_request_rejects_empty_audio_bytes() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        let err = validate_audio_playback_request(&req).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("audio bytes must be non-empty")
        );
    }

    #[test]
    fn playback_request_rejects_gain_over_400() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: Some(401),
            target_output_level_percent: None,
        };
        let err = validate_audio_playback_request(&req).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("software gain percent must be <= 400")
        );
    }

    #[test]
    fn playback_request_rejects_target_level_over_100() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: Some(101),
        };
        let err = validate_audio_playback_request(&req).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("target output level percent must be <= 100")
        );
    }

    #[test]
    fn playback_request_valid_boundary_values() {
        // Gain exactly 400 is ok
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: Some(400),
            target_output_level_percent: Some(100),
        };
        assert!(validate_audio_playback_request(&req).is_ok());
    }

    #[test]
    fn playback_request_valid_minimum_values() {
        let req = AudioPlaybackRequest {
            duration_ms: 1,
            sample_rate_hz: 1,
            channels: 1,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: None,
        };
        assert!(validate_audio_playback_request(&req).is_ok());
    }

    #[test]
    fn playback_request_gain_zero_is_ok() {
        let req = AudioPlaybackRequest {
            duration_ms: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: vec![0],
            software_gain_percent: Some(0),
            target_output_level_percent: Some(0),
        };
        assert!(validate_audio_playback_request(&req).is_ok());
    }

    // ----- default_speaker_descriptor -----

    #[test]
    fn descriptor_has_query_and_render() {
        let descriptor = default_speaker_descriptor("test", "id");
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(descriptor
            .operations
            .contains(&(CapabilityOperation::Render as i32)));
    }

    #[test]
    fn descriptor_has_no_constraints() {
        let descriptor = default_speaker_descriptor("test", "id");
        assert!(descriptor.default_constraints.is_empty());
    }

    #[test]
    fn descriptor_uses_provider_and_instance() {
        let descriptor = default_speaker_descriptor("my-speaker", "spk-0");
        assert_eq!(descriptor.provider_name, "my-speaker");
        assert_eq!(descriptor.provider_instance_id, "spk-0");
    }

    // ----- SpeakerDevice trait default methods -----

    #[test]
    fn speaker_device_default_output_level_returns_none() {
        struct DummySpeaker;
        impl CapabilityProvider for DummySpeaker {
            fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
                default_speaker_descriptor("dummy", "0")
            }
        }
        impl SpeakerDevice for DummySpeaker {
            fn speaker_info(&self) -> Result<SpeakerInfo, CapabilityError> {
                Ok(SpeakerInfo {
                    provider: "dummy".into(),
                    instance_id: "0".into(),
                    display_name: "Dummy".into(),
                    default_sample_rate_hz: 48_000,
                    channels: 2,
                    supports_playback: true,
                    supports_output_level_control: false,
                })
            }
            fn play_audio(
                &self,
                _req: &AudioPlaybackRequest,
            ) -> Result<AudioPlaybackResult, CapabilityError> {
                Ok(AudioPlaybackResult {
                    bytes_written: 0,
                    sample_rate_hz: 0,
                    channels: 0,
                    finished: true,
                })
            }
        }
        let device = DummySpeaker;
        assert!(device.output_level().unwrap().is_none());
    }

    #[test]
    fn speaker_device_default_set_output_level_unsupported() {
        struct DummySpeaker;
        impl CapabilityProvider for DummySpeaker {
            fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
                default_speaker_descriptor("dummy", "0")
            }
        }
        impl SpeakerDevice for DummySpeaker {
            fn speaker_info(&self) -> Result<SpeakerInfo, CapabilityError> {
                Ok(SpeakerInfo {
                    provider: "dummy".into(),
                    instance_id: "0".into(),
                    display_name: "Dummy".into(),
                    default_sample_rate_hz: 48_000,
                    channels: 2,
                    supports_playback: true,
                    supports_output_level_control: false,
                })
            }
            fn play_audio(
                &self,
                _req: &AudioPlaybackRequest,
            ) -> Result<AudioPlaybackResult, CapabilityError> {
                Ok(AudioPlaybackResult {
                    bytes_written: 0,
                    sample_rate_hz: 0,
                    channels: 0,
                    finished: true,
                })
            }
        }
        let device = DummySpeaker;
        let err = device.set_output_level(50).unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }
}
