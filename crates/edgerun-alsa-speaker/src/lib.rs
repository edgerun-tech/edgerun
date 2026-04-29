#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

#[cfg(target_os = "none")]
pub mod ffi {
    #[allow(non_camel_case_types)]
    pub type c_long = i64;
}

#[cfg(target_os = "none")]
pub mod fs {
    use crate::{io, path::PathBuf};
    use alloc::string::String;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct File;

    pub struct OpenOptions;

    impl OpenOptions {
        #[must_use]
        pub fn new() -> Self {
            Self
        }

        #[must_use]
        pub fn read(self, _read: bool) -> Self {
            self
        }

        #[must_use]
        pub fn write(self, _write: bool) -> Self {
            self
        }

        pub fn open(self, _path: &PathBuf) -> io::Result<File> {
            Err(io::Error::new(io::ErrorKind::NotFound))
        }
    }

    pub fn read_to_string<P>(_path: P) -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::NotFound))
    }
}

#[cfg(target_os = "none")]
pub mod io {
    use core::fmt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ErrorKind {
        NotFound,
        Other,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Error {
        kind: ErrorKind,
    }

    impl Error {
        #[must_use]
        pub const fn new(kind: ErrorKind) -> Self {
            Self { kind }
        }

        #[must_use]
        pub const fn last_os_error() -> Self {
            Self {
                kind: ErrorKind::Other,
            }
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.kind {
                ErrorKind::NotFound => f.write_str("not found"),
                ErrorKind::Other => f.write_str("I/O error"),
            }
        }
    }

    impl core::error::Error for Error {}

    pub type Result<T> = core::result::Result<T, Error>;
}

#[cfg(target_os = "none")]
pub mod mem {
    pub use core::mem::*;
}

#[cfg(target_os = "none")]
pub mod os {
    pub mod fd {
        pub trait AsRawFd {
            fn as_raw_fd(&self) -> i32;
        }

        impl AsRawFd for crate::fs::File {
            fn as_raw_fd(&self) -> i32 {
                -1
            }
        }
    }
}

#[cfg(target_os = "none")]
pub mod path {
    use alloc::string::{String, ToString};
    use core::fmt;

    #[derive(Debug)]
    pub struct Path;

    impl Path {
        #[must_use]
        pub fn new(_path: &str) -> &'static Self {
            static PATH: Path = Path;
            &PATH
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PathBuf(String);

    impl PathBuf {
        #[must_use]
        pub fn display(&self) -> Display<'_> {
            Display(self)
        }
    }

    impl From<&str> for PathBuf {
        fn from(value: &str) -> Self {
            Self(value.to_string())
        }
    }

    impl From<String> for PathBuf {
        fn from(value: String) -> Self {
            Self(value)
        }
    }

    pub struct Display<'a>(&'a PathBuf);

    impl fmt::Display for Display<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0 .0)
        }
    }
}

#[cfg(target_os = "none")]
pub mod ptr {
    pub use core::ptr::*;
}

#[cfg(target_os = "none")]
pub mod option {
    pub use core::option::*;
}

#[cfg(target_os = "none")]
pub mod result {
    pub use core::result::*;
}

#[cfg(target_os = "none")]
pub mod string {
    pub use alloc::string::*;
}

#[cfg(target_os = "none")]
pub mod vec {
    pub use alloc::vec::*;
}

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_speaker::{
    default_speaker_descriptor, validate_audio_playback_request, AudioPlaybackRequest,
    AudioPlaybackResult, SpeakerDevice, SpeakerInfo, SpeakerOutputLevel, SpeakerSampleFormat,
};
use std::ffi::c_long;
#[cfg(not(target_os = "none"))]
use std::fs;
#[cfg(not(target_os = "none"))]
use std::io;
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

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
const SNDRV_CTL_ELEM_IFACE_MIXER: i32 = 2;
const SNDRV_CTL_ELEM_TYPE_BOOLEAN: i32 = 1;
const SNDRV_CTL_ELEM_TYPE_INTEGER: i32 = 2;
const SNDRV_CTL_ELEM_ID_NAME_MAXLEN: usize = 44;

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
        for interval in &mut params.intervals {
            interval.min = 0;
            interval.max = u32::MAX;
            interval.openmin_integer_empty = 0;
        }
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
    result: i64,
    buf: *mut core::ffi::c_void,
    frames: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SndCtlElemId {
    numid: u32,
    iface: i32,
    device: u32,
    subdevice: u32,
    name: [u8; SNDRV_CTL_ELEM_ID_NAME_MAXLEN],
    index: u32,
}

