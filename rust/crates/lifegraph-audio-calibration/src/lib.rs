use lifegraph_alsa_microphone::{discover_alsa_pcms, AlsaMicrophoneBackend};
use lifegraph_alsa_speaker::discover_speakers;
use lifegraph_capabilities::CapabilityError;
use lifegraph_microphone::{AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat};
use lifegraph_speaker::{AudioPlaybackRequest, SpeakerDevice, SpeakerSampleFormat};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct AudioSweepStepResult {
    pub requested_level_percent: u8,
    pub applied_level_percent: Option<u8>,
    pub rms_dbfs: f32,
    pub peak_dbfs: f32,
    pub clipped_samples: usize,
    pub total_samples: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioSweepConfig {
    pub speaker_card: u32,
    pub speaker_device: u32,
    pub microphone_card: u32,
    pub microphone_device: u32,
    pub start_level_percent: u8,
    pub end_level_percent: u8,
    pub step_percent: u8,
    pub duration_ms: u32,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub tone_hz: f32,
    pub software_gain_percent: u16,
    pub lead_in_ms: u64,
}

pub fn synth_tone(duration_ms: u32, sample_rate_hz: u32, channels: u16, hz: f32) -> Vec<u8> {
    let frames = (u64::from(duration_ms) * u64::from(sample_rate_hz) / 1000) as usize;
    let mut out = Vec::with_capacity(frames * usize::from(channels) * 2);
    for i in 0..frames {
        let t = i as f32 / sample_rate_hz as f32;
        let sample = (t * hz * std::f32::consts::TAU).sin();
        let value = (sample * 0.25 * i16::MAX as f32) as i16;
        for _ in 0..channels {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

pub fn analyze_s16le_monoish(bytes: &[u8]) -> (f32, f32, usize, usize) {
    let mut sum_sq = 0.0f64;
    let mut peak = 0i32;
    let mut clipped = 0usize;
    let mut total = 0usize;
    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
        let abs = sample.unsigned_abs() as i32;
        peak = peak.max(abs);
        if abs >= i16::MAX as i32 - 8 {
            clipped += 1;
        }
        let norm = sample as f64 / i16::MAX as f64;
        sum_sq += norm * norm;
        total += 1;
    }
    if total == 0 {
        return (f32::NEG_INFINITY, f32::NEG_INFINITY, 0, 0);
    }
    let rms = (sum_sq / total as f64).sqrt() as f32;
    let peak_norm = peak as f32 / i16::MAX as f32;
    let rms_dbfs = if rms > 0.0 {
        20.0 * rms.log10()
    } else {
        f32::NEG_INFINITY
    };
    let peak_dbfs = if peak_norm > 0.0 {
        20.0 * peak_norm.log10()
    } else {
        f32::NEG_INFINITY
    };
    (rms_dbfs, peak_dbfs, clipped, total)
}

pub fn run_speaker_mic_sweep(
    config: &AudioSweepConfig,
) -> Result<Vec<AudioSweepStepResult>, CapabilityError> {
    if config.step_percent == 0 {
        return Err(CapabilityError::InvalidRequest(
            "audio sweep step percent must be > 0",
        ));
    }
    let speaker = discover_speakers()?
        .into_iter()
        .find(|d| d.card_index == config.speaker_card && d.device_index == config.speaker_device)
        .ok_or_else(|| CapabilityError::Provider("speaker backend not found".into()))?;
    let pcm = discover_alsa_pcms()?
        .into_iter()
        .find(|p| {
            p.card_index == config.microphone_card && p.device_index == config.microphone_device
        })
        .ok_or_else(|| CapabilityError::Provider("microphone backend not found".into()))?;

    let mut level = config.start_level_percent;
    let mut out = Vec::new();
    loop {
        let mut mic = AlsaMicrophoneBackend {
            proc_root: "/proc/asound".into(),
            pcm: pcm.clone(),
        };
        let speaker_clone = speaker.clone();
        let tone = synth_tone(
            config.duration_ms,
            config.sample_rate_hz,
            config.channels,
            config.tone_hz,
        );
        let lead_in_ms = config.lead_in_ms;
        let gain = config.software_gain_percent;
        let level_for_thread = level;
        let duration_ms = config.duration_ms;
        let sample_rate_hz = config.sample_rate_hz;
        let channels = config.channels;
        let playback = thread::spawn(move || {
            thread::sleep(Duration::from_millis(lead_in_ms));
            speaker_clone.play_audio(&AudioPlaybackRequest {
                duration_ms,
                sample_rate_hz,
                channels,
                format: SpeakerSampleFormat::PcmS16Le,
                audio_bytes: tone,
                software_gain_percent: Some(gain),
                target_output_level_percent: Some(level_for_thread),
            })
        });
        let capture = mic.capture_audio(&AudioCaptureRequest {
            duration_ms: config.duration_ms + config.lead_in_ms as u32,
            sample_rate_hz: config.sample_rate_hz,
            channels: config.channels,
            format: MicrophoneSampleFormat::PcmS16Le,
        })?;
        playback
            .join()
            .map_err(|_| CapabilityError::Provider("speaker playback thread panicked".into()))??;

        let (rms_dbfs, peak_dbfs, clipped_samples, total_samples) =
            analyze_s16le_monoish(&capture.bytes);
        let applied = speaker.output_level()?.map(|v| v.current_percent);
        out.push(AudioSweepStepResult {
            requested_level_percent: level,
            applied_level_percent: applied,
            rms_dbfs,
            peak_dbfs,
            clipped_samples,
            total_samples,
        });

        if level >= config.end_level_percent {
            break;
        }
        let next = level.saturating_add(config.step_percent);
        level = next.min(config.end_level_percent);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_s16le_reports_peak_and_rms() {
        let mut bytes = Vec::new();
        for sample in [1000i16, -1000i16, 0i16, 2000i16] {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let (rms_dbfs, peak_dbfs, clipped, total) = analyze_s16le_monoish(&bytes);
        assert!(rms_dbfs.is_finite());
        assert!(peak_dbfs.is_finite());
        assert_eq!(clipped, 0);
        assert_eq!(total, 4);
    }

    #[test]
    fn synth_tone_generates_bytes() {
        let bytes = synth_tone(20, 48_000, 2, 440.0);
        assert!(!bytes.is_empty());
        assert_eq!(bytes.len() % 4, 0);
    }
}
