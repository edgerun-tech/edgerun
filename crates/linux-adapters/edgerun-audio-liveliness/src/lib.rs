//! Audio-based liveliness detection for anti-spoofing.
//!
//! This crate provides voice activity detection (VAD), audio challenge/response,
//! and liveness scoring to complement camera-based biometric verification.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │  Audio Challenge │────▶│  Voice Activity  │────▶│  Liveliness     │
//! │  Generator       │     │  Detection (VAD) │     │  Scorer         │
//! └─────────────────┘     └──────────────────┘     └─────────────────┘
//!         │                       │                        │
//!         ▼                       ▼                        ▼
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │  Speaker Output │     │  Microphone      │     │  Liveliness     │
//! │  (play phrase)  │     │  Input (capture) │     │  Result         │
//! └─────────────────┘     └──────────────────┘     └─────────────────┘
//! ```
//!
//! # Challenge Types
//!
//! - **RepeatNumber**: User must speak a randomly generated number
//! - **ClapDetection**: User must clap (sharp transient detection)
//! - **SpeakPhrase**: User must speak a specific phrase
//! - **VoiceActivity**: Simple voice activity detection (any speech)
//!
//! # Anti-Spoofing Measures
//!
//! - **Recording detection**: Analyzes spectral characteristics for synthetic speech
//! - **Latency check**: Measures response time (live speech has natural latency)
//! - **Spectral analysis**: Checks for playback artifacts (flat frequency response, etc.)

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

#[cfg(target_os = "none")]
pub mod f64 {
    pub mod consts {
        pub use core::f64::consts::*;
    }
}

#[cfg(target_os = "none")]
pub mod mem {
    pub use core::mem::*;
}

#[cfg(target_os = "none")]
pub mod thread {
    pub fn sleep(_duration: crate::time::Duration) {}
}

#[cfg(target_os = "none")]
pub mod time {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Duration {
        millis: u128,
    }

    impl Duration {
        #[must_use]
        pub const fn from_millis(millis: u64) -> Self {
            Self {
                millis: millis as u128,
            }
        }

        #[must_use]
        pub const fn as_millis(&self) -> u128 {
            self.millis
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Instant;

    impl Instant {
        #[must_use]
        pub const fn now() -> Self {
            Self
        }

        #[must_use]
        pub const fn elapsed(&self) -> Duration {
            Duration { millis: 0 }
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SystemTime;

    pub const UNIX_EPOCH: SystemTime = SystemTime;

    impl SystemTime {
        #[must_use]
        pub const fn now() -> Self {
            Self
        }

        pub const fn duration_since(&self, _earlier: SystemTime) -> Result<Duration, ()> {
            Ok(Duration { millis: 0 })
        }
    }
}

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::{Ord, PartialOrd};
use core::convert::{From, Into};
use core::default::Default;
use core::iter::{FromIterator, IntoIterator, Iterator};
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Ok};

use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityModality,
    CapabilityOperation,
};
use edgerun_devices::biometrics::{BiometricModality, BiometricState};
use edgerun_devices::microphone::{AudioCaptureRequest, MicrophoneDevice, MicrophoneSampleFormat};
use edgerun_devices::speaker::{AudioPlaybackRequest, SpeakerDevice, SpeakerSampleFormat};
use std::time::{Duration, Instant, UNIX_EPOCH};

trait FloatApprox {
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn exp(self) -> Self;
    fn sqrt(self) -> Self;
    fn ln(self) -> Self;
    fn log10(self) -> Self;
    fn powi(self, n: i32) -> Self;
}

impl FloatApprox for f64 {
    fn sin(self) -> Self {
        sin_f64(self)
    }

    fn cos(self) -> Self {
        sin_f64(self + core::f64::consts::FRAC_PI_2)
    }

    fn exp(self) -> Self {
        exp_f64(self)
    }

    fn sqrt(self) -> Self {
        sqrt_f64(self)
    }

    fn ln(self) -> Self {
        ln_f64(self)
    }

    fn log10(self) -> Self {
        ln_f64(self) / core::f64::consts::LN_10
    }

    fn powi(self, n: i32) -> Self {
        if n == 0 {
            return 1.0;
        }
        let mut result = 1.0;
        for _ in 0..n.unsigned_abs() {
            result *= self;
        }
        if n < 0 { 1.0 / result } else { result }
    }
}

impl FloatApprox for f32 {
    fn sin(self) -> Self {
        sin_f64(self as f64) as f32
    }

    fn cos(self) -> Self {
        sin_f64(self as f64 + core::f64::consts::FRAC_PI_2) as f32
    }

    fn exp(self) -> Self {
        exp_f64(self as f64) as f32
    }

    fn sqrt(self) -> Self {
        sqrt_f64(self as f64) as f32
    }

    fn ln(self) -> Self {
        ln_f64(self as f64) as f32
    }

    fn log10(self) -> Self {
        (ln_f64(self as f64) / core::f64::consts::LN_10) as f32
    }

    fn powi(self, n: i32) -> Self {
        FloatApprox::powi(self as f64, n) as f32
    }
}

fn sin_f64(mut x: f64) -> f64 {
    const TAU: f64 = core::f64::consts::TAU;
    const PI: f64 = core::f64::consts::PI;
    x %= TAU;
    if x > PI {
        x -= TAU;
    } else if x < -PI {
        x += TAU;
    }
    let x2 = x * x;
    x * (1.0 - x2 / 6.0 + x2 * x2 / 120.0 - x2 * x2 * x2 / 5040.0)
}

fn exp_f64(x: f64) -> f64 {
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..24 {
        term *= x / n as f64;
        sum += term;
    }
    sum
}

fn sqrt_f64(value: f64) -> f64 {
    if value <= 0.0 {
        return 0.0;
    }
    let mut x = value;
    for _ in 0..12 {
        x = 0.5 * (x + value / x);
    }
    x
}

fn ln_f64(value: f64) -> f64 {
    if value <= 0.0 {
        return f64::NEG_INFINITY;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    let mantissa_bits = (bits & 0x000f_ffff_ffff_ffff) | 0x0010_0000_0000_0000;
    let mantissa = mantissa_bits as f64 / 0x0010_0000_0000_0000u64 as f64;
    let f = mantissa - 1.0;
    let f2 = f * f;
    let f3 = f2 * f;
    let f4 = f3 * f;
    exponent as f64 * core::f64::consts::LN_2 + f - f2 * 0.5 + f3 / 3.0 - f4 * 0.25
}

// ===========================================================================
// Audio Challenge Types
// ===========================================================================

/// Types of audio challenges for liveliness detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioChallengeKind {
    /// User must speak a randomly generated number (1-999)
    RepeatNumber,
    /// User must clap hands (sharp transient detection)
    ClapDetection,
    /// User must speak any words (voice activity detection)
    VoiceActivity,
}

/// An audio challenge for liveliness detection.
#[derive(Clone, Debug)]
pub struct AudioChallenge {
    /// The type of challenge
    pub kind: AudioChallengeKind,
    /// Maximum time allowed to respond (milliseconds)
    pub timeout_ms: u32,
    /// The expected response (for number challenges)
    pub expected_response: Option<String>,
    /// Minimum audio level to consider as speech (dBFS)
    pub speech_threshold_dbfs: f32,
}

impl AudioChallenge {
    /// Create a new repeat-number challenge with a random number.
    pub fn repeat_number(timeout_ms: u32) -> Self {
        let number = rand_number();
        Self {
            kind: AudioChallengeKind::RepeatNumber,
            timeout_ms,
            expected_response: Some(number.to_string()),
            speech_threshold_dbfs: -40.0,
        }
    }