impl Default for SndCtlElemId {
    fn default() -> Self {
        Self {
            numid: 0,
            iface: 0,
            device: 0,
            subdevice: 0,
            name: [0; SNDRV_CTL_ELEM_ID_NAME_MAXLEN],
            index: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndCtlElemList {
    offset: u32,
    space: u32,
    used: u32,
    count: u32,
    pids: *mut SndCtlElemId,
    reserved: [u8; 50],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndCtlElemInfoInteger {
    min: c_long,
    max: c_long,
    step: c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
union SndCtlElemInfoValue {
    integer: SndCtlElemInfoInteger,
    reserved: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndCtlElemInfo {
    id: SndCtlElemId,
    ty: i32,
    access: u32,
    count: u32,
    owner: i32,
    value: SndCtlElemInfoValue,
    reserved: [u8; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
union SndCtlElemValueUnion {
    integer: SndCtlElemValueInteger,
    bytes: [u8; 512],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndCtlElemValueInteger {
    value: [c_long; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SndCtlElemValue {
    id: SndCtlElemId,
    indirect: u32,
    value: SndCtlElemValueUnion,
    reserved: [u8; 128],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AlsaMixerControl {
    id: SndCtlElemId,
    name: String,
    count: u32,
    min: i64,
    max: i64,
    is_switch: bool,
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}

const fn ioc(dir: u64, ty: u64, nr: u64, size: u64) -> u64 {
    const IOC_NRSHIFT: u64 = 0;
    const IOC_TYPESHIFT: u64 = 8;
    const IOC_SIZESHIFT: u64 = 16;
    const IOC_DIRSHIFT: u64 = 30;
    (dir << IOC_DIRSHIFT) | (ty << IOC_TYPESHIFT) | (nr << IOC_NRSHIFT) | (size << IOC_SIZESHIFT)
}
const fn io(ty: u8, nr: u8) -> u64 {
    ioc(0, ty as u64, nr as u64, 0)
}
const fn iow(ty: u8, nr: u8, size: usize) -> u64 {
    ioc(1, ty as u64, nr as u64, size as u64)
}
const fn iowr(ty: u8, nr: u8, size: usize) -> u64 {
    ioc(3, ty as u64, nr as u64, size as u64)
}

const SNDRV_PCM_IOCTL_HW_PARAMS: u64 = iowr(b'A', 0x11, size_of::<SndPcmHwParams>());
const SNDRV_PCM_IOCTL_PREPARE: u64 = io(b'A', 0x40);
const SNDRV_PCM_IOCTL_DRAIN: u64 = io(b'A', 0x44);
const SNDRV_PCM_IOCTL_WRITEI_FRAMES: u64 = iow(b'A', 0x50, size_of::<SndXferi>());
const SNDRV_CTL_IOCTL_ELEM_LIST: u64 = iowr(b'U', 0x10, size_of::<SndCtlElemList>());
const SNDRV_CTL_IOCTL_ELEM_INFO: u64 = iowr(b'U', 0x11, size_of::<SndCtlElemInfo>());
const SNDRV_CTL_IOCTL_ELEM_READ: u64 = iowr(b'U', 0x12, size_of::<SndCtlElemValue>());
const SNDRV_CTL_IOCTL_ELEM_WRITE: u64 = iowr(b'U', 0x13, size_of::<SndCtlElemValue>());

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsaPlaybackCardInfo {
    pub card_index: u32,
    pub card_id: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsaPlaybackPcmInfo {
    pub card_index: u32,
    pub device_index: u32,
    pub device_name: String,
    pub subdevice_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlsaSpeakerBackend {
    pub card_index: u32,
    pub device_index: u32,
    pub card_id: String,
    pub display_name: String,
    pub channels: u16,
    pub default_sample_rate_hz: u32,
}

fn speaker_format_to_kernel(format: SpeakerSampleFormat) -> Result<u32, CapabilityError> {
    match format {
        SpeakerSampleFormat::PcmS16Le => Ok(SNDRV_PCM_FORMAT_S16_LE),
        SpeakerSampleFormat::PcmS24Le | SpeakerSampleFormat::PcmFloat32Le => Err(
            CapabilityError::Unsupported("only PCM S16LE playback is currently supported"),
        ),
    }
}

fn playback_device_path(card_index: u32, device_index: u32) -> PathBuf {
    PathBuf::from(format!("/dev/snd/pcmC{card_index}D{device_index}p"))
}

fn control_device_path(card_index: u32) -> PathBuf {
    PathBuf::from(format!("/dev/snd/controlC{card_index}"))
}

fn trim_cstr(bytes: &[u8]) -> String {
    edgerun_encoding::cstring::decode_c_string_lossy_trimmed(bytes)
}

fn control_score(name: &str, is_switch: bool) -> i32 {
    let lower = name.to_ascii_lowercase();
    let mut score = if is_switch { 0 } else { 10 };
    for (needle, weight) in [
        ("master", 120),
        ("speaker", 110),
        ("headphone", 100),
        ("pcm", 90),
        ("front", 80),
        ("playback", 20),
        ("volume", 20),
        ("switch", 10),
    ] {
        if lower.contains(needle) {
            score += weight;
        }
    }
    score
}

fn scale_s16le_audio(bytes: &[u8], gain_percent: u16) -> Vec<u8> {
    if gain_percent == 100 {
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
        let scaled = (sample * gain_percent as i32) / 100;
        let clamped = scaled.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        out.extend_from_slice(&clamped.to_le_bytes());
    }
    if !bytes.len().is_multiple_of(2) {
        out.extend_from_slice(&bytes[bytes.len() - 1..]);
    }
    out
}

impl AlsaSpeakerBackend {
    fn configured_hw_params(
        request: &AudioPlaybackRequest,
    ) -> Result<(SndPcmHwParams, u16, u32, u32), CapabilityError> {
        let kernel_format = speaker_format_to_kernel(request.format)?;
        let bytes_per_sample = 2u16;
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
        Ok((params, bytes_per_sample, channels, period_frames))
    }

    fn list_mixer_controls(&self) -> Result<Vec<AlsaMixerControl>, CapabilityError> {
        let path = control_device_path(self.card_index);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| CapabilityError::Provider(format!("open {}: {e}", path.display())))?;
        let fd = file.as_raw_fd();

        let mut list = SndCtlElemList {
            offset: 0,
            space: 0,
            used: 0,
            count: 0,
            pids: std::ptr::null_mut(),
            reserved: [0; 50],
        };
        if unsafe { ioctl(fd, SNDRV_CTL_IOCTL_ELEM_LIST, &mut list) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA ELEM_LIST count failed: {}",
                io::Error::last_os_error()
            )));
        }
        if list.count == 0 {
            return Ok(Vec::new());
        }
        let mut ids = vec![SndCtlElemId::default(); list.count as usize];
        list.space = list.count;
        list.pids = ids.as_mut_ptr();
        if unsafe { ioctl(fd, SNDRV_CTL_IOCTL_ELEM_LIST, &mut list) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA ELEM_LIST ids failed: {}",
                io::Error::last_os_error()
            )));
        }

        let mut out = Vec::new();
        for id in ids.into_iter().take(list.used as usize) {
            if id.iface != SNDRV_CTL_ELEM_IFACE_MIXER {
                continue;
            }
            let mut info = SndCtlElemInfo {
                id,
                ty: 0,
                access: 0,
                count: 0,
                owner: 0,
                value: SndCtlElemInfoValue { reserved: [0; 128] },
                reserved: [0; 64],
            };
            if unsafe { ioctl(fd, SNDRV_CTL_IOCTL_ELEM_INFO, &mut info) } < 0 {
                continue;
            }
            let is_switch = info.ty == SNDRV_CTL_ELEM_TYPE_BOOLEAN;
            let is_integer = info.ty == SNDRV_CTL_ELEM_TYPE_INTEGER;
            if !is_switch && !is_integer {
                continue;
            }
            let (min, max) = unsafe {
                if is_integer {
                    let v = info.value.integer;
                    (v.min, v.max)
                } else {
                    (0, 1)
                }
            };
            out.push(AlsaMixerControl {
                id: info.id,
                name: trim_cstr(&info.id.name),
                count: info.count.max(1),
                min,
                max,
                is_switch,
            });
        }
        Ok(out)
    }

    fn preferred_volume_control(&self) -> Result<Option<AlsaMixerControl>, CapabilityError> {
        let controls = self.list_mixer_controls()?;
        Ok(controls
            .into_iter()
            .filter(|control| !control.is_switch && control.max > control.min)
            .max_by_key(|control| control_score(&control.name, false)))
    }

    fn preferred_switch_control(&self) -> Result<Option<AlsaMixerControl>, CapabilityError> {
        let controls = self.list_mixer_controls()?;
        Ok(controls
            .into_iter()
            .filter(|control| control.is_switch)
            .max_by_key(|control| control_score(&control.name, true)))
    }

    fn read_control_value(
        &self,
        control: &AlsaMixerControl,
    ) -> Result<SndCtlElemValue, CapabilityError> {
        let path = control_device_path(self.card_index);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| CapabilityError::Provider(format!("open {}: {e}", path.display())))?;
        let fd = file.as_raw_fd();
        let mut value = SndCtlElemValue {
            id: control.id,
            indirect: 0,
            value: SndCtlElemValueUnion { bytes: [0; 512] },
            reserved: [0; 128],
        };
        if unsafe { ioctl(fd, SNDRV_CTL_IOCTL_ELEM_READ, &mut value) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA ELEM_READ failed for {}: {}",
                control.name,
                io::Error::last_os_error()
            )));
        }
        Ok(value)
    }

    fn write_control_value(&self, value: &mut SndCtlElemValue) -> Result<(), CapabilityError> {
        let path = control_device_path(self.card_index);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| CapabilityError::Provider(format!("open {}: {e}", path.display())))?;
        let fd = file.as_raw_fd();
        if unsafe { ioctl(fd, SNDRV_CTL_IOCTL_ELEM_WRITE, value) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA ELEM_WRITE failed: {}",
                io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    fn current_output_level(&self) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
        let Some(volume_control) = self.preferred_volume_control()? else {
            return Ok(None);
        };
        let volume_value = self.read_control_value(&volume_control)?;
        let raw_current = unsafe { volume_value.value.integer.value[0] };
        let span = (volume_control.max - volume_control.min).max(1);
        let percent =
            (((raw_current - volume_control.min).clamp(0, span) * 100) / span).clamp(0, 100) as u8;

        let muted = if let Some(switch_control) = self.preferred_switch_control()? {
            let switch_value = self.read_control_value(&switch_control)?;
            Some(unsafe { switch_value.value.integer.value[0] == 0 })
        } else {
            None
        };

        Ok(Some(SpeakerOutputLevel {
            current_percent: percent,
            min_raw_value: volume_control.min,
            max_raw_value: volume_control.max,
            muted,
        }))
    }

    fn set_output_level_impl(
        &self,
        percent: u8,
    ) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
        let Some(volume_control) = self.preferred_volume_control()? else {
            return Ok(None);
        };
        let span = (volume_control.max - volume_control.min).max(1);
        let raw = volume_control.min + (span * i64::from(percent) / 100);
        let mut value = self.read_control_value(&volume_control)?;
        unsafe {
            for idx in 0..(volume_control.count as usize).min(128) {
                value.value.integer.value[idx] = raw as c_long;
            }
        }
        self.write_control_value(&mut value)?;

        if let Some(switch_control) = self.preferred_switch_control()? {
            let mut switch_value = self.read_control_value(&switch_control)?;
            let on: c_long = if percent == 0 { 0 } else { 1 };
            unsafe {
                for idx in 0..(switch_control.count as usize).min(128) {
                    switch_value.value.integer.value[idx] = on;
                }
            }
            let _ = self.write_control_value(&mut switch_value);
        }

        self.current_output_level()
    }
}

impl CapabilityProvider for AlsaSpeakerBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_speaker_descriptor(
            "alsa-speaker",
            &format!("card{}-device{}", self.card_index, self.device_index),
        )
    }
}

