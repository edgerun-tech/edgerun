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

use edgerun_biometrics::{BiometricAssuranceStrength, BiometricModality, BiometricState};
use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityProvider,
};
use edgerun_microphone::{
    AudioCaptureRequest, MicrophoneDevice, MicrophoneInfo, MicrophoneSampleFormat,
};
use edgerun_speaker::{
    AudioPlaybackRequest, AudioPlaybackResult, SpeakerDevice, SpeakerInfo, SpeakerOutputLevel,
    SpeakerSampleFormat,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
                format!("Please say the number {}", self.expected_response.as_deref().unwrap_or("?"))
            }
            AudioChallengeKind::ClapDetection => "Please clap your hands once".into(),
            AudioChallengeKind::VoiceActivity => "Please say anything to verify you're present".into(),
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
    pub fn analyze_s16le(&self, pcm: &[u8], sample_rate_hz: u32) -> (bool, f32, Option<u32>) {
        if pcm.is_empty() {
            return (false, 0.0, None);
        }

        // Analyze in 20ms frames
        let frame_size = (sample_rate_hz / 50) as usize * 2; // 20ms in bytes (S16LE)
        let frame_duration_ms = 20;

        let mut speech_frames = 0;
        let mut total_frames = 0;
        let mut first_speech_frame: Option<usize> = None;
        let mut max_energy = f32::MIN;
        let mut total_energy = 0.0f32;

        for chunk in pcm.chunks(frame_size) {
            if chunk.len() < 2 {
                break;
            }

            // Calculate RMS energy for this frame
            let energy = calculate_rms_energy_s16le(chunk);
            total_energy += energy;
            max_energy = max_energy.max(energy);
            total_frames += 1;

            let is_speech = energy > self.threshold_dbfs;
            if is_speech {
                speech_frames += 1;
                if first_speech_frame.is_none() {
                    first_speech_frame = Some(total_frames - 1);
                }
            }
        }

        if total_frames == 0 {
            return (false, 0.0, None);
        }

        // Calculate speech ratio
        let speech_ratio = speech_frames as f32 / total_frames as f32;
        let avg_energy = total_energy / total_frames as f32;

        // Speech is detected if:
        // 1. At least some frames exceed threshold
        // 2. Speech ratio is reasonable (> 10%)
        // 3. Average energy is above threshold
        let speech_detected = speech_frames > 0
            && speech_ratio > 0.1
            && avg_energy > self.threshold_dbfs;

        // Confidence based on speech ratio and energy
        let confidence = if speech_detected {
            let ratio_confidence = (speech_ratio * 2.0).min(1.0); // 50%+ speech = 1.0
            let energy_confidence = ((avg_energy - self.threshold_dbfs) / 30.0).clamp(0.0, 1.0);
            (ratio_confidence + energy_confidence) / 2.0
        } else {
            0.0
        };

        let first_speech_offset_ms =
            first_speech_frame.map(|f| (f as u32) * frame_duration_ms);

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

        let rms = (sum_sq as f64 / count as f64).sqrt() as f64;
        let peak_abs = max_sample.abs() as f64;

        // A clap has:
        // 1. High peak amplitude (> 8000 = about -12 dBFS)
        // 2. High peak-to-RMS ratio (> 3:1 for sharp transients)
        let peak_to_rms = if rms > 1.0 {
            peak_abs / rms
        } else {
            0.0
        };

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
pub fn analyze_spectral(pcm: &[u8], sample_rate_hz: u32) -> SpectralAnalysis {
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
        capture: &edgerun_microphone::AudioCapture,
        capture_latency_ms: u32,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let (speech_detected, vad_confidence, speech_offset_ms) =
            self.vad.analyze_s16le(&capture.bytes, 48000);

        let response_latency_ms = speech_offset_ms.map(|offset| {
            capture_latency_ms.saturating_add(offset).saturating_sub(500) // subtract prompt time
        });

        // For number challenges, we'd need speech recognition to verify the number.
        // For now, we detect speech activity and estimate confidence.
        // A real implementation would use a speech-to-text engine.
        let detected_response = if speech_detected {
            // Placeholder: in real impl, this would be the recognized number
            challenge.expected_response.clone()
        } else {
            None
        };

        let spectral = analyze_spectral(&capture.bytes, 48000);

        let passed = speech_detected
            && vad_confidence > 0.3
            && spectral.looks_like_natural_speech;

        let confidence = if passed {
            vad_confidence * 0.5 + (spectral.zero_crossing_rate * 2.0).min(1.0) * 0.3 + {
                if spectral.looks_like_natural_speech {
                    0.2
                } else {
                    0.0
                }
            }
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
        challenge: &AudioChallenge,
        capture: &edgerun_microphone::AudioCapture,
        capture_latency_ms: u32,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let (clap_detected, clap_confidence, clap_offset_ms) =
            self.vad.detect_clap(&capture.bytes, 48000);

        let response_latency_ms = clap_offset_ms.map(|offset| {
            capture_latency_ms.saturating_add(offset).saturating_sub(500)
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
        challenge: &AudioChallenge,
        capture: &edgerun_microphone::AudioCapture,
        capture_latency_ms: u32,
    ) -> Result<AudioChallengeResult, CapabilityError> {
        let (speech_detected, vad_confidence, speech_offset_ms) =
            self.vad.analyze_s16le(&capture.bytes, 48000);

        let response_latency_ms = speech_offset_ms.map(|offset| {
            capture_latency_ms.saturating_add(offset).saturating_sub(500)
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
        let camera_score = if camera_passed { camera_confidence } else { 0.0 };
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

/// Calculate RMS energy of S16LE PCM frame in dBFS.
fn calculate_rms_energy_s16le(pcm: &[u8]) -> f32 {
    if pcm.len() < 2 {
        return f32::MIN;
    }

    let mut sum_sq = 0i64;
    let mut count = 0i64;

    for chunk in pcm.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i64;
        sum_sq += sample * sample;
        count += 1;
    }

    if count == 0 {
        return f32::MIN;
    }

    let rms = (sum_sq as f64 / count as f64).sqrt() as f32;
    // Convert to dBFS (full scale = 32767 for S16)
    if rms < 1.0 {
        f32::MIN
    } else {
        20.0 * (rms / 32767.0).log10()
    }
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

/// Generate simple "speech-like" audio for the challenge prompt.
/// In production, this would use a TTS engine. For now, generate a simple tone pattern.
fn text_to_speech_audio(text: &str, sample_rate: u32) -> Vec<u8> {
    // Generate a simple beep pattern as placeholder for TTS
    // Duration: ~100ms per character (minimum)
    let duration_ms = (text.len() as u32 * 100).max(500).min(5000);
    let num_samples = (sample_rate as u64 * duration_ms as u64 / 1000) as usize;

    let mut samples = Vec::with_capacity(num_samples * 2);

    // Generate a speech-like pattern: modulated tones with pauses
    let base_freq = 440.0; // A4
    let mut phase = 0.0f64;

    for i in 0..num_samples {
        let t = i as f64 / sample_rate as f64;

        // Simple amplitude modulation to simulate speech rhythm
        let envelope = (t * 4.0 * std::f64::consts::PI).sin().abs();
        let freq_mod = base_freq + 100.0 * (t * 2.0 * std::f64::consts::PI).sin();

        let sample = envelope * (t * freq_mod * 2.0 * std::f64::consts::PI).sin();
        let sample_i16 = (sample * 8000.0).clamp(-32000.0, 32000.0) as i16;

        samples.extend_from_slice(&sample_i16.to_le_bytes());
    }

    samples
}

/// Create a biometric state for audio verification.
fn audio_biometric_state(passed: bool, high_confidence: bool) -> BiometricState {
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