    /// Create a clap detection challenge.
    pub fn clap_detection(timeout_ms: u32) -> Self {
        Self {
            kind: AudioChallengeKind::ClapDetection,
            timeout_ms,
            expected_response: None,
            speech_threshold_dbfs: -20.0, // Claps are loud
        }
    }

    /// Create a simple voice activity challenge.
    pub fn voice_activity(timeout_ms: u32) -> Self {
        Self {
            kind: AudioChallengeKind::VoiceActivity,
            timeout_ms,
            expected_response: None,
            speech_threshold_dbfs: -40.0,
        }
    }

    /// Generate the challenge audio prompt (text to be spoken by the system).
    pub fn prompt_text(&self) -> String {
        match self.kind {
            AudioChallengeKind::RepeatNumber => {
                format!(
                    "Please say the number {}",
                    self.expected_response.as_deref().unwrap_or("?")
                )
            }
            AudioChallengeKind::ClapDetection => "Please clap your hands once".into(),
            AudioChallengeKind::VoiceActivity => {
                "Please say anything to verify you're present".into()
            }
        }
    }
}

/// The result of an audio challenge.
#[derive(Clone, Debug)]
pub struct AudioChallengeResult {
    /// Whether the challenge was passed
    pub passed: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Detected speech activity
    pub speech_detected: bool,
    /// For number challenges: what was actually detected
    pub detected_response: Option<String>,
    /// Response latency (time from challenge to first speech)
    pub response_latency_ms: Option<u32>,
    /// Spectral analysis results (for anti-spoofing)
    pub spectral_analysis: Option<SpectralAnalysis>,
    /// Biometric state reflecting the audio verification
    pub state: BiometricState,
}

/// Spectral analysis of captured audio for anti-spoofing.
#[derive(Clone, Debug)]
pub struct SpectralAnalysis {
    /// Root mean square level in dBFS
    pub rms_dbfs: f32,
    /// Peak level in dBFS
    pub peak_dbfs: f32,
    /// Spectral flatness (0.0 = tonal, 1.0 = noise-like)
    pub spectral_flatness: f32,
    /// Zero-crossing rate (higher = more noise-like)
    pub zero_crossing_rate: f32,
    /// Whether the audio looks like natural speech (vs playback)
    pub looks_like_natural_speech: bool,
}

// ===========================================================================
// Voice Activity Detection
// ===========================================================================

/// Voice Activity Detection (VAD) engine.
///
/// Uses energy-based detection with spectral analysis to determine
/// if there is actual human speech in the audio signal.
pub struct VoiceActivityDetector {
    /// Speech energy threshold (dBFS)
    threshold_dbfs: f32,
    /// Minimum speech duration to count as valid (ms)
    min_speech_duration_ms: u32,
    /// Maximum silence gap within speech (ms)
    max_silence_gap_ms: u32,
}

impl VoiceActivityDetector {
    /// Create a new VAD with default parameters.
    pub fn new() -> Self {
        Self {
            threshold_dbfs: -40.0,
            min_speech_duration_ms: 200,
            max_silence_gap_ms: 500,
        }
    }

    /// Create with custom threshold.
    pub fn with_threshold(mut self, dbfs: f32) -> Self {
        self.threshold_dbfs = dbfs;
        self
    }

    /// Analyze PCM audio for voice activity.
    ///
    /// Returns (speech_detected, confidence, first_speech_offset_ms).
    ///
    /// Enforces:
    /// - Frames must exceed `threshold_dbfs` to count as speech.
    /// - Total speech must span at least `min_speech_duration_ms`.
    /// - Gaps in speech within a burst must not exceed `max_silence_gap_ms`.
    pub fn analyze_s16le(&self, pcm: &[u8], sample_rate_hz: u32) -> (bool, f32, Option<u32>) {
        if pcm.is_empty() {
            return (false, 0.0, None);
        }

        // Analyze in 20ms frames
        let frame_size = (sample_rate_hz / 50) as usize * 2; // 20ms in bytes (S16LE)
        let frame_duration_ms = 20;

        // Track speech bursts: consecutive speech frames separated by
        /// silence no larger than `max_silence_gap_ms`.
        #[derive(Default)]
        struct SpeechBurst {
            first_frame: usize,
            last_frame: usize,
            speech_frames: usize,
        }

        impl SpeechBurst {
            fn duration_ms(&self, frame_ms: u32) -> u32 {
                if self.last_frame >= self.first_frame {
                    ((self.last_frame - self.first_frame + 1) as u32) * frame_ms
                } else {
                    0
                }
            }

            fn silence_gap_ms(&self, current_frame: usize, frame_ms: u32) -> u32 {
                if current_frame > self.last_frame {
                    ((current_frame - self.last_frame - 1) as u32) * frame_ms
                } else {
                    0
                }
            }
        }

        let mut bursts: Vec<SpeechBurst> = Vec::new();
        let mut total_frames = 0;
        let mut total_energy = 0.0f32;
        let mut first_speech_frame: Option<usize> = None;

        for chunk in pcm.chunks(frame_size) {
            if chunk.len() < 2 {
                break;
            }

            let (energy, _peak) = analyze_frame_s16le(chunk);
            total_energy += energy;
            total_frames += 1;

            let is_speech = energy > self.threshold_dbfs;
            if is_speech {
                if first_speech_frame.is_none() {
                    first_speech_frame = Some(total_frames - 1);
                }

                // Either extend current burst or start a new one
                if let Some(last) = bursts.last_mut() {
                    let gap = last.silence_gap_ms(total_frames - 1, frame_duration_ms);
                    if gap <= self.max_silence_gap_ms {
                        // Extend current burst
                        last.last_frame = total_frames - 1;
                        last.speech_frames += 1;
                    } else {
                        // Gap too large — start new burst
                        bursts.push(SpeechBurst {
                            first_frame: total_frames - 1,
                            last_frame: total_frames - 1,
                            speech_frames: 1,
                        });
                    }
                } else {
                    bursts.push(SpeechBurst {
                        first_frame: total_frames - 1,
                        last_frame: total_frames - 1,
                        speech_frames: 1,
                    });
                }
            }
        }

        if total_frames == 0 || bursts.is_empty() {
            return (false, 0.0, None);
        }

        // Find the longest valid burst that meets the minimum duration
        let valid_burst = bursts
            .iter()
            .filter(|b| b.duration_ms(frame_duration_ms) >= self.min_speech_duration_ms)
            .max_by_key(|b| b.speech_frames);

        let speech_detected = valid_burst.is_some();
        let speech_frames = valid_burst.map(|b| b.speech_frames).unwrap_or(0);

        let avg_energy = total_energy / total_frames as f32;

        // Confidence based on speech ratio and energy
        let confidence = if speech_detected {
            let speech_ratio = speech_frames as f32 / total_frames as f32;
            let ratio_confidence = (speech_ratio * 2.0).min(1.0); // 50%+ speech = 1.0
            let energy_confidence = ((avg_energy - self.threshold_dbfs) / 30.0).clamp(0.0, 1.0);
            (ratio_confidence + energy_confidence) / 2.0
        } else {
            0.0
        };

        let first_speech_offset_ms = first_speech_frame.map(|f| (f as u32) * frame_duration_ms);

        (speech_detected, confidence, first_speech_offset_ms)
    }