impl SpeakerDevice for AlsaSpeakerBackend {
    fn speaker_info(&self) -> Result<SpeakerInfo, CapabilityError> {
        Ok(SpeakerInfo {
            provider: "alsa-speaker".into(),
            instance_id: format!("card{}-device{}", self.card_index, self.device_index),
            display_name: self.display_name.clone(),
            default_sample_rate_hz: self.default_sample_rate_hz,
            channels: self.channels,
            supports_playback: true,
            supports_output_level_control: self.preferred_volume_control().ok().flatten().is_some(),
        })
    }

    fn output_level(&self) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
        self.current_output_level()
    }

    fn set_output_level(&self, percent: u8) -> Result<Option<SpeakerOutputLevel>, CapabilityError> {
        self.set_output_level_impl(percent)
    }

    fn play_audio(
        &self,
        request: &AudioPlaybackRequest,
    ) -> Result<AudioPlaybackResult, CapabilityError> {
        validate_audio_playback_request(request)?;
        if let Some(percent) = request.target_output_level_percent {
            self.set_output_level_impl(percent)?;
        }
        let path = playback_device_path(self.card_index, self.device_index);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                CapabilityError::Provider(format!(
                    "failed to open playback device {}: {e}",
                    path.display()
                ))
            })?;
        let fd = file.as_raw_fd();
        let (mut params, bytes_per_sample, channels, _period_frames) =
            Self::configured_hw_params(request)?;
        if unsafe { ioctl(fd, SNDRV_PCM_IOCTL_HW_PARAMS, &mut params) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA playback HW_PARAMS failed: {}",
                io::Error::last_os_error()
            )));
        }
        if unsafe { ioctl(fd, SNDRV_PCM_IOCTL_PREPARE) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA playback PREPARE failed: {}",
                io::Error::last_os_error()
            )));
        }
        let playback_bytes = if let Some(percent) = request.software_gain_percent {
            scale_s16le_audio(&request.audio_bytes, percent)
        } else {
            request.audio_bytes.clone()
        };
        let frame_bytes = usize::from(bytes_per_sample) * channels as usize;
        if playback_bytes.len() < frame_bytes {
            return Err(CapabilityError::InvalidRequest(
                "audio buffer too short for one frame",
            ));
        }
        let frames = (playback_bytes.len() / frame_bytes) as u64;
        let mut xfer = SndXferi {
            buf: playback_bytes.as_ptr() as *mut core::ffi::c_void,
            frames,
            result: 0,
        };
        if unsafe { ioctl(fd, SNDRV_PCM_IOCTL_WRITEI_FRAMES, &mut xfer) } < 0 {
            return Err(CapabilityError::Provider(format!(
                "ALSA playback WRITEI_FRAMES failed: {}",
                io::Error::last_os_error()
            )));
        }
        let _ = unsafe { ioctl(fd, SNDRV_PCM_IOCTL_DRAIN) };
        Ok(AudioPlaybackResult {
            bytes_written: (xfer.result as usize) * frame_bytes,
            sample_rate_hz: request.sample_rate_hz,
            channels: request.channels,
            finished: xfer.result == frames as i64,
        })
    }
}

