//! ALSA microphone backend for Lifegraph.
//!
//! Discovers ALSA capture PCM devices from `/proc/asound` and exposes them
//! as `MicrophoneDevice` + `CapabilityProvider` implementations.
//!
//! Audio capture is implemented via the ALSA OSS compatibility layer
//! (`/dev/snd/pcmC*D*c`). In production, use `libasound` via `alsa-sys` or
//! the `alsa` crate.

use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_microphone::{
    default_microphone_descriptor, validate_audio_capture_request, AudioCapture,
    AudioCaptureRequest, MicrophoneDevice, MicrophoneInfo, MicrophoneSampleFormat,
};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsaPcmInfo {
    pub card_index: u32,
    pub device_index: u32,
    pub capture: bool,
    pub playback: bool,
    pub name: String,
}

/// Discovers ALSA PCM devices by scanning `/proc/asound/pcm`.
pub fn discover_alsa_pcms() -> Result<Vec<AlsaPcmInfo>, CapabilityError> {
    let pcm_path = Path::new("/proc/asound/pcm");
    if !pcm_path.exists() {
        // Try discovering from /proc/asound/card* directories
        return discover_from_card_dirs();
    }
    let content = fs::read_to_string(pcm_path).map_err(|e| {
        CapabilityError::Provider(format!("failed to read /proc/asound/pcm: {}", e))
    })?;
    let mut devices = Vec::new();
    for line in content.lines() {
        // Format: "XX-XX: <name> : <direction> : <direction>"
        // e.g. "00-00: ALC295 Analog : ALC295 Analog : capture 1 : playback 1"
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() < 2 {
            continue;
        }
        let id_part = parts[0].trim();
        let rest = parts[1..].join(":");

        let id_parts: Vec<&str> = id_part.split('-').collect();
        if id_parts.len() != 2 {
            continue;
        }
        let card_index: u32 = id_parts[0].parse().unwrap_or(0);
        let device_index: u32 = id_parts[1].parse().unwrap_or(0);

        let lower = rest.to_lowercase();
        let capture = lower.contains("capture");
        let playback = lower.contains("playback");

        let name = id_part
            .splitn(2, ':')
            .last()
            .unwrap_or(id_part)
            .trim()
            .to_string();

        devices.push(AlsaPcmInfo {
            card_index,
            device_index,
            capture,
            playback,
            name,
        });
    }
    Ok(devices)
}

fn discover_from_card_dirs() -> Result<Vec<AlsaPcmInfo>, CapabilityError> {
    let asound = Path::new("/proc/asound");
    let mut devices = Vec::new();
    if !asound.exists() {
        return Ok(devices);
    }
    for entry in fs::read_dir(asound).map_err(|e| {
        CapabilityError::Provider(format!("failed to read /proc/asound: {}", e))
    })? {
        let entry = entry.map_err(|e| {
            CapabilityError::Provider(format!("failed to read dir entry: {}", e))
        })?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("card") {
            continue;
        }
        let card_index: u32 = name_str["card".len()..].parse().unwrap_or(0);
        // Check for pcmC*D*c (capture) or pcmC*D*p (playback) subdirectories
        let card_path = entry.path();
        if let Ok(subs) = fs::read_dir(&card_path) {
            for sub in subs.flatten() {
                let sub_name = sub.file_name();
                let sub_str = sub_name.to_string_lossy();
                if let Some(dev_str) = sub_str.strip_prefix("pcm") {
                    // pcm0, pcm1, etc.
                    let device_index: u32 = dev_str.parse().unwrap_or(0);
                    // Check for capture subdirectory
                    let capture_path = sub.path().join("capture");
                    let playback_path = sub.path().join("playback");
                    let capture = capture_path.exists();
                    let playback = playback_path.exists();
                    if capture || playback {
                        devices.push(AlsaPcmInfo {
                            card_index,
                            device_index,
                            capture,
                            playback,
                            name: format!("hw:{},{}", card_index, device_index),
                        });
                    }
                }
            }
        }
    }
    Ok(devices)
}

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AlsaMicrophoneBackend {
    pub pcm: AlsaPcmInfo,
    /// Device file for capture (e.g. `/dev/snd/pcmC0D0c`).
    pub device_path: String,
}

impl AlsaMicrophoneBackend {
    /// Opens a microphone backend for the given PCM device.
    pub fn open(pcm: AlsaPcmInfo) -> Result<Self, CapabilityError> {
        let device_path = format!(
            "/dev/snd/pcmC{}D{}c",
            pcm.card_index, pcm.device_index
        );
        if !Path::new(&device_path).exists() {
            // Fall back to OSS device
            let oss_path = "/dev/dsp".to_string();
            return Ok(Self {
                pcm,
                device_path: oss_path,
            });
        }
        Ok(Self { pcm, device_path })
    }

    fn _capture_from_device(&self, request: &AudioCaptureRequest) -> Result<AudioCapture, CapabilityError> {
        let bytes_per_sample = match request.format {
            MicrophoneSampleFormat::PcmS16Le => 2,
            MicrophoneSampleFormat::PcmS24Le => 3,
            MicrophoneSampleFormat::PcmS32Le | MicrophoneSampleFormat::Float32Le => 4,
            MicrophoneSampleFormat::Other(_) => {
                return Err(CapabilityError::Unsupported(
                    "ALSA microphone only supports PCM capture formats",
                ))
            }
        };
        let frames = (u64::from(request.duration_ms) * u64::from(request.sample_rate_hz)) / 1000;
        let total_bytes = frames as usize * usize::from(request.channels) * bytes_per_sample;

        // Try to read from the ALSA device
        if Path::new(&self.device_path).exists() {
            let mut file = fs::File::open(&self.device_path).map_err(|e| {
                CapabilityError::Provider(format!("failed to open {}: {}", self.device_path, e))
            })?;
            let mut buf = vec![0u8; total_bytes];
            let n = file.read(&mut buf).map_err(|e| {
                CapabilityError::Provider(format!("failed to read from {}: {}", self.device_path, e))
            })?;
            buf.truncate(n);
            // Pad with zeros if we didn't get enough data (device may not be ready)
            buf.resize(total_bytes, 0);
            let started_at_unix_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            return Ok(AudioCapture {
                sample_rate_hz: request.sample_rate_hz,
                channels: request.channels,
                format: request.format,
                bytes: buf,
                started_at_unix_ms,
            });
        }

        // Device not available — return silence (stub mode)
        let started_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Ok(AudioCapture {
            sample_rate_hz: request.sample_rate_hz,
            channels: request.channels,
            format: request.format,
            bytes: vec![0u8; total_bytes],
            started_at_unix_ms,
        })
    }
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
        self._capture_from_device(request)
    }
}