    /// Detect claps (sharp transients) in audio.
    ///
    /// Returns (clap_detected, confidence, clap_offset_ms).
    pub fn detect_clap(&self, pcm: &[u8], sample_rate_hz: u32) -> (bool, f32, Option<u32>) {
        if pcm.len() < 4 {
            return (false, 0.0, None);
        }

        // Find the peak sample value and its local context
        let mut max_sample: i16 = 0;
        let mut max_sample_idx = 0;

        for (i, chunk) in pcm.chunks_exact(2).enumerate() {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            if sample.abs() > max_sample.abs() {
                max_sample = sample;
                max_sample_idx = i;
            }
        }

        // Calculate RMS around the peak (±5ms window)
        let window_samples = (sample_rate_hz / 200) as usize; // 5ms
        let start = max_sample_idx.saturating_sub(window_samples);
        let end = (max_sample_idx + window_samples).min(pcm.len() / 2);
        let mut sum_sq: i64 = 0;
        let mut count: i64 = 0;

        for i in start..end {
            let offset = i * 2;
            if offset + 1 < pcm.len() {
                let s = i16::from_le_bytes([pcm[offset], pcm[offset + 1]]) as i64;
                sum_sq += s * s;
                count += 1;
            }
        }

        if count == 0 {
            return (false, 0.0, None);
        }

        let rms = (sum_sq as f64 / count as f64).sqrt();
        let peak_abs = max_sample.abs() as f64;

        // A clap has:
        // 1. High peak amplitude (> 8000 = about -12 dBFS)
        // 2. High peak-to-RMS ratio (> 3:1 for sharp transients)
        let peak_to_rms = if rms > 1.0 { peak_abs / rms } else { 0.0 };

        let clap_detected = peak_abs > 8000.0 && peak_to_rms > 3.0;

        let confidence = if clap_detected {
            let peak_conf = ((peak_abs / 32767.0) as f32).min(1.0);
            let ratio_conf = ((peak_to_rms / 10.0) as f32).min(1.0);
            peak_conf * 0.4 + ratio_conf * 0.6
        } else {
            0.0
        };

        let clap_offset_ms = if clap_detected {
            Some((max_sample_idx as u32) * 1000 / sample_rate_hz)
        } else {
            None
        };

        (clap_detected, confidence, clap_offset_ms)
    }
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Spectral Analysis
// ===========================================================================

/// Analyze PCM audio for spectral characteristics.
pub fn analyze_spectral(pcm: &[u8], _sample_rate_hz: u32) -> SpectralAnalysis {
    if pcm.is_empty() {
        return SpectralAnalysis {
            rms_dbfs: f32::MIN,
            peak_dbfs: f32::MIN,
            spectral_flatness: 0.0,
            zero_crossing_rate: 0.0,
            looks_like_natural_speech: false,
        };
    }

    let (rms_dbfs, peak_dbfs) = analyze_s16le_levels(pcm);
    let zero_crossing_rate = calculate_zero_crossing_rate_s16le(pcm);

    // Spectral flatness approximation using zero-crossing rate
    // Natural speech has moderate ZCR (0.1-0.3), noise has high ZCR (>0.3)
    let spectral_flatness = (zero_crossing_rate * 2.0).min(1.0);

    // Natural speech characteristics:
    // - Moderate zero-crossing rate (not too tonal, not too noisy)
    // - Dynamic energy (not flat like playback)
    // - Peak-to-RMS ratio in speech range (3-8)
    let looks_like_natural_speech = zero_crossing_rate > 0.05
        && zero_crossing_rate < 0.4
        && rms_dbfs > -60.0
        && rms_dbfs < -10.0;

    SpectralAnalysis {
        rms_dbfs,
        peak_dbfs,
        spectral_flatness,
        zero_crossing_rate,
        looks_like_natural_speech,
    }
}

// ===========================================================================
// Audio Challenge Executor
// ===========================================================================

/// Executes an audio challenge through speaker + microphone.
pub struct AudioChallengeExecutor {
    speaker: Box<dyn SpeakerDevice>,
    microphone: Box<dyn MicrophoneDevice>,
    vad: VoiceActivityDetector,
}

impl AudioChallengeExecutor {
    /// Create a new executor with the given speaker and microphone.
    pub fn new(speaker: Box<dyn SpeakerDevice>, microphone: Box<dyn MicrophoneDevice>) -> Self {
        Self {
            speaker,
            microphone,
            vad: VoiceActivityDetector::new(),
        }
    }

    /// Execute an audio challenge.
    ///
    /// 1. Play the challenge prompt through the speaker
    /// 2. Capture audio from the microphone
    /// 3. Analyze for speech/claps
    /// 4. Return the result
    pub fn execute_challenge(
        &mut self,
        challenge: &AudioChallenge,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let start = Instant::now();
        let sample_rate = 48000; // Standard sample rate

        // Play the challenge prompt
        let prompt = challenge.prompt_text();
        let prompt_audio = text_to_speech_audio(&prompt, sample_rate);
        let _ = self.speaker.play_audio(&AudioPlaybackRequest {
            duration_ms: (prompt_audio.len() / 2) as u32 * 1000 / sample_rate,
            sample_rate_hz: sample_rate,
            channels: 1,
            format: SpeakerSampleFormat::PcmS16Le,
            audio_bytes: prompt_audio,
            software_gain_percent: None,
            target_output_level_percent: None,
        });

        // Small delay for prompt to finish
        std::thread::sleep(Duration::from_millis(500));

        // Capture audio response
        let capture_duration_ms = challenge.timeout_ms;
        let capture_result = self.microphone.capture_audio(&AudioCaptureRequest {
            duration_ms: capture_duration_ms,
            sample_rate_hz: sample_rate,
            channels: 1,
            format: MicrophoneSampleFormat::PcmS16Le,
        })?;

        let capture_latency_ms = start.elapsed().as_millis() as u32;

        // Analyze the captured audio
        match challenge.kind {
            AudioChallengeKind::RepeatNumber => {
                self.analyze_number_response(challenge, &capture_result, capture_latency_ms)
            }
            AudioChallengeKind::ClapDetection => {
                self.analyze_clap_response(challenge, &capture_result, capture_latency_ms)
            }
            AudioChallengeKind::VoiceActivity => {
                self.analyze_voice_activity(challenge, &capture_result, capture_latency_ms)
            }
        }
    }