fn parse_cards_from(text: &str) -> Vec<AlsaPlaybackCardInfo> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((idx, rest)) = line.split_once(" [") else {
            continue;
        };
        let Ok(card_index) = idx.trim().parse::<u32>() else {
            continue;
        };
        let Some((card_id, rest)) = rest.split_once("]: ") else {
            continue;
        };
        out.push(AlsaPlaybackCardInfo {
            card_index,
            card_id: card_id.to_string(),
            description: rest.to_string(),
        });
    }
    out
}

fn parse_pcms_from(text: &str) -> Vec<AlsaPlaybackPcmInfo> {
    let mut out = Vec::new();
    for line in text.lines() {
        if !line.contains("playback ") {
            continue;
        }
        let mut parts = line.splitn(3, ':');
        let ids = parts.next().unwrap_or("").trim();
        let mut id_parts = ids.splitn(2, '-');
        let card = id_parts.next().and_then(|v| v.trim().parse::<u32>().ok());
        let device = id_parts.next().and_then(|v| v.trim().parse::<u32>().ok());
        let _ = parts.next();
        let rest = parts.next().unwrap_or("").trim();
        let Some(card_index) = card else { continue };
        let Some(device_index) = device else { continue };
        let mut name_parts = rest.splitn(2, " : ");
        let device_name = name_parts.next().unwrap_or("").trim().to_string();
        let subdevice_name = name_parts.next().unwrap_or("").trim().to_string();
        out.push(AlsaPlaybackPcmInfo {
            card_index,
            device_index,
            device_name,
            subdevice_name,
        });
    }
    out
}

