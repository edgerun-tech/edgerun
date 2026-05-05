use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ord;
use core::convert::{From, Into};
use core::iter::IntoIterator;
use core::option::Option::{self, Some};
use core::result::Result::{self, Err, Ok};

#[cfg(unix)]
use edgerun_alsa_microphone::{discover_alsa_pcms, AlsaMicrophoneBackend};
#[cfg(unix)]
use edgerun_alsa_speaker::discover_speakers;
use edgerun_capabilities::CapabilityError;
use edgerun_microphone::{AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat};
use edgerun_speaker::{AudioPlaybackRequest, SpeakerDevice, SpeakerSampleFormat};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
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
        let sample = sin_approx(t * hz * core::f32::consts::TAU);
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
    let rms = sqrt_approx(sum_sq / total as f64) as f32;
    let peak_norm = peak as f32 / i16::MAX as f32;
    let rms_dbfs = if rms > 0.0 {
        20.0 * log10_approx(rms)
    } else {
        f32::NEG_INFINITY
    };
    let peak_dbfs = if peak_norm > 0.0 {
        20.0 * log10_approx(peak_norm)
    } else {
        f32::NEG_INFINITY
    };
    (rms_dbfs, peak_dbfs, clipped, total)
}

fn sin_approx(mut x: f32) -> f32 {
    const TAU: f32 = core::f32::consts::TAU;
    const PI: f32 = core::f32::consts::PI;
    x %= TAU;
    if x > PI {
        x -= TAU;
    } else if x < -PI {
        x += TAU;
    }
    let x2 = x * x;
    x * (1.0 - x2 / 6.0 + (x2 * x2) / 120.0 - (x2 * x2 * x2) / 5040.0)
}

fn sqrt_approx(value: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    let mut x = value;
    for _ in 0..12 {
        x = 0.5 * (x + value / x);
    }
    x
}