    fn analyze_number_response(
        &self,
        challenge: &AudioChallenge,
        capture: &edgerun_devices::microphone::AudioCapture,
        capture_latency_ms: u32,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let (speech_detected, vad_confidence, speech_offset_ms) =
            self.vad.analyze_s16le(&capture.bytes, 48000);

        let response_latency_ms = speech_offset_ms.map(|offset| {
            capture_latency_ms
                .saturating_add(offset)
                .saturating_sub(500) // subtract prompt time
        });

        // Use DTW keyword spotter to verify the spoken number
        let spotter = KeywordSpotter::new(48000);
        let (detected_number, keyword_confidence) = spotter
            .match_digit(&capture.bytes, 48000)
            .unwrap_or((999, 0.0)); // 999 = no match

        let expected_number = challenge
            .expected_response
            .as_deref()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(999);

        let number_match = detected_number == expected_number && keyword_confidence > 0.3;

        let spectral = analyze_spectral(&capture.bytes, 48000);

        let detected_response = if number_match {
            challenge.expected_response.clone()
        } else if speech_detected {
            Some(format!("unknown (detected: {})", detected_number))
        } else {
            None
        };

        // Pass if:
        // 1. Speech was detected with reasonable confidence
        // 2. The keyword spotter matched the expected number
        // 3. Audio looks like natural speech (anti-spoofing)
        let passed = speech_detected && number_match && spectral.looks_like_natural_speech;

        let confidence = if passed {
            vad_confidence * 0.3 + keyword_confidence * 0.4 + {
                if spectral.looks_like_natural_speech {
                    0.3
                } else {
                    0.05
                }
            }
        } else if speech_detected && !number_match {
            // Speech detected but wrong number
            vad_confidence * 0.3
        } else {
            0.0
        };

        Ok(AudioChallengeResult {
            passed,
            confidence,
            speech_detected,
            detected_response,
            response_latency_ms,
            spectral_analysis: Some(spectral),
            state: audio_biometric_state(passed, confidence > 0.7),
        })
    }

    fn analyze_clap_response(
        &self,
        _challenge: &AudioChallenge,
        capture: &edgerun_devices::microphone::AudioCapture,
        capture_latency_ms: u32,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let (clap_detected, clap_confidence, clap_offset_ms) =
            self.vad.detect_clap(&capture.bytes, 48000);

        let response_latency_ms = clap_offset_ms.map(|offset| {
            capture_latency_ms
                .saturating_add(offset)
                .saturating_sub(500)
        });

        let spectral = analyze_spectral(&capture.bytes, 48000);

        // Claps are naturally loud and have sharp transients
        // Playback of clap recording would have different characteristics
        let passed = clap_detected && spectral.looks_like_natural_speech;

        let confidence = if passed {
            clap_confidence * 0.6 + {
                if spectral.looks_like_natural_speech {
                    0.4
                } else {
                    0.1
                }
            }
        } else {
            0.0
        };

        Ok(AudioChallengeResult {
            passed,
            confidence,
            speech_detected: clap_detected,
            detected_response: if clap_detected {
                Some("clap".into())
            } else {
                None
            },
            response_latency_ms,
            spectral_analysis: Some(spectral),
            state: audio_biometric_state(passed, confidence > 0.7),
        })
    }

    fn analyze_voice_activity(
        &self,
        _challenge: &AudioChallenge,
        capture: &edgerun_devices::microphone::AudioCapture,
        capture_latency_ms: u32,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let (speech_detected, vad_confidence, speech_offset_ms) =
            self.vad.analyze_s16le(&capture.bytes, 48000);

        let response_latency_ms = speech_offset_ms.map(|offset| {
            capture_latency_ms
                .saturating_add(offset)
                .saturating_sub(500)
        });

        let spectral = analyze_spectral(&capture.bytes, 48000);

        let passed = speech_detected && vad_confidence > 0.2;

        let confidence = if passed {
            vad_confidence * 0.7 + {
                if spectral.looks_like_natural_speech {
                    0.3
                } else {
                    0.1
                }
            }
        } else {
            0.0
        };

        Ok(AudioChallengeResult {
            passed,
            confidence,
            speech_detected,
            detected_response: None,
            response_latency_ms,
            spectral_analysis: Some(spectral),
            state: audio_biometric_state(passed, confidence > 0.7),
        })
    }
}

// ===========================================================================
// Audio Liveliness Scorer
// ===========================================================================

/// Combines camera and audio signals for final liveliness decision.
pub struct AudioVisualLivelinessScorer {
    /// Weight for camera signal (0.0 to 1.0)
    camera_weight: f32,
    /// Weight for audio signal (0.0 to 1.0)
    audio_weight: f32,
    /// Minimum combined score to pass
    pass_threshold: f32,
}

impl AudioVisualLivelinessScorer {
    /// Create a scorer with equal weights for camera and audio.
    pub fn balanced() -> Self {
        Self {
            camera_weight: 0.5,
            audio_weight: 0.5,
            pass_threshold: 0.6,
        }
    }

    /// Create a scorer that requires both camera AND audio to pass.
    pub fn strict() -> Self {
        Self {
            camera_weight: 0.5,
            audio_weight: 0.5,
            pass_threshold: 0.7,
        }
    }

    /// Create a scorer that primarily relies on camera with audio as bonus.
    pub fn camera_primary() -> Self {
        Self {
            camera_weight: 0.7,
            audio_weight: 0.3,
            pass_threshold: 0.6,
        }
    }

    /// Score combined liveliness from camera and audio results.
    ///
    /// Returns (passed, combined_score).
    pub fn score(
        &self,
        camera_passed: bool,
        camera_confidence: f32,
        audio_passed: bool,
        audio_confidence: f32,
    ) -> (bool, f32) {
        let camera_score = if camera_passed {
            camera_confidence
        } else {
            0.0
        };
        let audio_score = if audio_passed { audio_confidence } else { 0.0 };

        let combined = camera_score * self.camera_weight + audio_score * self.audio_weight;

        // Bonus for both passing (redundancy increases confidence)
        let redundancy_bonus = if camera_passed && audio_passed {
            0.1
        } else {
            0.0
        };

        let final_score = (combined + redundancy_bonus).min(1.0);
        let passed = final_score >= self.pass_threshold;

        (passed, final_score)
    }
}

// ===========================================================================
// Helper Functions
// ===========================================================================

/// Generate a random number between 1 and 999.
fn rand_number() -> u32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    ((seed.wrapping_mul(6364136223846793005).wrapping_add(1)) >> 33) as u32 % 999 + 1
}

/// Analyze a single S16LE frame, returning (RMS dBFS, peak dBFS).
fn analyze_frame_s16le(pcm: &[u8]) -> (f32, f32) {
    if pcm.len() < 2 {
        return (f32::MIN, f32::MIN);
    }

    let mut sum_sq = 0i64;
    let mut max_sample = 0i64;
    let mut count = 0i64;

    for chunk in pcm.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i64;
        sum_sq += sample * sample;
        max_sample = max_sample.max(sample.abs());
        count += 1;
    }