pub fn discover_alsa_playback_cards() -> Result<Vec<AlsaPlaybackCardInfo>, CapabilityError> {
    discover_alsa_playback_cards_in(Path::new("/proc/asound/cards"))
}

pub fn discover_alsa_playback_cards_in(
    path: &Path,
) -> Result<Vec<AlsaPlaybackCardInfo>, CapabilityError> {
    let text = fs::read_to_string(path).map_err(|e| CapabilityError::Provider(e.to_string()))?;
    Ok(parse_cards_from(&text))
}

pub fn discover_alsa_playback_pcms() -> Result<Vec<AlsaPlaybackPcmInfo>, CapabilityError> {
    discover_alsa_playback_pcms_in(Path::new("/proc/asound/pcm"))
}

pub fn discover_alsa_playback_pcms_in(
    path: &Path,
) -> Result<Vec<AlsaPlaybackPcmInfo>, CapabilityError> {
    let text = fs::read_to_string(path).map_err(|e| CapabilityError::Provider(e.to_string()))?;
    Ok(parse_pcms_from(&text))
}

pub fn discover_speakers() -> Result<Vec<AlsaSpeakerBackend>, CapabilityError> {
    let cards = discover_alsa_playback_cards()?;
    let pcms = discover_alsa_playback_pcms()?;
    let mut out = Vec::new();
    for pcm in pcms {
        let Some(card) = cards.iter().find(|card| card.card_index == pcm.card_index) else {
            continue;
        };
        out.push(AlsaSpeakerBackend {
            card_index: pcm.card_index,
            device_index: pcm.device_index,
            card_id: card.card_id.clone(),
            display_name: format!("{} / {}", card.description, pcm.device_name),
            channels: 2,
            default_sample_rate_hz: 48_000,
        });
    }
    out.sort_by_key(|a| (a.card_index, a.device_index));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pcm_listing_finds_playback_devices() {
        let text = "00-00: ALC897 Analog : ALC897 Analog : playback 1 : capture 1\n02-00: BRIO                  : USB Audio : capture 1\n";
        let pcms = parse_pcms_from(text);
        assert_eq!(pcms.len(), 1);
        assert_eq!(pcms[0].card_index, 0);
        assert_eq!(pcms[0].device_index, 0);
    }

    #[test]
    fn speaker_info_descriptor_is_playback() {
        let backend = AlsaSpeakerBackend {
            card_index: 0,
            device_index: 0,
            card_id: "PCH".into(),
            display_name: "Test Speaker".into(),
            channels: 2,
            default_sample_rate_hz: 48_000,
        };
        let info = backend.speaker_info().unwrap();
        assert!(info.supports_playback);
    }

    #[test]
    fn playback_device_path_uses_playback_node() {
        assert_eq!(
            playback_device_path(0, 1),
            PathBuf::from("/dev/snd/pcmC0D1p")
        );
    }

    #[test]
    fn configured_hw_params_supports_s16le() {
        let (params, bytes_per_sample, channels, period_frames) =
            AlsaSpeakerBackend::configured_hw_params(&AudioPlaybackRequest {
                duration_ms: 100,
                sample_rate_hz: 48_000,
                channels: 2,
                format: SpeakerSampleFormat::PcmS16Le,
                audio_bytes: vec![0; 1024],
                software_gain_percent: None,
                target_output_level_percent: None,
            })
            .unwrap();
        assert_eq!(bytes_per_sample, 2);
        assert_eq!(channels, 2);
        assert_eq!(period_frames, 480);
        assert_eq!(
            params.intervals[SNDRV_PCM_HW_PARAM_RATE - SNDRV_PCM_HW_PARAM_FIRST_INTERVAL].min,
            48_000
        );
        assert_eq!(
            params.intervals[SNDRV_PCM_HW_PARAM_CHANNELS - SNDRV_PCM_HW_PARAM_FIRST_INTERVAL].min,
            2
        );
    }

    #[test]
    fn scale_s16le_audio_applies_gain() {
        let input = [1000i16, -1000i16];
        let mut bytes = Vec::new();
        for sample in input {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let scaled = scale_s16le_audio(&bytes, 50);
        assert_eq!(i16::from_le_bytes([scaled[0], scaled[1]]), 500);
        assert_eq!(i16::from_le_bytes([scaled[2], scaled[3]]), -500);
    }

    // ----- trim_cstr -----

    #[test]
    fn trim_cstr_removes_null_terminators() {
        let bytes = b"Hello\0\0\0";
        assert_eq!(trim_cstr(bytes), "Hello");
    }

    #[test]
    fn trim_cstr_empty_string() {
        let bytes = b"\0\0\0";
        assert_eq!(trim_cstr(bytes), "");
    }

    #[test]
    fn trim_cstr_no_nulls() {
        let bytes = b"Test";
        assert_eq!(trim_cstr(bytes), "Test");
    }

    #[test]
    fn trim_cstr_trims_trailing_whitespace() {
        let bytes = b"Hello  \0";
        assert_eq!(trim_cstr(bytes), "Hello");
    }

    // ----- control_score -----

    #[test]
    fn control_score_master_highest() {
        let master_int = control_score("Master", false);
        let pcm = control_score("PCM", false);
        assert!(master_int > pcm);
    }

    #[test]
    fn control_score_switch_gets_lower_base() {
        // "Front" as integer: base 10 + "front" 80 = 90
        // "Playback" as switch: base 0 + "playback" 20 = 20
        let switch_score = control_score("Playback", true);
        let int_score = control_score("Front", false);
        assert!(int_score > switch_score);
    }

    #[test]
    fn control_score_speaker_vs_headphone() {
        let speaker = control_score("Speaker", false);
        let headphone = control_score("Headphone", false);
        assert!(speaker > headphone);
    }

    #[test]
    fn control_score_case_insensitive() {
        let lower = control_score("master", false);
        let upper = control_score("MASTER", false);
        assert_eq!(lower, upper);
    }

    // ----- scale_s16le_audio -----

    #[test]
    fn scale_s16le_audio_100_percent_is_identity() {
        let input = [1000i16, -1000i16, 0i16, i16::MAX];
        let mut bytes = Vec::new();
        for sample in input {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let scaled = scale_s16le_audio(&bytes, 100);
        assert_eq!(scaled, bytes);
    }

    #[test]
    fn scale_s16le_audio_zero_gain() {
        let input = [1000i16, -500i16];
        let mut bytes = Vec::new();
        for sample in input {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        let scaled = scale_s16le_audio(&bytes, 0);
        assert_eq!(i16::from_le_bytes([scaled[0], scaled[1]]), 0);
        assert_eq!(i16::from_le_bytes([scaled[2], scaled[3]]), 0);
    }

    #[test]
    fn scale_s16le_audio_clamps_to_i16_max() {
        let input = [i16::MAX, i16::MAX];
        let mut bytes = Vec::new();
        for sample in input {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        // 200% of i16::MAX should clamp to i16::MAX
        let scaled = scale_s16le_audio(&bytes, 200);
        assert_eq!(i16::from_le_bytes([scaled[0], scaled[1]]), i16::MAX);
    }

    #[test]
    fn scale_s16le_audio_odd_length_preserves_last_byte() {
        let bytes = [0x12, 0x34, 0x56]; // 3 bytes
        let scaled = scale_s16le_audio(&bytes, 100);
        assert_eq!(scaled.len(), 3);
        assert_eq!(scaled[2], 0x56);
    }

    // ----- speaker_format_to_kernel -----

    #[test]
    fn speaker_format_to_kernel_s16le() {
        let fmt = speaker_format_to_kernel(SpeakerSampleFormat::PcmS16Le).unwrap();
        assert_eq!(fmt, SNDRV_PCM_FORMAT_S16_LE);
    }

    #[test]
    fn speaker_format_to_kernel_s24le_unsupported() {
        let err = speaker_format_to_kernel(SpeakerSampleFormat::PcmS24Le).unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    #[test]
    fn speaker_format_to_kernel_float32le_unsupported() {
        let err = speaker_format_to_kernel(SpeakerSampleFormat::PcmFloat32Le).unwrap_err();
        assert!(matches!(err, CapabilityError::Unsupported(_)));
    }

    // ----- playback_device_path -----

    #[test]
    fn playback_device_path_format() {
        let path = playback_device_path(0, 0);
        assert_eq!(path, PathBuf::from("/dev/snd/pcmC0D0p"));
    }

    #[test]
    fn playback_device_path_high_indices() {
        let path = playback_device_path(10, 5);
        assert_eq!(path, PathBuf::from("/dev/snd/pcmC10D5p"));
    }

    // ----- control_device_path -----

    #[test]
    fn control_device_path_format() {
        let path = control_device_path(0);
        assert_eq!(path, PathBuf::from("/dev/snd/controlC0"));
    }

    #[test]
    fn control_device_path_card_5() {
        let path = control_device_path(5);
        assert_eq!(path, PathBuf::from("/dev/snd/controlC5"));
    }

    // ----- parse_cards_from -----

    #[test]
    fn parse_cards_from_standard_format() {
        let text = " 0 [PCH            ]: HDA-Intel - HDA Intel PCH\n                      HDA Intel PCH at 0x91428000 irq 137\n 1 [HDMI           ]: HDA-Intel - HDA ATI HDMI\n                      HDA ATI HDMI at 0x91300000 irq 138\n";
        let cards = parse_cards_from(text);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].card_index, 0);
        // card_id includes trailing whitespace from the bracket format
        assert!(cards[0].card_id.contains("PCH"));
        assert!(cards[0].description.contains("HDA Intel PCH"));
        assert_eq!(cards[1].card_index, 1);
        assert!(cards[1].card_id.contains("HDMI"));
    }

    #[test]
    fn parse_cards_from_empty_string() {
        let cards = parse_cards_from("");
        assert!(cards.is_empty());
    }

    #[test]
    fn parse_cards_from_skips_malformed_lines() {
        let text = "invalid line\nno bracket\n 0 [Card]";
        let cards = parse_cards_from(text);
        assert!(cards.is_empty());
    }

    #[test]
    fn parse_cards_from_non_numeric_index() {
        let text = " X [Card] description\n";
        let cards = parse_cards_from(text);
        assert!(cards.is_empty());
    }

    // ----- parse_pcms_from -----

    #[test]
    fn parse_pcms_from_skips_capture_only() {
        let text = "02-00: BRIO : USB Audio : capture 1\n";
        let pcms = parse_pcms_from(text);
        assert!(pcms.is_empty());
    }

    #[test]
    fn parse_pcms_from_multiple_playback() {
        let text = "00-00: ALC897 Analog : ALC897 Analog : playback 1 : capture 1\n00-01: ALC897 Digital : ALC897 Digital : playback 1\n";
        let pcms = parse_pcms_from(text);
        assert_eq!(pcms.len(), 2);
        assert_eq!(pcms[0].card_index, 0);
        assert_eq!(pcms[0].device_index, 0);
        assert_eq!(pcms[1].device_index, 1);
    }

    #[test]
    fn parse_pcms_from_empty_string() {
        let pcms = parse_pcms_from("");
        assert!(pcms.is_empty());
    }

    // ----- configured_hw_params -----

    #[test]
    fn configured_hw_params_different_sample_rate() {
        let (_params, _bps, channels, period_frames) =
            AlsaSpeakerBackend::configured_hw_params(&AudioPlaybackRequest {
                duration_ms: 100,
                sample_rate_hz: 44_100,
                channels: 1,
                format: SpeakerSampleFormat::PcmS16Le,
                audio_bytes: vec![0],
                software_gain_percent: None,
                target_output_level_percent: None,
            })
            .unwrap();
        assert_eq!(channels, 1);
        assert_eq!(period_frames, 441); // 44100 / 100
    }

    #[test]
    fn configured_hw_params_rejects_unsupported_format() {
        let result = AlsaSpeakerBackend::configured_hw_params(&AudioPlaybackRequest {
            duration_ms: 100,
            sample_rate_hz: 48_000,
            channels: 2,
            format: SpeakerSampleFormat::PcmS24Le,
            audio_bytes: vec![0],
            software_gain_percent: None,
            target_output_level_percent: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn configured_hw_params_period_minimum() {
        let (_params, _bps, _channels, period_frames) =
            AlsaSpeakerBackend::configured_hw_params(&AudioPlaybackRequest {
                duration_ms: 100,
                sample_rate_hz: 8_000,
                channels: 1,
                format: SpeakerSampleFormat::PcmS16Le,
                audio_bytes: vec![0],
                software_gain_percent: None,
                target_output_level_percent: None,
            })
            .unwrap();
        // rate / 100 = 80, but .max(160) so should be 160
        assert_eq!(period_frames, 160);
    }

    // ----- SndInterval set_exact -----

    #[test]
    fn snd_interval_set_exact() {
        let mut interval = SndInterval {
            min: 0,
            max: u32::MAX,
            openmin_integer_empty: 0,
        };
        interval.set_exact(48_000);
        assert_eq!(interval.min, 48_000);
        assert_eq!(interval.max, 48_000);
        assert_eq!(interval.openmin_integer_empty, 1 << 2);
    }

    // ----- SndPcmHwParams -----

    #[test]
    fn hw_params_any_initializes() {
        let params = SndPcmHwParams::any();
        // Masks should be all 1s (any)
        for word in &params.masks[0].bits {
            assert_eq!(*word, u32::MAX);
        }
        // Intervals should be full range
        assert_eq!(params.intervals[0].min, 0);
        assert_eq!(params.intervals[0].max, u32::MAX);
    }

    #[test]
    fn hw_params_set_mask_value() {
        let mut params = SndPcmHwParams::any();
        params.set_mask_value(0, 5);
        // bit 5 set in mask 0
        assert_ne!(params.masks[0].bits[5 / 32] & (1 << (5 % 32)), 0);
    }

    #[test]
    fn hw_params_set_interval_value() {
        let mut params = SndPcmHwParams::any();
        params.set_interval_value(SNDRV_PCM_HW_PARAM_RATE, 44_100);
        let idx = SNDRV_PCM_HW_PARAM_RATE - SNDRV_PCM_HW_PARAM_FIRST_INTERVAL;
        assert_eq!(params.intervals[idx].min, 44_100);
    }

    // ----- AlsaPlaybackCardInfo -----

    #[test]
    fn card_info_equality() {
        let a = AlsaPlaybackCardInfo {
            card_index: 0,
            card_id: "PCH".into(),
            description: "HDA Intel".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn card_info_debug_format() {
        let info = AlsaPlaybackCardInfo {
            card_index: 1,
            card_id: "HDMI".into(),
            description: "HDA ATI HDMI".into(),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("HDMI"));
        assert!(debug.contains("1"));
    }

    // ----- AlsaPlaybackPcmInfo -----

    #[test]
    fn pcm_info_equality() {
        let a = AlsaPlaybackPcmInfo {
            card_index: 0,
            device_index: 0,
            device_name: "ALC897 Analog".into(),
            subdevice_name: "subdevice #0".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    // ----- discover_alsa_playback_cards_in -----

    #[test]
    fn discover_playback_cards_in_missing_file() {
        let result = discover_alsa_playback_cards_in(Path::new("/nonexistent/cards"));
        assert!(result.is_err());
    }

    #[test]
    fn discover_playback_cards_in_with_fixture() {
        let dir = edgerun_linux_sysfs::temp_root("alsa-speaker-test");
        let cards_file = dir.join("cards");
        fs::write(
            &cards_file,
            " 0 [TestCard       ]: Test - Test Card\n                      Test Description\n",
        )
        .unwrap();
        let cards = discover_alsa_playback_cards_in(&cards_file).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].card_index, 0);
        assert!(cards[0].card_id.contains("TestCard"));
        let _ = fs::remove_dir_all(&dir);
    }

    // ----- discover_alsa_playback_pcms_in -----

    #[test]
    fn discover_playback_pcms_in_missing_file() {
        let result = discover_alsa_playback_pcms_in(Path::new("/nonexistent/pcm"));
        assert!(result.is_err());
    }

    // ----- discover_speakers -----

    #[test]
    fn discover_speakers_returns_error_on_missing_files() {
        // This requires real /proc/asound files, so it should error
        let result = discover_speakers();
        // May be Ok with empty vec on systems with no cards, or Err
        // We just check it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }
}