fn log10_approx(value: f32) -> f32 {
    if value <= 0.0 {
        return f32::NEG_INFINITY;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127;
    let mantissa_bits = (bits & 0x7f_ffff) | 0x80_0000;
    let mantissa = mantissa_bits as f32 / 0x80_0000 as f32;
    let f = mantissa - 1.0;
    let ln_mantissa = f - f * f * 0.5 + f * f * f / 3.0;
    (exponent as f32 + ln_mantissa / core::f32::consts::LN_2) * core::f32::consts::LOG10_2
}

#[cfg(unix)]
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
        let device_path = format!("/dev/snd/pcmC{}D{}c", pcm.card_index, pcm.device_index);
        let mut mic = AlsaMicrophoneBackend {
            pcm: pcm.clone(),
            device_path,
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

#[cfg(not(unix))]
pub fn run_speaker_mic_sweep(
    config: &AudioSweepConfig,
) -> Result<Vec<AudioSweepStepResult>, CapabilityError> {
    let _ = config;
    Err(CapabilityError::Unsupported(
        "audio sweep requires host ALSA playback/capture",
    ))
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

    // ----- AudioSweepStepResult -----

    #[test]
    fn sweep_step_result_equality() {
        let a = AudioSweepStepResult {
            requested_level_percent: 50,
            applied_level_percent: Some(48),
            rms_dbfs: -20.0,
            peak_dbfs: -10.0,
            clipped_samples: 0,
            total_samples: 1000,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn sweep_step_result_debug_format() {
        let result = AudioSweepStepResult {
            requested_level_percent: 75,
            applied_level_percent: None,
            rms_dbfs: -30.0,
            peak_dbfs: -15.0,
            clipped_samples: 5,
            total_samples: 2000,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("75"));
        assert!(debug.contains("-30"));
    }

    #[test]
    fn sweep_step_result_neq_different_values() {
        let a = AudioSweepStepResult {
            requested_level_percent: 50,
            applied_level_percent: None,
            rms_dbfs: -20.0,
            peak_dbfs: -10.0,
            clipped_samples: 0,
            total_samples: 1000,
        };
        let b = AudioSweepStepResult {
            requested_level_percent: 60,
            applied_level_percent: None,
            rms_dbfs: -20.0,
            peak_dbfs: -10.0,
            clipped_samples: 0,
            total_samples: 1000,
        };
        assert_ne!(a, b);
    }

    // ----- AudioSweepConfig -----

    #[test]
    fn sweep_config_equality() {
        let config = AudioSweepConfig {
            speaker_card: 0,
            speaker_device: 0,
            microphone_card: 0,
            microphone_device: 0,
            start_level_percent: 10,
            end_level_percent: 100,
            step_percent: 10,
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 2,
            tone_hz: 1000.0,
            software_gain_percent: 100,
            lead_in_ms: 50,
        };
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn sweep_config_debug_format() {
        let config = AudioSweepConfig {
            speaker_card: 0,
            speaker_device: 0,
            microphone_card: 0,
            microphone_device: 0,
            start_level_percent: 0,
            end_level_percent: 100,
            step_percent: 20,
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 1,
            tone_hz: 440.0,
            software_gain_percent: 100,
            lead_in_ms: 0,
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("440"));
        assert!(debug.contains("20"));
    }

    // ----- analyze_s16le_monoish -----

    #[test]
    fn analyze_s16le_empty_bytes() {
        let (rms_dbfs, peak_dbfs, clipped, total) = analyze_s16le_monoish(&[]);
        assert_eq!(rms_dbfs, f32::NEG_INFINITY);
        assert_eq!(peak_dbfs, f32::NEG_INFINITY);
        assert_eq!(clipped, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn analyze_s16le_silence() {
        let bytes = [0u8, 0u8, 0u8, 0u8]; // two zero samples
        let (rms_dbfs, peak_dbfs, clipped, total) = analyze_s16le_monoish(&bytes);
        assert_eq!(rms_dbfs, f32::NEG_INFINITY);
        assert_eq!(peak_dbfs, f32::NEG_INFINITY);
        assert_eq!(clipped, 0);
        assert_eq!(total, 2);
    }

    #[test]
    fn analyze_s16le_full_scale_clipping() {
        let sample = i16::MAX;
        let bytes = sample.to_le_bytes();
        let (_rms_dbfs, peak_dbfs, clipped, total) = analyze_s16le_monoish(&bytes);
        assert_eq!(clipped, 1);
        assert_eq!(total, 1);
        assert!(peak_dbfs > -1.0); // near 0 dBFS
    }

    #[test]
    fn analyze_s16le_negative_values() {
        let sample: i16 = -16384; // -50% amplitude
        let bytes = sample.to_le_bytes();
        let (rms_dbfs, peak_dbfs, clipped, total) = analyze_s16le_monoish(&bytes);
        assert_eq!(total, 1);
        assert_eq!(clipped, 0);
        assert!(rms_dbfs.is_finite());
        assert!(peak_dbfs.is_finite());
        // Should be around -6 dBFS for 50% amplitude
        assert!(peak_dbfs > -7.0 && peak_dbfs < -5.0);
    }

    #[test]
    fn analyze_s16le_rms_lower_than_peak() {
        // Mix of quiet and loud samples
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&100i16.to_le_bytes()); // quiet
        bytes.extend_from_slice(&16000i16.to_le_bytes()); // loud
        let (rms_dbfs, peak_dbfs, _clipped, total) = analyze_s16le_monoish(&bytes);
        assert_eq!(total, 2);
        assert!(rms_dbfs < peak_dbfs); // RMS is always <= peak
    }

    #[test]
    fn analyze_s16le_single_sample() {
        let sample = 16384i16; // 50% of i16::MAX
        let bytes = sample.to_le_bytes();
        let (rms_dbfs, peak_dbfs, clipped, total) = analyze_s16le_monoish(&bytes);
        assert_eq!(total, 1);
        assert_eq!(clipped, 0);
        assert_eq!(rms_dbfs, peak_dbfs); // For single sample, RMS == peak
    }

    // ----- synth_tone -----

    #[test]
    fn synth_tone_duration_zero() {
        let bytes = synth_tone(0, 48_000, 2, 440.0);
        assert!(bytes.is_empty());
    }

    #[test]
    fn synth_tone_mono_channel() {
        let bytes = synth_tone(10, 48_000, 1, 440.0);
        assert_eq!(bytes.len() % 2, 0); // mono = 2 bytes per sample
    }

    #[test]
    fn synth_tone_different_frequencies() {
        let low = synth_tone(10, 48_000, 1, 100.0);
        let high = synth_tone(10, 48_000, 1, 4000.0);
        // Both should produce non-empty output
        assert!(!low.is_empty());
        assert!(!high.is_empty());
        assert_eq!(low.len(), high.len()); // Same duration = same size
    }

    #[test]
    fn synth_tone_frame_count_matches_params() {
        let duration_ms = 25;
        let sample_rate = 48_000;
        let channels: u16 = 2;
        let bytes = synth_tone(duration_ms, sample_rate, channels, 440.0);
        let expected_frames = (duration_ms as u64 * sample_rate as u64) / 1000;
        let expected_bytes = expected_frames as usize * channels as usize * 2;
        assert_eq!(bytes.len(), expected_bytes);
    }

    // ----- run_speaker_mic_sweep -----

    #[test]
    fn sweep_rejects_zero_step_percent() {
        let config = AudioSweepConfig {
            speaker_card: 0,
            speaker_device: 0,
            microphone_card: 0,
            microphone_device: 0,
            start_level_percent: 10,
            end_level_percent: 100,
            step_percent: 0,
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 2,
            tone_hz: 1000.0,
            software_gain_percent: 100,
            lead_in_ms: 50,
        };
        let err = run_speaker_mic_sweep(&config).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("audio sweep step percent must be > 0")
        );
    }

    #[test]
    fn sweep_config_clone() {
        let config = AudioSweepConfig {
            speaker_card: 0,
            speaker_device: 0,
            microphone_card: 0,
            microphone_device: 0,
            start_level_percent: 0,
            end_level_percent: 50,
            step_percent: 5,
            duration_ms: 50,
            sample_rate_hz: 16_000,
            channels: 1,
            tone_hz: 800.0,
            software_gain_percent: 150,
            lead_in_ms: 100,
        };
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }
}