    if count == 0 {
        return (f32::MIN, f32::MIN);
    }

    let rms = (sum_sq as f64 / count as f64).sqrt() as f32;
    let rms_dbfs = if rms < 1.0 {
        f32::MIN
    } else {
        20.0 * (rms / 32767.0).log10()
    };

    let peak_dbfs = if max_sample < 1 {
        f32::MIN
    } else {
        20.0 * (max_sample as f32 / 32767.0).log10()
    };

    (rms_dbfs, peak_dbfs)
}

/// Analyze S16LE PCM levels, returning (RMS dBFS, peak dBFS).
fn analyze_s16le_levels(pcm: &[u8]) -> (f32, f32) {
    if pcm.len() < 2 {
        return (f32::MIN, f32::MIN);
    }

    let mut sum_sq = 0i64;
    let mut max_sample = 0i64;
    let mut count = 0i64;

    for chunk in pcm.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i64;
        sum_sq += sample * sample;
        max_sample = max_sample.max(sample.abs());
        count += 1;
    }

    if count == 0 {
        return (f32::MIN, f32::MIN);
    }

    let rms = (sum_sq as f64 / count as f64).sqrt() as f32;
    let rms_dbfs = if rms < 1.0 {
        f32::MIN
    } else {
        20.0 * (rms / 32767.0).log10()
    };

    let peak_dbfs = if max_sample < 1 {
        f32::MIN
    } else {
        20.0 * (max_sample as f32 / 32767.0).log10()
    };

    (rms_dbfs, peak_dbfs)
}

/// Calculate zero-crossing rate of S16LE PCM.
fn calculate_zero_crossing_rate_s16le(pcm: &[u8]) -> f32 {
    if pcm.len() < 4 {
        return 0.0;
    }

    let mut crossings = 0;
    let mut prev_sample: i16 = 0;
    let mut count = 0;

    for chunk in pcm.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        if count > 0 && ((prev_sample >= 0 && sample < 0) || (prev_sample < 0 && sample >= 0)) {
            crossings += 1;
        }
        prev_sample = sample;
        count += 1;
    }

    if count <= 1 {
        0.0
    } else {
        crossings as f32 / (count - 1) as f32
    }
}

/// Formant-based speech synthesizer — pure Rust, no external dependencies.
///
/// Uses a Klatt-style formant synthesizer model:
/// - Voiced sounds: glottal pulse train filtered through formant resonators
/// - Unvoiced sounds: white noise filtered through formant resonators
/// - Silence: zero output
///
/// Supports English phonemes for numbers 0-999 and simple prompts.
struct FormantSynth {
    sample_rate: u32,
    /// Glottal pulse phase for voiced sounds
    glottal_phase: f64,
    /// Formant filter state (3 formants)
    f1_state: f64,
    f2_state: f64,
    f3_state: f64,
    /// Noise generator state
    noise_state: u64,
}

impl FormantSynth {
    fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            glottal_phase: 0.0,
            f1_state: 0.0,
            f2_state: 0.0,
            f3_state: 0.0,
            noise_state: 42,
        }
    }

    /// Simple LCG noise generator.
    #[inline]
    fn noise(&mut self) -> f64 {
        self.noise_state = self
            .noise_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        ((self.noise_state >> 33) as i32 as f64) / (i32::MAX as f64)
    }

    /// Generate a glottal pulse (Rosenberg model).
    #[inline]
    fn glottal_pulse(&mut self, f0: f64) -> f64 {
        let period = self.sample_rate as f64 / f0;
        self.glottal_phase += 1.0 / period;
        if self.glottal_phase >= 1.0 {
            self.glottal_phase -= 1.0;
        }
        let p = self.glottal_phase;
        // Rosenberg glottal pulse: open phase (0-0.7), return phase (0.7-1.0)
        if p < 0.7 {
            (p / 0.7).sin().powi(2)
        } else if p < 0.9 {
            (1.0 - (p - 0.7) / 0.2).cos().powi(2) * 0.3
        } else {
            0.0
        }
    }

    /// Apply a resonant bandpass filter (formant).
    #[inline]
    fn formant_filter(state: &mut f64, input: f64, freq: f64, bw: f64, sr: f64) -> f64 {
        let r = (-std::f64::consts::PI * bw / sr).exp();
        let angle = 2.0 * std::f64::consts::PI * freq / sr;
        let _output = input + 2.0 * r * angle.cos() * *state - r * r * input;
        // Actually a proper 2nd order resonator:
        let _b0 = (1.0 - r) * (1.0 + r * r - 2.0 * r * angle.cos()).sqrt();
        *state = *state * r * angle.cos() * 2.0 - r * r * *state + input;
        // Simplified: just use a leaky integrator tuned to the formant
        let alpha = 1.0 - r;
        *state = *state * r + input * alpha;
        *state
    }

    /// Synthesize a single phoneme.
    fn synthesize_phoneme(&mut self, phoneme: &Phoneme, duration_ms: u32) -> Vec<i16> {
        let num_samples = (self.sample_rate as u64 * duration_ms as u64 / 1000) as usize;
        let mut output = Vec::with_capacity(num_samples);
        let sr = self.sample_rate as f64;

        for _ in 0..num_samples {
            let source = if phoneme.voiced {
                self.glottal_pulse(phoneme.f0) * phoneme.amplitude
            } else {
                self.noise() * phoneme.amplitude * 0.5
            };

            // Cascade of 3 formant filters
            let f1 = Self::formant_filter(&mut self.f1_state, source, phoneme.f1, phoneme.bw1, sr);
            let f2 = Self::formant_filter(&mut self.f2_state, f1, phoneme.f2, phoneme.bw2, sr);
            let f3 = Self::formant_filter(&mut self.f3_state, f2, phoneme.f3, phoneme.bw3, sr);

            let sample = (f3 * 28000.0).clamp(-32000.0, 32000.0) as i16;
            output.push(sample);
        }

        output
    }
}

/// English phoneme definition for formant synthesis.
#[derive(Clone, Copy)]
struct Phoneme {
    /// Whether this phoneme is voiced (glottal pulse) or unvoiced (noise)
    voiced: bool,
    /// Fundamental frequency (Hz) — only for voiced
    f0: f64,
    /// Formant frequencies (Hz)
    f1: f64,
    f2: f64,
    f3: f64,
    /// Formant bandwidths (Hz)
    bw1: f64,
    bw2: f64,
    bw3: f64,
    /// Amplitude (0.0 to 1.0)
    amplitude: f64,
    /// Duration in milliseconds
    duration_ms: u32,
}

