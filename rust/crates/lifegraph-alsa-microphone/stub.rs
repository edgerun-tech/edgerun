use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_microphone::{
    default_microphone_descriptor, validate_audio_capture_request, AudioCapture,
    AudioCaptureRequest, MicrophoneDevice, MicrophoneInfo, MicrophoneSampleFormat,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsaPcmInfo {
    pub card_index: u32,
    pub device_index: u32,
    pub capture: bool,
    pub playback: bool,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsaMicrophoneBackend {
    pub proc_root: String,
    pub pcm: AlsaPcmInfo,
}

pub fn discover_alsa_pcms() -> Result<Vec<AlsaPcmInfo>, CapabilityError> {
    Ok(Vec::new())
}

impl CapabilityProvider for AlsaMicrophoneBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_microphone_descriptor(
            "alsa-microphone",
            &format!(
                "card{}-device{}",
                self.pcm.card_index, self.pcm.device_index
            ),
        )
    }
}

impl MicrophoneDevice for AlsaMicrophoneBackend {
    fn microphone_info(&self) -> Result<MicrophoneInfo, CapabilityError> {
        Ok(MicrophoneInfo {
            provider: "alsa-microphone".into(),
            device_name: self.pcm.name.clone(),
            instance_id: format!(
                "card{}-device{}",
                self.pcm.card_index, self.pcm.device_index
            ),
            channels: 2,
            sample_rate_hz: 48_000,
            format: MicrophoneSampleFormat::PcmS16Le,
            hardware_aec: false,
            hardware_noise_suppression: false,
        })
    }

    fn capture_audio(
        &mut self,
        request: &AudioCaptureRequest,
    ) -> Result<AudioCapture, CapabilityError> {
        validate_audio_capture_request(request)?;
        let bytes_per_sample = match request.format {
            MicrophoneSampleFormat::PcmS16Le => 2,
            MicrophoneSampleFormat::PcmS24Le => 3,
            MicrophoneSampleFormat::PcmS32Le | MicrophoneSampleFormat::Float32Le => 4,
            MicrophoneSampleFormat::Other(_) => {
                return Err(CapabilityError::Unsupported(
                    "stub microphone only supports PCM capture formats",
                ))
            }
        };
        let frames = (u64::from(request.duration_ms) * u64::from(request.sample_rate_hz)) / 1000;
        let total_bytes = frames as usize * usize::from(request.channels) * bytes_per_sample;
        let started_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Ok(AudioCapture {
            sample_rate_hz: request.sample_rate_hz,
            channels: request.channels,
            format: request.format,
            bytes: vec![0; total_bytes],
            started_at_unix_ms,
        })
    }
}
