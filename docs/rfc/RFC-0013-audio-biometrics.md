# RFC-0013: Audio, Biometrics & Input

**Status:** Working Draft
**Date:** 2026-04-12

---

## Abstract

Audio, biometrics, and input crates provide hardware abstraction and driver implementations for microphone capture, speaker output, audio calibration, biometric authentication (camera-based, fingerprint, face detection), and input event handling.

---

## Audio Stack

### Type-Only Crates (Generated from Proto)

| Crate | Purpose |
|-------|---------|
| `edgerun-microphone` | Microphone device types |
| `edgerun-speaker` | Speaker device types |

### Linux Backend Crates

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-alsa-microphone` | ⚠️ Partial | ALSA microphone capture |
| `edgerun-alsa-speaker` | ⚠️ Partial | ALSA speaker output |
| `edgerun-audio-calibration` | ⚠️ Partial | Audio calibration |
| `edgerun-audio-liveliness` | ⚠️ Partial | Audio liveliness detection |

---

## Biometrics Stack

### edgerun-biometrics

**Status:** ⚠️ Partial
**Purpose:** Biometric abstraction layer — common interface for all biometric modalities.

### Camera-Based Biometrics

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-camera-biometrics` | ⚠️ Partial | Camera-based biometric authentication |
| `edgerun-face-detection` | ⚠️ Partial | Face detection |
| `edgerun-v4l2-camera` | ⚠️ Partial | V4L2 camera capture |

### Fingerprint

| Crate | Status | Purpose |
|-------|--------|---------|
| `edgerun-fingerprint` | ⚠️ Partial | Fingerprint sensor abstraction |
| `edgerun-goodix-fingerprint` | ⚠️ Partial | Goodix fingerprint driver |

---

## Input Stack

### edgerun-input

**Status:** ✅ Generated
**Purpose:** Input device types from proto.

### edgerun-evdev-input

**Status:** ⚠️ Partial
**Purpose:** Linux evdev input handling — keyboard, mouse, touch events from `/dev/input/event*`.

### edgerun-touch

**Status:** ✅ Generated
**Purpose:** Touch event types from proto.

---

## Known Issues

1. **All partial** — Every audio, biometric, and input crate is marked partial/skeleton
2. **No actual audio processing** — ALSA crates exist but no PCM capture/playback pipeline
3. **No actual biometric algorithms** — Face detection, fingerprint matching not implemented
4. **No camera pipeline** — V4L2 types exist but no capture/encoding/streaming
5. **No input event loop** — evdev types defined but no event reading/dispatch
6. **Audio calibration/liveliness undefined** — Purpose unclear from crate names alone
7. **No speech recognition** — Despite having microphone capture types, no speech-to-text pipeline
8. **Hardware dependencies** — Fingerprint drivers require specific hardware (Goodix)