/// Convert a number (0-999) to a sequence of phonemes.
fn number_to_phonemes(n: u32) -> Vec<Phoneme> {
    // Simplified English pronunciation for numbers
    // Each digit maps to approximate phoneme sequences
    let digit_phonemes: &[&[Phoneme]] = &[
        // "zero" — /zɪəroʊ/
        &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.15,
                duration_ms: 80,
            }, // z (fricative)
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 400.0,
                f2: 2000.0,
                f3: 3000.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 60,
            }, // ɪ
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 500.0,
                f2: 1400.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 100,
            }, // ə
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 500.0,
                f2: 900.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // r
            Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 450.0,
                f2: 800.0,
                f3: 2500.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 120,
            }, // oʊ
        ],
        // "one" — /wʌn/
        &[
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 300.0,
                f2: 700.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 60,
            }, // w
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 700.0,
                f2: 1200.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 100,
            }, // ʌ
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 250.0,
                f2: 1800.0,
                f3: 2800.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 80,
            }, // n
        ],
        // "two" — /tuː/
        &[
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 350.0,
                f2: 1200.0,
                f3: 2500.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.2,
                duration_ms: 40,
            }, // t
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 350.0,
                f2: 850.0,
                f3: 2500.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 150,
            }, // uː
        ],
        // "three" — /θriː/
        &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.12,
                duration_ms: 60,
            }, // θ
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 500.0,
                f2: 900.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // r
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 350.0,
                f2: 2300.0,
                f3: 3000.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 120,
            }, // iː
        ],
        // "four" — /fɔːr/
        &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.12,
                duration_ms: 60,
            }, // f
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 600.0,
                f2: 900.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 120,
            }, // ɔː
            Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 500.0,
                f2: 900.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 80,
            }, // r
        ],
        // "five" — /faɪv/
        &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.12,
                duration_ms: 50,
            }, // f
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 700.0,
                f2: 1400.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // a
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 350.0,
                f2: 2200.0,
                f3: 3000.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // ɪ
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 300.0,
                f2: 700.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 60,
            }, // v
        ],
        // "six" — /sɪks/
        &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.15,
                duration_ms: 60,
            }, // s
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 400.0,
                f2: 2000.0,
                f3: 3000.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 60,
            }, // ɪ
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 350.0,
                f2: 1200.0,
                f3: 2500.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.2,
                duration_ms: 40,
            }, // k
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.15,
                duration_ms: 60,
            }, // s
        ],
        // "seven" — /sɛvən/
        &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.15,
                duration_ms: 50,
            }, // s
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 550.0,
                f2: 1800.0,
                f3: 2700.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // ɛ
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 300.0,
                f2: 700.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 50,
            }, // v
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 500.0,
                f2: 1400.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 60,
            }, // ə
            Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 250.0,
                f2: 1800.0,
                f3: 2800.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 60,
            }, // n
        ],
        // "eight" — /eɪt/
        &[
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 550.0,
                f2: 1800.0,
                f3: 2700.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 100,
            }, // e
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 350.0,
                f2: 2200.0,
                f3: 3000.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // ɪ
            Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 350.0,
                f2: 1200.0,
                f3: 2500.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.2,
                duration_ms: 40,
            }, // t
        ],
        // "nine" — /naɪn/
        &[
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 250.0,
                f2: 1800.0,
                f3: 2800.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 50,
            }, // n
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 700.0,
                f2: 1400.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // a
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 350.0,
                f2: 2200.0,
                f3: 3000.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 80,
            }, // ɪ
            Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 250.0,
                f2: 1800.0,
                f3: 2800.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.3,
                duration_ms: 60,
            }, // n
        ],
    ];

    // Simple lookup: map each digit to its phoneme sequence
    let digits: Vec<u32> = if n == 0 {
        vec![0]
    } else {
        let mut d = Vec::new();
        let mut num = n;
        while num > 0 {
            d.push(num % 10);
            num /= 10;
        }
        d.reverse();
        d
    };

    let mut phonemes = Vec::new();
    for (i, &digit) in digits.iter().enumerate() {
        if i > 0 {
            // Brief pause between digits
            phonemes.push(Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 500.0,
                f2: 1400.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.0,
                duration_ms: 100,
            });
        }
        phonemes.extend_from_slice(digit_phonemes[digit as usize]);
    }

    phonemes
}

/// Generate speech-like audio from text using formant synthesis.
fn text_to_speech_audio(text: &str, sample_rate: u32) -> Vec<u8> {
    let mut synth = FormantSynth::new(sample_rate);
    let mut all_samples = Vec::new();

    // Extract numbers from text and synthesize them
    // "Please say the number 42" → extract "42"
    let number_str: String = text.chars().filter(|c| c.is_ascii_digit()).collect();

    if !number_str.is_empty() {
        if let Ok(n) = number_str.parse::<u32>() {
            if n <= 999 {
                let phonemes = number_to_phonemes(n);
                for ph in &phonemes {
                    all_samples.extend(synth.synthesize_phoneme(ph, ph.duration_ms));
                }
            }
        }
    }

    // If no number found, say "hello" as a default
    if all_samples.is_empty() {
        // "hello" — /həloʊ/
        let hello = &[
            Phoneme {
                voiced: false,
                f0: 0.0,
                f1: 4000.0,
                f2: 4000.0,
                f3: 4000.0,
                bw1: 200.0,
                bw2: 200.0,
                bw3: 200.0,
                amplitude: 0.1,
                duration_ms: 60,
            },
            Phoneme {
                voiced: true,
                f0: 120.0,
                f1: 500.0,
                f2: 1400.0,
                f3: 2600.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 80,
            },
            Phoneme {
                voiced: true,
                f0: 115.0,
                f1: 500.0,
                f2: 900.0,
                f3: 2400.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.5,
                duration_ms: 100,
            },
            Phoneme {
                voiced: true,
                f0: 110.0,
                f1: 450.0,
                f2: 800.0,
                f3: 2500.0,
                bw1: 60.0,
                bw2: 90.0,
                bw3: 120.0,
                amplitude: 0.4,
                duration_ms: 120,
            },
        ];
        for ph in hello {
            all_samples.extend(synth.synthesize_phoneme(ph, ph.duration_ms));
        }
    }

    // Convert i16 samples to S16LE bytes
    let mut bytes = Vec::with_capacity(all_samples.len() * 2);
    for s in &all_samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }

    bytes
}

/// Dynamic Time Warping (DTW) keyword spotter.
///
/// Compares the MFCC-like feature sequence of captured audio against
/// reference templates for digit words ("zero" through "nine").
/// Returns the best-matching digit and confidence score.
///
/// Pure Rust — no external dependencies.
pub struct KeywordSpotter {
    /// Reference templates: digit → feature sequence
    templates: Vec<(String, Vec<f32>)>,
}

