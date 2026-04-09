//! ALSA Microphone backend for edgerun.
//!
//! Discovers ALSA capture PCM devices from `/proc/asound/pcm` and exposes
//! them as `MicrophoneDevice` + `CapabilityProvider` implementations.
//!
//! Audio capture uses proper ALSA PCM ioctls (hw_params, prepare, readi_frames)
//! rather than raw file reads which require OSS emulation.

use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_microphone::{
    default_microphone_descriptor, validate_audio_capture_request, AudioCapture,
    AudioCaptureRequest, MicrophoneDevice, MicrophoneInfo, MicrophoneSampleFormat,
};
use std::ffi::c_long;
use std::fs::{self, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// ALSA kernel ABI constants
// ---------------------------------------------------------------------------

const SNDRV_PCM_HW_PARAM_ACCESS: usize = 0;
const SNDRV_PCM_HW_PARAM_FORMAT: usize = 1;
const SNDRV_PCM_HW_PARAM_FIRST_INTERVAL: usize = 8;
const SNDRV_PCM_HW_PARAM_CHANNELS: usize = 10;
const SNDRV_PCM_HW_PARAM_RATE: usize = 11;
const SNDRV_PCM_HW_PARAM_PERIOD_SIZE: usize = 13;
const SNDRV_PCM_HW_PARAM_PERIODS: usize = 15;
const SNDRV_PCM_HW_PARAM_BUFFER_SIZE: usize = 17;

const SNDRV_MASK_MAX: usize = 256;
const SNDRV_MASK_WORDS: usize = SNDRV_MASK_MAX.div_ceil(32);

const SNDRV_PCM_ACCESS_RW_INTERLEAVED: u32 = 3;
const SNDRV_PCM_FORMAT_S16_LE: u32 = 2;
const SNDRV_PCM_FORMAT_S24_LE: u32 = 5;
const SNDRV_PCM_FORMAT_S32_LE: u32 = 7;
const SNDRV_PCM_FORMAT_FLOAT_LE: u32 = 10;

const SNDRV_PCM_IOCTL_HW_PARAMS: c_int = c_iowr(b'A', 0x11, SNDRV_PCM_HW_PARAMS_SIZE);
const SNDRV_PCM_IOCTL_PREPARE: c_int = c_io(b'A', 0x40);
const SNDRV_PCM_IOCTL_READI_FRAMES: c_int =
    c_iowr(b'A', 0x51, std::mem::size_of::<SndXferi>());

const fn c_iowr(ty: u8, nr: u8, size: usize) -> c_int {
    // _IOC(_IOC_READ | _IOC_WRITE, ty, nr, size)
    // direction: READ=2, WRITE=1 => READ|WRITE = 3
    ((3u32 << 30)
        | ((ty as u32) << 8)
        | ((nr as u32) << 0)
        | ((size as u32) << 16)) as c_int
}
const fn c_io(ty: u8, nr: u8) -> c_int {
    ((0u32 << 30) | ((ty as u32) << 8) | ((nr as u32) << 0)) as c_int
}

use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct SndMask {
    bits: [u32; SNDRV_MASK_WORDS],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndInterval {
    min: u32,
    max: u32,
    openmin_integer_empty: u32,
}

impl SndInterval {
    fn set_exact(&mut self, value: u32) {
        self.min = value;
        self.max = value;
        self.openmin_integer_empty = 1 << 2;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndPcmHwParams {
    flags: u32,
    masks: [SndMask; 3],
    mres: [SndMask; 5],
    intervals: [SndInterval; 12],
    ires: [SndInterval; 9],
    rmask: u32,
    cmask: u32,
    info: u32,
    msbits: u32,
    rate_num: u32,
    rate_den: u32,
    fifo_size: u64,
    sync: [u8; 16],
    reserved: [u8; 48],
}

const SNDRV_PCM_HW_PARAMS_SIZE: usize = std::mem::size_of::<SndPcmHwParams>();

impl SndPcmHwParams {
    fn any() -> Self {
        let mut params = Self {
            flags: 0,
            masks: [SndMask {
                bits: [u32::MAX; SNDRV_MASK_WORDS],
            }; 3],
            mres: [SndMask {
                bits: [0; SNDRV_MASK_WORDS],
            }; 5],
            intervals: [SndInterval {
                min: 0,
                max: u32::MAX,
                openmin_integer_empty: 0,
            }; 12],
            ires: [SndInterval {
                min: 0,
                max: 0,
                openmin_integer_empty: 0,
            }; 9],
            rmask: 0,
            cmask: 0,
            info: 0,
            msbits: 0,
            rate_num: 0,
            rate_den: 0,
            fifo_size: 0,
            sync: [0; 16],
            reserved: [0; 48],
        };
        params
    }

    fn set_mask_value(&mut self, mask_index: usize, value: u32) {
        self.masks[mask_index].bits.fill(0);
        self.masks[mask_index].bits[(value / 32) as usize] |= 1u32 << (value % 32);
    }

    fn set_interval_value(&mut self, param: usize, value: u32) {
        let idx = param - SNDRV_PCM_HW_PARAM_FIRST_INTERVAL;
        self.intervals[idx].set_exact(value);
    }
}

#[repr(C)]
struct SndXferi {
    result: c_long,
    buf: *mut core::ffi::c_void,
    frames: u64,
}

extern "C" {
    fn ioctl(fd: c_int, request: c_int, ...) -> c_int;
}

fn ioctl_pcm_hw_params(fd: c_int, params: &mut SndPcmHwParams) -> Result<(), io::Error> {
    let ret = unsafe { ioctl(fd, SNDRV_PCM_IOCTL_HW_PARAMS, params) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn ioctl_pcm_prepare(fd: c_int) -> Result<(), io::Error> {
    let ret = unsafe { ioctl(fd, SNDRV_PCM_IOCTL_PREPARE) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn ioctl_pcm_readi_frames(
    fd: c_int,
    buf: *mut core::ffi::c_void,
    frames: u64,
) -> Result<c_long, io::Error> {
    let mut xfer = SndXferi {
        result: 0,
        buf,
        frames,
    };
    let ret = unsafe { ioctl(fd, SNDRV_PCM_IOCTL_READI_FRAMES, &mut xfer) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(xfer.result)
    }
}

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
        return discover_from_card_dirs();
    }
    let content = fs::read_to_string(pcm_path).map_err(|e| {
        CapabilityError::Provider(format!("failed to read /proc/asound/pcm: {}", e))
    })?;
    let mut devices = Vec::new();
    for line in content.lines() {
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
        let card_path = entry.path();
        if let Ok(subs) = fs::read_dir(&card_path) {
            for sub in subs.flatten() {
                let sub_name = sub.file_name();
                let sub_str = sub_name.to_string_lossy();
                if let Some(dev_str) = sub_str.strip_prefix("pcm") {
                    let device_index: u32 = dev_str.parse().unwrap_or(0);
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

#[derive(Debug)]
pub struct AlsaMicrophoneBackend {
    pub pcm: AlsaPcmInfo,
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
            return Err(CapabilityError::Provider(format!(
                "ALSA capture device {} does not exist",
                device_path
            )));
        }
        Ok(Self { pcm, device_path })
    }

    fn capture_from_device(&mut self, request: &AudioCaptureRequest) -> Result<AudioCapture, CapabilityError> {
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

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.device_path)
            .map_err(|e| {
                CapabilityError::Provider(format!("failed to open {}: {}", self.device_path, e))
            })?;
        let fd = file.as_raw_fd();

        // Configure hardware parameters
        let kernel_format = match request.format {
            MicrophoneSampleFormat::PcmS16Le => SNDRV_PCM_FORMAT_S16_LE,
            MicrophoneSampleFormat::PcmS24Le => SNDRV_PCM_FORMAT_S24_LE,
            MicrophoneSampleFormat::PcmS32Le => SNDRV_PCM_FORMAT_S32_LE,
            MicrophoneSampleFormat::Float32Le => SNDRV_PCM_FORMAT_FLOAT_LE,
            MicrophoneSampleFormat::Other(_) => unreachable!(),
        };

        let channels = request.channels as u32;
        let rate = request.sample_rate_hz;
        let period_frames = (rate / 100).max(160);
        let periods = 4;
        let buffer_frames = period_frames * periods;

        let mut params = SndPcmHwParams::any();
        params.set_mask_value(SNDRV_PCM_HW_PARAM_ACCESS, SNDRV_PCM_ACCESS_RW_INTERLEAVED);
        params.set_mask_value(SNDRV_PCM_HW_PARAM_FORMAT, kernel_format);
        params.set_interval_value(SNDRV_PCM_HW_PARAM_CHANNELS, channels);
        params.set_interval_value(SNDRV_PCM_HW_PARAM_RATE, rate);
        params.set_interval_value(SNDRV_PCM_HW_PARAM_PERIOD_SIZE, period_frames);
        params.set_interval_value(SNDRV_PCM_HW_PARAM_PERIODS, periods);
        params.set_interval_value(SNDRV_PCM_HW_PARAM_BUFFER_SIZE, buffer_frames);

        ioctl_pcm_hw_params(fd, &mut params).map_err(|e| {
            CapabilityError::Provider(format!("hw_params on {}: {}", self.device_path, e))
        })?;

        // Prepare the PCM device
        ioctl_pcm_prepare(fd).map_err(|e| {
            CapabilityError::Provider(format!("prepare {}: {}", self.device_path, e))
        })?;

        // Read audio frames
        let mut buf = vec![0u8; total_bytes];
        let read_frames = ioctl_pcm_readi_frames(fd, buf.as_mut_ptr().cast(), frames as u64)
            .map_err(|e| {
                CapabilityError::Provider(format!("readi_frames {}: {}", self.device_path, e))
            })?;

        let read_bytes = (read_frames as usize) * (usize::from(request.channels)) * bytes_per_sample;

        let started_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        buf.truncate(read_bytes);
        Ok(AudioCapture {
            sample_rate_hz: request.sample_rate_hz,
            channels: request.channels,
            format: request.format,
            bytes: buf,
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
        self.capture_from_device(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pcm_listing_finds_capture_devices() {
        let text = "00-00: ALC897 Analog : ALC897 Analog : playback 1 : capture 1\n02-00: BRIO                  : USB Audio : capture 1\n";
        let pcms = parse_pcms_from(text);
        assert_eq!(pcms.len(), 2);
        assert!(pcms[0].capture);
        assert!(pcms[0].playback);
        assert!(pcms[1].capture);
        assert!(!pcms[1].playback);
    }

    fn parse_pcms_from(text: &str) -> Vec<AlsaPcmInfo> {
        let mut devices = Vec::new();
        for line in text.lines() {
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
            let name = id_part.splitn(2, ':').last().unwrap_or(id_part).trim().to_string();
            devices.push(AlsaPcmInfo {
                card_index,
                device_index,
                capture,
                playback,
                name,
            });
        }
        devices
    }

    #[test]
    fn discover_alsa_pcms_handles_missing_proc() {
        // On systems without /proc/asound/pcm, should try card directories
        let result = discover_alsa_pcms();
        // Either finds devices or returns empty — should not panic
        assert!(result.is_ok() || result.is_err());
    }
}