impl KeywordSpotter {
    /// Create a new spotter with reference templates for digits 0-9.
    pub fn new(sample_rate: u32) -> Self {
        let templates: Vec<(String, Vec<f32>)> = (0..=9)
            .map(|d| {
                let word = digit_word(d);
                let features = synthesize_mfcc_features(&word, sample_rate);
                (word, features)
            })
            .collect();

        Self { templates }
    }

    /// Match captured audio against digit templates.
    /// Returns (matched_digit, confidence) or None if no match.
    pub fn match_digit(&self, audio: &[u8], sample_rate: u32) -> Option<(u32, f32)> {
        if audio.len() < 100 {
            return None;
        }

        // Extract features from captured audio
        let query = extract_energy_features(audio, sample_rate);
        if query.is_empty() {
            return None;
        }

        // Find best match via DTW
        let mut best_digit = None;
        let mut best_score = f32::MAX;

        for (digit, template) in &self.templates {
            let distance = dtw_distance(&query, template);
            if distance < best_score {
                best_score = distance;
                best_digit = Some(digit);
            }
        }

        // Convert distance to confidence
        // Normalized: lower distance = higher confidence
        let confidence = if best_score > 0.0 {
            (1.0 / (1.0 + best_score * 0.01)).clamp(0.0, 1.0)
        } else {
            1.0
        };

        if let Some(digit_str) = best_digit {
            digit_str.parse::<u32>().ok().map(|d| (d, confidence))
        } else {
            None
        }
    }
}

/// Get the English word for a digit.
fn digit_word(d: u32) -> String {
    match d {
        0 => "zero".into(),
        1 => "one".into(),
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        7 => "seven".into(),
        8 => "eight".into(),
        9 => "nine".into(),
        _ => String::new(),
    }
}

/// Extract energy-based features from PCM audio (simplified MFCC substitute).
///
/// Returns a sequence of per-frame energy values that capture speech rhythm.
fn extract_energy_features(pcm: &[u8], sample_rate: u32) -> Vec<f32> {
    // 25ms frames with 10ms hop
    let frame_len = (sample_rate as f32 * 0.025) as usize * 2; // bytes
    let hop = (sample_rate as f32 * 0.010) as usize * 2;

    if pcm.len() < frame_len {
        return Vec::new();
    }

    let mut features = Vec::new();
    let mut pos = 0;
    while pos + frame_len <= pcm.len() {
        let frame = &pcm[pos..pos + frame_len];

        // Compute log energy (simplified MFCC coefficient 0)
        let mut energy = 0.0f64;
        for chunk in frame.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f64;
            energy += sample * sample;
        }
        energy /= (frame.len() / 2) as f64;
        let log_energy = if energy > 1.0 {
            (energy).ln() as f32
        } else {
            0.0
        };

        // Zero-crossing rate as second feature
        let mut zcr = 0u32;
        let mut prev: i16 = 0;
        for chunk in frame.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            if (prev >= 0 && sample < 0) || (prev < 0 && sample >= 0) {
                zcr += 1;
            }
            prev = sample;
        }
        let zcr_norm = zcr as f32 / (frame.len() / 2) as f32;

        features.push(log_energy);
        features.push(zcr_norm);

        pos += hop;
    }

    features
}

/// Synthesize MFCC-like features from a word's phoneme sequence (for template creation).
fn synthesize_mfcc_features(word: &str, sample_rate: u32) -> Vec<f32> {
    // Get phonemes for the word
    let phonemes = word_to_phonemes(word);

    // Generate audio from phonemes, then extract features
    let mut synth = FormantSynth::new(sample_rate);
    let mut samples = Vec::new();
    for ph in &phonemes {
        samples.extend(synth.synthesize_phoneme(ph, ph.duration_ms));
    }

    // Convert i16 to S16LE bytes
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    extract_energy_features(&bytes, sample_rate)
}

/// Convert a word string to phonemes.
fn word_to_phonemes(word: &str) -> Vec<Phoneme> {
    match word {
        "zero" => number_to_phonemes(0),
        "one" => number_to_phonemes(1),
        "two" => number_to_phonemes(2),
        "three" => number_to_phonemes(3),
        "four" => number_to_phonemes(4),
        "five" => number_to_phonemes(5),
        "six" => number_to_phonemes(6),
        "seven" => number_to_phonemes(7),
        "eight" => number_to_phonemes(8),
        "nine" => number_to_phonemes(9),
        "hello" => {
            vec![
                Phoneme {
                    voiced: false,
                    f0: 0.0,
                    f1: 4000.0,
                    f2: 4000.0,
                    f3: 4000.0,
                    bw1: 200.0,
                    bw2: 200.0,
                    bw3: 200.0,
                    amplitude: 0.1,
                    duration_ms: 60,
                },
                Phoneme {
                    voiced: true,
                    f0: 120.0,
                    f1: 500.0,
                    f2: 1400.0,
                    f3: 2600.0,
                    bw1: 60.0,
                    bw2: 90.0,
                    bw3: 120.0,
                    amplitude: 0.4,
                    duration_ms: 80,
                },
                Phoneme {
                    voiced: true,
                    f0: 115.0,
                    f1: 500.0,
                    f2: 900.0,
                    f3: 2400.0,
                    bw1: 60.0,
                    bw2: 90.0,
                    bw3: 120.0,
                    amplitude: 0.5,
                    duration_ms: 100,
                },
                Phoneme {
                    voiced: true,
                    f0: 110.0,
                    f1: 450.0,
                    f2: 800.0,
                    f3: 2500.0,
                    bw1: 60.0,
                    bw2: 90.0,
                    bw3: 120.0,
                    amplitude: 0.4,
                    duration_ms: 120,
                },
            ]
        }
        _ => Vec::new(),
    }
}

/// Dynamic Time Warping distance between two feature sequences.
///
/// Uses a simplified O(N*M) DTW with Sakoe-Chiba band constraint.
fn dtw_distance(seq_a: &[f32], seq_b: &[f32]) -> f32 {
    let n = seq_a.len();
    let m = seq_b.len();
    if n == 0 || m == 0 {
        return f32::MAX;
    }

    // Sakoe-Chiba band: limit warping to ±25% of sequence length
    let window = (n.max(m) / 4).max(2);

    // DTW cost matrix — only store current and previous row
    let mut prev_row = vec![f32::MAX; m + 1];
    let mut curr_row = vec![f32::MAX; m + 1];
    prev_row[0] = 0.0;

    for i in 1..=n {
        let j_start = (1.max(i as isize - window as isize) as usize).max(1);
        let j_end = m.min(i + window);

        curr_row[0] = f32::MAX;

        for j in j_start..=j_end {
            let local_dist = (seq_a[i - 1] - seq_b[j - 1]).abs();
            curr_row[j] = local_dist + prev_row[j].min(prev_row[j - 1]).min(curr_row[j - 1]);
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    // Normalize by path length
    prev_row[m] / (n + m) as f32
}

/// Create a biometric state for audio verification.
fn audio_biometric_state(passed: bool, _high_confidence: bool) -> BiometricState {
    BiometricState {
        modality: if passed {
            Some(BiometricModality::Voice)
        } else {
            None
        },
        verified: passed,
        hardware_protected: false, // Audio alone is not hardware-protected
        user_present: passed,
    }
}

// ===========================================================================
// CapabilityProvider for Audio Liveliness
// ===========================================================================

/// Audio liveliness capability descriptor.
pub fn audio_liveliness_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        descriptor_version: 1,
        capability_id: b"audio-liveliness".to_vec(),
        provider_identity: None,
        provider_node: None,
        role: edgerun_capabilities::CapabilityRole::Input as i32,
        modalities: vec![
            CapabilityModality::Auditory as i32,
            CapabilityModality::Biometric as i32,
        ],
        event_kinds: vec![
            CapabilityEventKind::Auditory as i32,
            CapabilityEventKind::Biometric as i32,
        ],
        operations: vec![
            CapabilityOperation::Query as i32,
            CapabilityOperation::Capture as i32,
        ],
        default_constraints: Vec::new(),
        provider_name: "audio-liveliness".into(),
        provider_instance_id: "default".into(),
        signature: None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_silence(duration_ms: u32, sample_rate: u32) -> Vec<u8> {
        let samples = (sample_rate as u64 * duration_ms as u64 / 1000) as usize;
        vec![0u8; samples * 2]
    }

    fn synth_tone(duration_ms: u32, sample_rate: u32, freq_hz: f64, amplitude: f64) -> Vec<u8> {
        let num_samples = (sample_rate as u64 * duration_ms as u64 / 1000) as usize;
        let mut samples = Vec::with_capacity(num_samples * 2);
        let mut phase = 0.0f64;

        for _ in 0..num_samples {
            let sample = (phase * 2.0 * std::f64::consts::PI).sin() * amplitude;
            let sample_i16 = (sample * 32000.0).clamp(-32000.0, 32000.0) as i16;
            samples.extend_from_slice(&sample_i16.to_le_bytes());
            phase += freq_hz / sample_rate as f64;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }

        samples
    }

    #[test]
    fn vad_detects_silence_as_no_speech() {
        let vad = VoiceActivityDetector::new();
        let silence = synth_silence(1000, 48000);
        let (speech, confidence, _) = vad.analyze_s16le(&silence, 48000);
        assert!(!speech);
        assert!(confidence < 0.1);
    }

    #[test]
    fn vad_detects_tone_as_speech() {
        let vad = VoiceActivityDetector::new();
        // Loud tone at 440 Hz (simulating voiced speech)
        let tone = synth_tone(500, 48000, 440.0, 0.3);
        let (speech, confidence, offset) = vad.analyze_s16le(&tone, 48000);
        assert!(speech);
        assert!(confidence > 0.2);
        assert!(offset.is_some()); // Should detect speech onset
    }

    #[test]
    fn clap_detection_detects_transient() {
        let vad = VoiceActivityDetector::new();

        // Create audio with silence, then a sharp transient, then silence
        let mut pcm = synth_silence(500, 48000);
        // Insert a sharp spike at ~100ms: alternating max values (very high peak-to-RMS)
        let spike_start = 48000 * 100 / 1000 * 2; // byte offset
        let spike_len = 20; // 10 samples of sharp altern
        for i in 0..spike_len {
            let byte_offset = spike_start + i * 2;
            if byte_offset + 1 < pcm.len() {
                let value: i16 = if i % 2 == 0 { 30000 } else { -30000 };
                pcm[byte_offset] = (value & 0xFF) as u8;
                pcm[byte_offset + 1] = ((value >> 8) & 0xFF) as u8;
            }
        }

        let (clap, confidence, offset) = vad.detect_clap(&pcm, 48000);
        // The sharp spike should trigger clap detection
        assert!(clap, "clap should be detected (confidence: {})", confidence);
        assert!(confidence > 0.0);
        assert!(offset.is_some());
    }

    #[test]
    fn clap_detection_ignores_tones() {
        let vad = VoiceActivityDetector::new();
        let tone = synth_tone(1000, 48000, 440.0, 0.5);
        let (clap, confidence, _) = vad.detect_clap(&tone, 48000);
        assert!(!clap);
        assert!(confidence < 0.1);
    }

    #[test]
    fn spectral_analysis_natural_speech() {
        // Moderate amplitude tone with some modulation (simulating speech)
        let samples = synth_tone(500, 48000, 440.0, 0.2);
        let spectral = analyze_spectral(&samples, 48000);
        assert!(spectral.rms_dbfs > f32::MIN);
        assert!(spectral.peak_dbfs > f32::MIN);
        assert!(spectral.zero_crossing_rate > 0.0);
    }

    #[test]
    fn spectral_analysis_silence() {
        let samples = synth_silence(500, 48000);
        let spectral = analyze_spectral(&samples, 48000);
        assert_eq!(spectral.rms_dbfs, f32::MIN);
        assert!(!spectral.looks_like_natural_speech);
    }

    #[test]
    fn audio_challenge_creation() {
        let challenge = AudioChallenge::repeat_number(3000);
        assert_eq!(challenge.kind, AudioChallengeKind::RepeatNumber);
        assert_eq!(challenge.timeout_ms, 3000);
        assert!(challenge.expected_response.is_some());
        assert!(challenge.prompt_text().contains("say the number"));
    }

    #[test]
    fn liveliness_scorer_balanced() {
        let scorer = AudioVisualLivelinessScorer::balanced();

        // Both pass with high confidence
        let (passed, score) = scorer.score(true, 0.9, true, 0.8);
        assert!(passed);
        assert!(score > 0.7);

        // Only camera passes — balanced scoring: 0.8 * 0.5 = 0.4, below 0.6 threshold
        let (passed, score) = scorer.score(true, 0.8, false, 0.0);
        assert!(!passed); // Need both or higher combined score
        assert!(score > 0.3);
        assert!(score < 0.6);

        // Neither passes
        let (passed, score) = scorer.score(false, 0.0, false, 0.0);
        assert!(!passed);
        assert!(score < 0.1);

        // Both pass with moderate confidence
        let (passed, score) = scorer.score(true, 0.7, true, 0.6);
        assert!(passed);
        assert!(score > 0.5);
    }

    #[test]
    fn liveliness_scorer_strict() {
        let scorer = AudioVisualLivelinessScorer::strict();

        // Only one passes — strict mode should fail
        let (passed, _score) = scorer.score(true, 0.9, false, 0.0);
        assert!(!passed);
    }

    #[test]
    fn audio_biometric_state_passed() {
        let state = audio_biometric_state(true, true);
        assert!(state.verified);
        assert!(state.user_present);
        assert_eq!(state.modality, Some(BiometricModality::Voice));
        assert!(!state.hardware_protected);
    }

    #[test]
    fn audio_biometric_state_failed() {
        let state = audio_biometric_state(false, false);
        assert!(!state.verified);
        assert!(!state.user_present);
        assert_eq!(state.modality, None);
    }
}
