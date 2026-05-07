pub mod prelude {
    pub mod v1 {
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

#[cfg(not(unix))]
pub mod collections {
    pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
}

#[cfg(not(unix))]
pub mod fs {
    use crate::{io, path::PathBuf};

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

    pub struct ReadDir;
    pub struct DirEntry;

    impl Iterator for ReadDir {
        type Item = io::Result<DirEntry>;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl DirEntry {
        #[must_use]
        pub fn path(&self) -> PathBuf {
            PathBuf::new()
        }

        #[must_use]
        pub fn file_name(&self) -> PathBuf {
            PathBuf::new()
        }
    }

    pub fn read_dir<P>(_path: P) -> io::Result<ReadDir> {
        Err(io::Error::new(io::ErrorKind::NotFound))
    }
}

#[cfg(not(unix))]
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

        #[must_use]
        pub const fn raw_os_error(&self) -> Option<i32> {
            None
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

#[cfg(not(unix))]
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

    pub mod raw {
        #[allow(non_camel_case_types)]
        pub type c_int = i32;
        #[allow(non_camel_case_types)]
        pub type c_ulong = usize;
        #[allow(non_camel_case_types)]
        pub type c_void = core::ffi::c_void;
    }
}

#[cfg(not(unix))]
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

        #[must_use]
        pub fn exists(&self) -> bool {
            false
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PathBuf(String);

    impl PathBuf {
        #[must_use]
        pub fn new() -> Self {
            Self(String::new())
        }

        #[must_use]
        pub fn file_name(&self) -> Option<&str> {
            None
        }

        #[must_use]
        pub fn to_string_lossy(&self) -> String {
            self.0.clone()
        }
    }

    impl From<&str> for PathBuf {
        fn from(value: &str) -> Self {
            Self(value.to_string())
        }
    }

    impl fmt::Display for PathBuf {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.0)
        }
    }
}

#[cfg(not(unix))]
pub mod thread {
    use crate::time::Duration;

    pub fn sleep(_duration: Duration) {}
}

#[cfg(not(unix))]
pub mod time {
    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct SystemTimeError;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SystemTime(Duration);

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
    pub struct Instant(Duration);

    pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

    impl SystemTime {
        #[must_use]
        pub const fn now() -> Self {
            UNIX_EPOCH
        }

        pub fn duration_since(
            &self,
            earlier: SystemTime,
        ) -> core::result::Result<Duration, SystemTimeError> {
            self.0.checked_sub(earlier.0).ok_or(SystemTimeError)
        }
    }

    impl Instant {
        #[must_use]
        pub const fn now() -> Self {
            Self(Duration::from_secs(0))
        }

        #[must_use]
        pub fn elapsed(&self) -> Duration {
            Self::now().0.saturating_sub(self.0)
        }
    }

    impl core::ops::Add<Duration> for Instant {
        type Output = Self;

        fn add(self, rhs: Duration) -> Self::Output {
            Self(self.0.saturating_add(rhs))
        }
    }
}

#[cfg(not(unix))]
pub mod error {
    pub use core::error::*;
}

#[cfg(not(unix))]
pub mod option {
    pub use core::option::*;
}

#[cfg(not(unix))]
pub mod result {
    pub use core::result::*;
}

#[cfg(not(unix))]
pub mod string {
    pub use alloc::string::*;
}

#[cfg(not(unix))]
pub mod vec {
    pub use alloc::vec::*;
}

pub use core::{ptr, slice};

use crate::prelude::v1::*;
use edgerun_camera_biometrics::{
    default_face_biometric_state, validate_liveness_challenge, CameraBiometricError,
    CameraBiometricPurpose, CameraBiometricReader, CameraCapture, CameraCaptureQuality,
    CameraEnrollProgress, CameraEnrollmentSession, CameraFrame, CameraLivenessChallenge,
    CameraLivenessEvidence, CameraLivenessFrameObservation, CameraLivenessResult,
    CameraPixelFormat, CameraReaderInfo, CameraStreamRole, CameraTemplateRecord,
    CameraVerification, CameraVerifyRequest, PairedCameraBiometricReader, PairedCameraFrame,
};
use std::collections::HashMap;
#[cfg(unix)]
use std::fs;
use std::fs::File;
#[cfg(unix)]
use std::io;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong, c_void};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
const V4L2_CAP_VIDEO_CAPTURE: u32 = 0x0000_0001;
const V4L2_CAP_STREAMING: u32 = 0x0400_0000;
const V4L2_MEMORY_MMAP: u32 = 1;
const V4L2_PIX_FMT_MJPEG: u32 = fourcc(b'M', b'J', b'P', b'G');
const V4L2_PIX_FMT_YUYV: u32 = fourcc(b'Y', b'U', b'Y', b'V');
const V4L2_PIX_FMT_NV12: u32 = fourcc(b'N', b'V', b'1', b'2');
const V4L2_PIX_FMT_RGB24: u32 = fourcc(b'R', b'G', b'B', b'3');
const V4L2_PIX_FMT_GREY: u32 = fourcc(b'G', b'R', b'E', b'Y');
const V4L2_PIX_FMT_Y10: u32 = fourcc(b'Y', b'1', b'0', b' ');
const V4L2_PIX_FMT_Y12: u32 = fourcc(b'Y', b'1', b'2', b' ');
const V4L2_PIX_FMT_Y16: u32 = fourcc(b'Y', b'1', b'6', b' ');

const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
const IOC_READ: u32 = 2;
const IOC_WRITE: u32 = 1;
const V4L2_IOC_TYPE: u8 = b'V';

const fn ioc(dir: u32, ty: u8, nr: u8, size: usize) -> u32 {
    (dir << IOC_DIRSHIFT)
        | ((ty as u32) << IOC_TYPESHIFT)
        | ((nr as u32) << IOC_NRSHIFT)
        | ((size as u32) << IOC_SIZESHIFT)
}
const fn iowr<T>(ty: u8, nr: u8) -> u32 {
    ioc(IOC_READ | IOC_WRITE, ty, nr, core::mem::size_of::<T>())
}
const fn ior<T>(ty: u8, nr: u8) -> u32 {
    ioc(IOC_READ, ty, nr, core::mem::size_of::<T>())
}

const VIDIOC_QUERYCAP: c_ulong = ior::<V4l2Capability>(V4L2_IOC_TYPE, 0) as c_ulong;
// v4l2_format's ioctl size depends on the largest union member/alignment, not just our
// simplified Rust placeholder layout. Use the kernel-header request value directly.
const VIDIOC_G_FMT: c_ulong = 0xc0d0_5604;
const VIDIOC_S_FMT: c_ulong = 0xc0d0_5605;
const VIDIOC_ENUM_FMT: c_ulong = iowr::<V4l2Fmtdesc>(V4L2_IOC_TYPE, 2) as c_ulong;
const VIDIOC_REQBUFS: c_ulong = iowr::<V4l2RequestBuffers>(V4L2_IOC_TYPE, 8) as c_ulong;
const VIDIOC_QUERYBUF: c_ulong = iowr::<V4l2Buffer>(V4L2_IOC_TYPE, 9) as c_ulong;
const VIDIOC_QBUF: c_ulong = iowr::<V4l2Buffer>(V4L2_IOC_TYPE, 15) as c_ulong;
const VIDIOC_DQBUF: c_ulong = iowr::<V4l2Buffer>(V4L2_IOC_TYPE, 17) as c_ulong;
const VIDIOC_STREAMON: c_ulong = 0x4004_5612;
const VIDIOC_STREAMOFF: c_ulong = 0x4004_5613;

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: isize,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, length: usize) -> c_int;
}

const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const MAP_SHARED: c_int = 0x01;
const MAP_FAILED: *mut c_void = !0usize as *mut c_void;

const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

#[repr(C)]
struct V4l2Capability {
    driver: [u8; 16],
    card: [u8; 32],
    bus_info: [u8; 32],
    version: u32,
    capabilities: u32,
    device_caps: u32,
    reserved: [u32; 3],
}

#[repr(C)]
struct V4l2Format {
    type_: u32,
    reserved_alignment: u32,
    raw_data: [u8; 200],
}

#[repr(C)]
struct V4l2Fmtdesc {
    index: u32,
    type_: u32,
    flags: u32,
    description: [u8; 32],
    pixelformat: u32,
    reserved: [u32; 4],
}

#[repr(C)]
struct V4l2RequestBuffers {
    count: u32,
    type_: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct V4l2Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct V4l2Timecode {
    type_: u32,
    flags: u32,
    frames: u8,
    seconds: u8,
    minutes: u8,
    hours: u8,
    userbits: [u8; 4],
}

#[repr(C)]
union V4l2BufferMemory {
    offset: u32,
    userptr: usize,
    planes: usize,
    fd: i32,
}

#[repr(C)]
struct V4l2Buffer {
    index: u32,
    type_: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    timestamp: V4l2Timeval,
    timecode: V4l2Timecode,
    sequence: u32,
    memory: u32,
    m: V4l2BufferMemory,
    length: u32,
    reserved2: u32,
    request_fd: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2CameraInfo {
    pub devnode: PathBuf,
    pub driver: String,
    pub card: String,
    pub bus_info: String,
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub infrared: V4l2InfraredCapability,
}

impl V4l2CameraInfo {
    pub fn supports_video_capture(&self) -> bool {
        (self.capabilities & V4L2_CAP_VIDEO_CAPTURE != 0)
            || (self.device_caps & V4L2_CAP_VIDEO_CAPTURE != 0)
    }

    pub fn supports_streaming(&self) -> bool {
        (self.capabilities & V4L2_CAP_STREAMING != 0)
            || (self.device_caps & V4L2_CAP_STREAMING != 0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2FrameFormat {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub image_size: u32,
    pub pixel_format: CameraPixelFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2FormatDescription {
    pub pixel_format: CameraPixelFormat,
    pub raw_pixel_format: u32,
    pub description: String,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V4l2StreamRole {
    Unknown,
    RgbLikely,
    InfraredLikely,
    DepthLikely,
    MonochromeLikely,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V4l2InfraredCapability {
    Unknown,
    NotDetected,
    MonochromeLikely,
    InfraredLikely,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2CameraProbe {
    pub info: V4l2CameraInfo,
    pub current_format: V4l2FrameFormat,
    pub available_formats: Vec<V4l2FormatDescription>,
    pub stream_role: V4l2StreamRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2CameraGroup {
    pub key: String,
    pub card: String,
    pub bus_info: String,
    pub devices: Vec<V4l2CameraProbe>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2PairedCameraSelection {
    pub group_key: String,
    pub rgb: Option<V4l2CameraProbe>,
    pub infrared: Option<V4l2CameraProbe>,
    pub depth: Option<V4l2CameraProbe>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V4l2CameraError {
    Io(String),
    Parse(String),
    Unsupported(&'static str),
    Camera(CameraBiometricError),
}

impl core::fmt::Display for V4l2CameraError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(msg) => f.write_str(msg),
            Self::Parse(msg) => f.write_str(msg),
            Self::Unsupported(msg) => write!(f, "unsupported V4L2 camera operation: {msg}"),
            Self::Camera(err) => write!(f, "camera biometric error: {err}"),
        }
    }
}

impl std::error::Error for V4l2CameraError {}

impl From<CameraBiometricError> for V4l2CameraError {
    fn from(value: CameraBiometricError) -> Self {
        Self::Camera(value)
    }
}

pub fn parse_c_string(bytes: &[u8]) -> String {
    edgerun_encoding::cstring::decode_c_string_trimmed(bytes).unwrap_or_default()
}

fn pixel_format_from_v4l2(value: u32) -> CameraPixelFormat {
    match value {
        V4L2_PIX_FMT_MJPEG => CameraPixelFormat::Mjpeg,
        V4L2_PIX_FMT_YUYV => CameraPixelFormat::Yuyv,
        V4L2_PIX_FMT_NV12 => CameraPixelFormat::Nv12,
        V4L2_PIX_FMT_RGB24 => CameraPixelFormat::Rgb24,
        V4L2_PIX_FMT_GREY | V4L2_PIX_FMT_Y10 | V4L2_PIX_FMT_Y12 | V4L2_PIX_FMT_Y16 => {
            CameraPixelFormat::Gray8
        }
        other => CameraPixelFormat::Other(other),
    }
}

fn default_capture_quality() -> CameraCaptureQuality {
    CameraCaptureQuality::Good
}

fn ioctl_errno_to_string() -> String {
    io::Error::last_os_error().to_string()
}

fn infer_infrared_capability(
    card_name: &str,
    pixel_format: CameraPixelFormat,
) -> V4l2InfraredCapability {
    let lowered = card_name.to_ascii_lowercase();
    if lowered.contains("ir") || lowered.contains("infrared") || lowered.contains("depth") {
        return V4l2InfraredCapability::InfraredLikely;
    }
    #[allow(clippy::redundant_guards)]
    match pixel_format {
        CameraPixelFormat::Gray8 => V4l2InfraredCapability::MonochromeLikely,
        CameraPixelFormat::Other(v)
            if matches!(v, V4L2_PIX_FMT_Y10 | V4L2_PIX_FMT_Y12 | V4L2_PIX_FMT_Y16) =>
        {
            V4l2InfraredCapability::InfraredLikely
        }
        CameraPixelFormat::Mjpeg
        | CameraPixelFormat::Yuyv
        | CameraPixelFormat::Nv12
        | CameraPixelFormat::Rgb24 => V4l2InfraredCapability::NotDetected,
        CameraPixelFormat::Other(_) => V4l2InfraredCapability::Unknown,
    }
}

fn infer_stream_role(card_name: &str, pixel_format: CameraPixelFormat) -> V4l2StreamRole {
    let lowered = card_name.to_ascii_lowercase();
    if lowered.contains("depth") {
        return V4l2StreamRole::DepthLikely;
    }
    if lowered.contains("ir") || lowered.contains("infrared") {
        return V4l2StreamRole::InfraredLikely;
    }
    #[allow(clippy::redundant_guards)]
    match pixel_format {
        CameraPixelFormat::Gray8 => V4l2StreamRole::MonochromeLikely,
        CameraPixelFormat::Mjpeg
        | CameraPixelFormat::Yuyv
        | CameraPixelFormat::Nv12
        | CameraPixelFormat::Rgb24 => V4l2StreamRole::RgbLikely,
        CameraPixelFormat::Other(v)
            if matches!(v, V4L2_PIX_FMT_Y10 | V4L2_PIX_FMT_Y12 | V4L2_PIX_FMT_Y16) =>
        {
            V4l2StreamRole::InfraredLikely
        }
        CameraPixelFormat::Other(_) => V4l2StreamRole::Unknown,
    }
}

fn infer_infrared_from_formats(
    card_name: &str,
    formats: &[V4l2FormatDescription],
) -> V4l2InfraredCapability {
    if formats.is_empty() {
        return V4l2InfraredCapability::Unknown;
    }
    let mut inferred = V4l2InfraredCapability::NotDetected;
    for fmt in formats {
        let capability = infer_infrared_capability(card_name, fmt.pixel_format);
        match capability {
            V4l2InfraredCapability::InfraredLikely => return capability,
            V4l2InfraredCapability::MonochromeLikely => inferred = capability,
            V4l2InfraredCapability::NotDetected | V4l2InfraredCapability::Unknown => {}
        }
    }
    inferred
}

fn infer_stream_role_from_formats(
    card_name: &str,
    formats: &[V4l2FormatDescription],
) -> V4l2StreamRole {
    if formats.is_empty() {
        return V4l2StreamRole::Unknown;
    }
    let mut inferred = V4l2StreamRole::Unknown;
    for fmt in formats {
        let role = infer_stream_role(card_name, fmt.pixel_format);
        match role {
            V4l2StreamRole::DepthLikely | V4l2StreamRole::InfraredLikely => return role,
            V4l2StreamRole::MonochromeLikely => inferred = role,
            V4l2StreamRole::RgbLikely if matches!(inferred, V4l2StreamRole::Unknown) => {
                inferred = role
            }
            V4l2StreamRole::Unknown | V4l2StreamRole::RgbLikely => {}
        }
    }
    inferred
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4l2CameraDevice {
    pub devnode: PathBuf,
}

impl V4l2CameraDevice {
    pub fn new(devnode: impl Into<PathBuf>) -> Self {
        Self {
            devnode: devnode.into(),
        }
    }

    pub fn open(&self) -> Result<File, V4l2CameraError> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.devnode)
            .map_err(|e| V4l2CameraError::Io(e.to_string()))
    }

    pub fn capture_frame(&self, timeout_ms: u32) -> Result<CameraFrame, V4l2CameraError> {
        let info = self.query_info()?;
        if !info.supports_streaming() {
            return Err(V4l2CameraError::Unsupported(
                "device does not advertise streaming capture",
            ));
        }
        let format = self.normalized_format_for_streaming(self.current_format()?)?;
        let file = self.open()?;
        let fd = file.as_raw_fd();

        let mut req = V4l2RequestBuffers {
            count: 2,
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            capabilities: 0,
            flags: 0,
            reserved: [0; 3],
        };
        let rc = unsafe { ioctl(fd, VIDIOC_REQBUFS, &mut req) };
        if rc < 0 {
            return Err(V4l2CameraError::Io(ioctl_errno_to_string()));
        }
        if req.count == 0 {
            return Err(V4l2CameraError::Unsupported(
                "device did not provide capture buffers",
            ));
        }

        let mut mapped: Vec<(*mut c_void, usize)> = Vec::new();
        for index in 0..req.count.min(2) {
            let mut buf = V4l2Buffer {
                index,
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                bytesused: 0,
                flags: 0,
                field: 0,
                timestamp: V4l2Timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                timecode: V4l2Timecode {
                    type_: 0,
                    flags: 0,
                    frames: 0,
                    seconds: 0,
                    minutes: 0,
                    hours: 0,
                    userbits: [0; 4],
                },
                sequence: 0,
                memory: V4L2_MEMORY_MMAP,
                m: V4l2BufferMemory { offset: 0 },
                length: 0,
                reserved2: 0,
                request_fd: 0,
            };
            let rc = unsafe { ioctl(fd, VIDIOC_QUERYBUF, &mut buf) };
            if rc < 0 {
                return Err(V4l2CameraError::Io(ioctl_errno_to_string()));
            }
            let offset = unsafe { buf.m.offset } as isize;
            let ptr = unsafe {
                mmap(
                    ptr::null_mut(),
                    buf.length as usize,
                    PROT_READ | PROT_WRITE,
                    MAP_SHARED,
                    fd,
                    offset,
                )
            };
            if ptr == MAP_FAILED {
                return Err(V4l2CameraError::Io(ioctl_errno_to_string()));
            }
            mapped.push((ptr, buf.length as usize));
            let rc = unsafe { ioctl(fd, VIDIOC_QBUF, &mut buf) };
            if rc < 0 {
                for (ptr, len) in &mapped {
                    unsafe {
                        let _ = munmap(*ptr, *len);
                    }
                }
                return Err(V4l2CameraError::Io(ioctl_errno_to_string()));
            }
        }

        let mut ty = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        let rc = unsafe { ioctl(fd, VIDIOC_STREAMON, &mut ty) };
        if rc < 0 {
            for (ptr, len) in &mapped {
                unsafe {
                    let _ = munmap(*ptr, *len);
                }
            }
            return Err(V4l2CameraError::Io(ioctl_errno_to_string()));
        }

        let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1) as u64);
        let result = loop {
            let mut buf = V4l2Buffer {
                index: 0,
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                bytesused: 0,
                flags: 0,
                field: 0,
                timestamp: V4l2Timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                timecode: V4l2Timecode {
                    type_: 0,
                    flags: 0,
                    frames: 0,
                    seconds: 0,
                    minutes: 0,
                    hours: 0,
                    userbits: [0; 4],
                },
                sequence: 0,
                memory: V4L2_MEMORY_MMAP,
                m: V4l2BufferMemory { offset: 0 },
                length: 0,
                reserved2: 0,
                request_fd: 0,
            };
            let rc = unsafe { ioctl(fd, VIDIOC_DQBUF, &mut buf) };
            if rc == 0 {
                let index = buf.index as usize;
                if let Some((ptr, len)) = mapped.get(index) {
                    let bytesused = usize::try_from(buf.bytesused).unwrap_or(0).min(*len);
                    let bytes =
                        unsafe { slice::from_raw_parts(*ptr as *const u8, bytesused) }.to_vec();
                    break Ok(CameraFrame {
                        width: format.width,
                        height: format.height,
                        stride: format.stride,
                        format: format.pixel_format,
                        bytes,
                    });
                }
                break Err(V4l2CameraError::Parse(
                    "driver returned out-of-range buffer index".into(),
                ));
            }
            let err = io::Error::last_os_error();
            if matches!(err.raw_os_error(), Some(11)) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            break Err(V4l2CameraError::Io(err.to_string()));
        };

        let _ = unsafe { ioctl(fd, VIDIOC_STREAMOFF, &mut ty) };
        for (ptr, len) in &mapped {
            unsafe {
                let _ = munmap(*ptr, *len);
            }
        }
        let mut req = V4l2RequestBuffers {
            count: 0,
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            capabilities: 0,
            flags: 0,
            reserved: [0; 3],
        };
        let _ = unsafe { ioctl(fd, VIDIOC_REQBUFS, &mut req) };
        result
    }

    pub fn query_info(&self) -> Result<V4l2CameraInfo, V4l2CameraError> {
        let file = self.open()?;
        let mut cap = V4l2Capability {
            driver: [0; 16],
            card: [0; 32],
            bus_info: [0; 32],
            version: 0,
            capabilities: 0,
            device_caps: 0,
            reserved: [0; 3],
        };
        let rc = unsafe { ioctl(file.as_raw_fd(), VIDIOC_QUERYCAP, &mut cap) };
        if rc < 0 {
            return Err(V4l2CameraError::Io(io::Error::last_os_error().to_string()));
        }
        Ok(V4l2CameraInfo {
            devnode: self.devnode.clone(),
            driver: parse_c_string(&cap.driver),
            card: parse_c_string(&cap.card),
            bus_info: parse_c_string(&cap.bus_info),
            version: cap.version,
            capabilities: cap.capabilities,
            device_caps: cap.device_caps,
            infrared: V4l2InfraredCapability::Unknown,
        })
    }

    pub fn current_format(&self) -> Result<V4l2FrameFormat, V4l2CameraError> {
        let file = self.open()?;
        let mut fmt = V4l2Format {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            reserved_alignment: 0,
            raw_data: [0; 200],
        };
        let rc = unsafe { ioctl(file.as_raw_fd(), VIDIOC_G_FMT, &mut fmt) };
        if rc < 0 {
            return Err(V4l2CameraError::Io(io::Error::last_os_error().to_string()));
        }
        let width = u32::from_ne_bytes(fmt.raw_data[0..4].try_into().unwrap());
        let height = u32::from_ne_bytes(fmt.raw_data[4..8].try_into().unwrap());
        let pixelformat = u32::from_ne_bytes(fmt.raw_data[8..12].try_into().unwrap());
        let bytesperline = u32::from_ne_bytes(fmt.raw_data[16..20].try_into().unwrap());
        let sizeimage = u32::from_ne_bytes(fmt.raw_data[20..24].try_into().unwrap());
        Ok(V4l2FrameFormat {
            width,
            height,
            stride: bytesperline,
            image_size: sizeimage,
            pixel_format: pixel_format_from_v4l2(pixelformat),
        })
    }

    fn set_current_format(
        &self,
        format: &V4l2FrameFormat,
    ) -> Result<V4l2FrameFormat, V4l2CameraError> {
        let file = self.open()?;
        let mut fmt = V4l2Format {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            reserved_alignment: 0,
            raw_data: [0; 200],
        };
        let raw_pixel_format = match format.pixel_format {
            CameraPixelFormat::Mjpeg => V4L2_PIX_FMT_MJPEG,
            CameraPixelFormat::Yuyv => V4L2_PIX_FMT_YUYV,
            CameraPixelFormat::Nv12 => V4L2_PIX_FMT_NV12,
            CameraPixelFormat::Rgb24 => V4L2_PIX_FMT_RGB24,
            CameraPixelFormat::Gray8 => V4L2_PIX_FMT_GREY,
            CameraPixelFormat::Other(v) => v,
        };
        fmt.raw_data[0..4].copy_from_slice(&format.width.to_ne_bytes());
        fmt.raw_data[4..8].copy_from_slice(&format.height.to_ne_bytes());
        fmt.raw_data[8..12].copy_from_slice(&raw_pixel_format.to_ne_bytes());
        fmt.raw_data[12..16].copy_from_slice(&1u32.to_ne_bytes());
        fmt.raw_data[16..20].copy_from_slice(&format.stride.to_ne_bytes());
        fmt.raw_data[20..24].copy_from_slice(&format.image_size.to_ne_bytes());
        let rc = unsafe { ioctl(file.as_raw_fd(), VIDIOC_S_FMT, &mut fmt) };
        if rc < 0 {
            return Err(V4l2CameraError::Io(ioctl_errno_to_string()));
        }
        let width = u32::from_ne_bytes(fmt.raw_data[0..4].try_into().unwrap());
        let height = u32::from_ne_bytes(fmt.raw_data[4..8].try_into().unwrap());
        let pixelformat = u32::from_ne_bytes(fmt.raw_data[8..12].try_into().unwrap());
        let bytesperline = u32::from_ne_bytes(fmt.raw_data[16..20].try_into().unwrap());
        let sizeimage = u32::from_ne_bytes(fmt.raw_data[20..24].try_into().unwrap());
        Ok(V4l2FrameFormat {
            width,
            height,
            stride: bytesperline,
            image_size: sizeimage,
            pixel_format: pixel_format_from_v4l2(pixelformat),
        })
    }

    fn normalized_format_for_streaming(
        &self,
        format: V4l2FrameFormat,
    ) -> Result<V4l2FrameFormat, V4l2CameraError> {
        let expected_size = match format.pixel_format {
            CameraPixelFormat::Gray8 => format.stride.saturating_mul(format.height),
            _ => format.image_size,
        };
        if expected_size == 0 || expected_size == format.image_size {
            return Ok(format);
        }
        self.set_current_format(&V4l2FrameFormat {
            image_size: expected_size,
            ..format
        })
    }

    pub fn enumerate_formats(&self) -> Result<Vec<V4l2FormatDescription>, V4l2CameraError> {
        let file = self.open()?;
        let mut formats = Vec::new();
        for index in 0u32..64 {
            let mut desc = V4l2Fmtdesc {
                index,
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                flags: 0,
                description: [0; 32],
                pixelformat: 0,
                reserved: [0; 4],
            };
            let rc = unsafe { ioctl(file.as_raw_fd(), VIDIOC_ENUM_FMT, &mut desc) };
            if rc < 0 {
                let err = io::Error::last_os_error();
                if matches!(err.raw_os_error(), Some(22)) {
                    break;
                }
                return Err(V4l2CameraError::Io(err.to_string()));
            }
            formats.push(V4l2FormatDescription {
                pixel_format: pixel_format_from_v4l2(desc.pixelformat),
                raw_pixel_format: desc.pixelformat,
                description: parse_c_string(&desc.description),
                flags: desc.flags,
            });
        }
        Ok(formats)
    }

    pub fn probe(&self) -> Result<V4l2CameraProbe, V4l2CameraError> {
        let mut info = self.query_info()?;
        let current_format = self.current_format()?;
        let formats = self.enumerate_formats().unwrap_or_default();
        info.infrared = if formats.is_empty() {
            infer_infrared_capability(&info.card, current_format.pixel_format)
        } else {
            infer_infrared_from_formats(&info.card, &formats)
        };
        let stream_role = if formats.is_empty() {
            infer_stream_role(&info.card, current_format.pixel_format)
        } else {
            infer_stream_role_from_formats(&info.card, &formats)
        };
        Ok(V4l2CameraProbe {
            info,
            current_format,
            available_formats: formats,
            stream_role,
        })
    }
}

pub fn discover_camera_devices() -> Result<Vec<V4l2CameraDevice>, V4l2CameraError> {
    discover_camera_devices_in(Path::new("/dev"))
}

pub fn discover_camera_devices_in(root: &Path) -> Result<Vec<V4l2CameraDevice>, V4l2CameraError> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| V4l2CameraError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| V4l2CameraError::Io(e.to_string()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("video") {
            out.push(V4l2CameraDevice::new(entry.path()));
        }
    }
    out.sort_by(|a, b| a.devnode.cmp(&b.devnode));
    Ok(out)
}

pub fn probe_camera_devices() -> Result<Vec<V4l2CameraProbe>, V4l2CameraError> {
    let mut out = Vec::new();
    for device in discover_camera_devices()? {
        if let Ok(probe) = device.probe() {
            out.push(probe);
        }
    }
    Ok(out)
}

pub fn group_camera_probes(probes: Vec<V4l2CameraProbe>) -> Vec<V4l2CameraGroup> {
    let mut groups: Vec<V4l2CameraGroup> = Vec::new();
    for probe in probes {
        let key = if !probe.info.bus_info.is_empty() {
            format!("{}::{}", probe.info.bus_info, probe.info.card)
        } else {
            probe.info.card.clone()
        };
        if let Some(group) = groups.iter_mut().find(|g| g.key == key) {
            group.devices.push(probe);
            continue;
        }
        groups.push(V4l2CameraGroup {
            key,
            card: probe.info.card.clone(),
            bus_info: probe.info.bus_info.clone(),
            devices: vec![probe],
        });
    }
    groups.sort_by(|a, b| a.key.cmp(&b.key));
    for group in &mut groups {
        group
            .devices
            .sort_by(|a, b| a.info.devnode.cmp(&b.info.devnode));
    }
    groups
}

pub fn select_paired_camera(groups: &[V4l2CameraGroup]) -> Option<V4l2PairedCameraSelection> {
    groups.iter().find_map(select_paired_camera_from_group)
}

pub fn select_paired_camera_from_group(
    group: &V4l2CameraGroup,
) -> Option<V4l2PairedCameraSelection> {
    let mut rgb = None;
    let mut infrared = None;
    let mut depth = None;
    for probe in &group.devices {
        match probe.stream_role {
            V4l2StreamRole::RgbLikely if rgb.is_none() => rgb = Some(probe.clone()),
            V4l2StreamRole::InfraredLikely | V4l2StreamRole::MonochromeLikely
                if infrared.is_none() =>
            {
                infrared = Some(probe.clone())
            }
            V4l2StreamRole::DepthLikely if depth.is_none() => depth = Some(probe.clone()),
            V4l2StreamRole::Unknown
            | V4l2StreamRole::RgbLikely
            | V4l2StreamRole::InfraredLikely
            | V4l2StreamRole::DepthLikely
            | V4l2StreamRole::MonochromeLikely => {}
        }
    }
    if rgb.is_none() && infrared.is_none() && depth.is_none() {
        return None;
    }
    Some(V4l2PairedCameraSelection {
        group_key: group.key.clone(),
        rgb,
        infrared,
        depth,
    })
}

pub struct V4l2CameraBiometricReader {
    device: V4l2CameraDevice,
    /// Stored face templates: template_id -> grayscale face template bytes (64x64 = 4096 bytes)
    templates: HashMap<String, Vec<u8>>,
    /// Active enrollment sessions: session_id -> (session, accumulated_samples)
    enrollment_sessions: HashMap<String, (CameraEnrollmentSession, Vec<Vec<u8>>)>,
}

/// Target face template size (64x64 grayscale = 4096 bytes)
const FACE_TEMPLATE_SIZE: usize = 64 * 64;
/// Threshold for face matching (normalized cross-correlation, 0-1 range)
const FACE_MATCH_THRESHOLD: f64 = 0.75;

impl V4l2CameraBiometricReader {
    pub fn new(device: V4l2CameraDevice) -> Self {
        Self {
            device,
            templates: HashMap::new(),
            enrollment_sessions: HashMap::new(),
        }
    }

    pub fn device(&self) -> &V4l2CameraDevice {
        &self.device
    }

    pub fn probe_infrared(&self) -> Result<V4l2InfraredCapability, V4l2CameraError> {
        Ok(self.device.probe()?.info.infrared)
    }

    pub fn probe_stream_role(&self) -> Result<V4l2StreamRole, V4l2CameraError> {
        Ok(self.device.probe()?.stream_role)
    }
}

impl CameraBiometricReader for V4l2CameraBiometricReader {
    fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError> {
        let info = self
            .device
            .probe()
            .map_err(|e| CameraBiometricError::Provider(e.to_string()))?
            .info;
        Ok(CameraReaderInfo {
            provider: "v4l2-camera".into(),
            reader_name: info.card,
            supports_face_detection: !matches!(
                info.infrared,
                V4l2InfraredCapability::InfraredLikely
            ),
            supports_face_matching: true, // We now support face matching
            supports_liveness_detection: matches!(
                info.infrared,
                V4l2InfraredCapability::InfraredLikely | V4l2InfraredCapability::MonochromeLikely
            ),
            hardware_protected_match: false,
        })
    }

    fn capture(
        &mut self,
        _purpose: CameraBiometricPurpose,
        timeout_ms: u32,
    ) -> Result<CameraCapture, CameraBiometricError> {
        let frame = self
            .device
            .capture_frame(timeout_ms)
            .map_err(|e| CameraBiometricError::Provider(e.to_string()))?;
        // Try to detect face in the captured frame
        let face_detected = extract_face_region(&frame).is_some();
        Ok(CameraCapture {
            frame,
            quality: default_capture_quality(),
            face_bounds: None, // We detect faces but don't report bounds yet
            state: default_face_biometric_state(false, false, face_detected),
        })
    }

    fn begin_enrollment(
        &mut self,
        request: &edgerun_camera_biometrics::CameraEnrollRequest,
    ) -> Result<CameraEnrollmentSession, CameraBiometricError> {
        edgerun_camera_biometrics::validate_camera_enroll_request(request)?;
        let session = CameraEnrollmentSession {
            session_id: request.label.clone(),
            label: request.label.clone(),
            samples_required: request.samples_required,
            samples_collected: 0,
            require_liveness: request.require_liveness,
            require_hardware_match: request.require_hardware_match,
        };
        // Initialize enrollment session
        self.enrollment_sessions
            .insert(request.label.clone(), (session.clone(), Vec::new()));
        Ok(session)
    }

    fn enroll_step(
        &mut self,
        session_id: &str,
        capture: &CameraCapture,
    ) -> Result<CameraEnrollProgress, CameraBiometricError> {
        let session = self
            .enrollment_sessions
            .get_mut(session_id)
            .ok_or_else(|| CameraBiometricError::Provider("no active enrollment session".into()))?;

        // Extract face region from the capture
        if let Some(face_gray) = extract_face_region(&capture.frame) {
            session.1.push(face_gray);
        }

        let samples_collected = session.1.len() as u8;
        let complete = samples_collected >= session.0.samples_required;

        Ok(CameraEnrollProgress {
            session: CameraEnrollmentSession {
                session_id: session_id.to_string(),
                label: session.0.label.clone(),
                samples_required: session.0.samples_required,
                samples_collected,
                require_liveness: session.0.require_liveness,
                require_hardware_match: session.0.require_hardware_match,
            },
            complete,
            template_id: if complete {
                Some(session_id.to_string())
            } else {
                None
            },
            last_quality: capture.quality,
            face_detected: !session.1.is_empty(),
            liveness_detected: false,
        })
    }

    fn finish_enrollment(
        &mut self,
        session_id: &str,
    ) -> Result<CameraTemplateRecord, CameraBiometricError> {
        let (_, samples) = self
            .enrollment_sessions
            .remove(session_id)
            .ok_or_else(|| CameraBiometricError::Provider("no active enrollment session".into()))?;

        if samples.is_empty() {
            return Err(CameraBiometricError::Provider(
                "no face samples collected during enrollment".into(),
            ));
        }

        // Average the face templates
        let template = average_face_templates(&samples);
        self.templates.insert(session_id.to_string(), template);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        Ok(CameraTemplateRecord {
            template_id: session_id.to_string(),
            label: session_id.to_string(),
            enrolled_at_unix_ms: now,
            last_verified_unix_ms: None,
        })
    }

    fn verify_capture(
        &mut self,
        request: &CameraVerifyRequest,
        capture: &CameraCapture,
    ) -> Result<CameraVerification, CameraBiometricError> {
        if let Some(face_gray) = extract_face_region(&capture.frame) {
            // Compare against all stored templates
            let mut best_match_score = 0.0f64;
            let mut best_template_id: Option<String> = None;

            for (template_id, template) in &self.templates {
                // If request specifies specific templates, only check those
                if !request.allowed_template_ids.is_empty()
                    && !request.allowed_template_ids.contains(template_id)
                {
                    continue;
                }

                let score = normalized_cross_correlation(&face_gray, template);
                if score > best_match_score {
                    best_match_score = score;
                    best_template_id = Some(template_id.clone());
                }
            }

            let matched = best_match_score >= FACE_MATCH_THRESHOLD;

            if matched {
                // Update last_verified timestamp
                if let Some(ref _tid) = best_template_id {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    // Note: we'd need mutable access to templates to update this,
                    // but for now we just report the match
                    let _ = now;
                }
            }

            return Ok(CameraVerification {
                matched,
                template_id: best_template_id,
                liveness_detected: false,
                face_detected: true,
                state: default_face_biometric_state(matched, false, true),
            });
        }

        // No face detected
        Ok(CameraVerification {
            matched: false,
            template_id: None,
            liveness_detected: false,
            face_detected: false,
            state: default_face_biometric_state(false, false, false),
        })
    }

    fn list_templates(&self) -> Result<Vec<CameraTemplateRecord>, CameraBiometricError> {
        Ok(self
            .templates
            .keys()
            .map(|id| CameraTemplateRecord {
                template_id: id.clone(),
                label: id.clone(),
                enrolled_at_unix_ms: 0, // We don't track this separately
                last_verified_unix_ms: None,
            })
            .collect())
    }

    fn delete_template(&mut self, template_id: &str) -> Result<(), CameraBiometricError> {
        if self.templates.remove(template_id).is_some() {
            Ok(())
        } else {
            Err(CameraBiometricError::Provider(format!(
                "template '{}' not found",
                template_id
            )))
        }
    }
}

// ============================================================================
// Face extraction and matching utilities
// ============================================================================

/// Extract a grayscale face region from a camera frame.
/// Uses a center-crop heuristic: assumes the face is roughly in the center
/// of the frame and extracts a 64x64 region.
fn extract_face_region(frame: &CameraFrame) -> Option<Vec<u8>> {
    // Convert frame to grayscale and extract center 64x64 region
    let gray = convert_to_grayscale(frame);
    if gray.len() < FACE_TEMPLATE_SIZE {
        return None;
    }

    // Extract center region
    let face_w = 64u32;
    let face_h = 64u32;
    let frame_w = frame.width;
    let frame_h = frame.height;

    if frame_w < face_w || frame_h < face_h {
        return None;
    }

    let start_x = (frame_w - face_w) / 2;
    let start_y = (frame_h - face_h) / 2;

    let mut face = vec![0u8; (face_w * face_h) as usize];
    for y in 0..face_h {
        let src_row = ((start_y + y) * frame.stride + start_x) as usize;
        let dst_row = (y * face_w) as usize;
        face[dst_row..dst_row + face_w as usize]
            .copy_from_slice(&gray[src_row..src_row + face_w as usize]);
    }

    Some(face)
}

/// Convert a camera frame to grayscale.
fn convert_to_grayscale(frame: &CameraFrame) -> Vec<u8> {
    match frame.format {
        CameraPixelFormat::Gray8 => {
            // Already grayscale
            frame.bytes.clone()
        }
        CameraPixelFormat::Rgb24 => {
            // RGB24 to grayscale: Y = 0.299*R + 0.587*G + 0.114*B
            let mut gray = Vec::with_capacity((frame.width * frame.height) as usize);
            for chunk in frame.bytes.chunks_exact(3) {
                let r = chunk[0] as f64;
                let g = chunk[1] as f64;
                let b = chunk[2] as f64;
                let y = round_f64(0.299 * r + 0.587 * g + 0.114 * b) as u8;
                gray.push(y);
            }
            gray
        }
        CameraPixelFormat::Yuyv => {
            // YUYV to grayscale: extract Y values
            let mut gray = Vec::with_capacity((frame.width * frame.height) as usize);
            for chunk in frame.bytes.chunks_exact(4) {
                // Y0 U0 Y1 V0 -> Y0, Y1
                gray.push(chunk[0]);
                gray.push(chunk[2]);
            }
            gray
        }
        CameraPixelFormat::Mjpeg => {
            // MJPEG can't be easily decoded without a JPEG decoder
            // Return as-is (will be poor quality matching)
            frame.bytes.clone()
        }
        CameraPixelFormat::Nv12 => {
            // NV12: Y plane followed by interleaved UV
            // Just use the Y plane
            let y_size = (frame.width * frame.height) as usize;
            frame.bytes[..y_size.min(frame.bytes.len())].to_vec()
        }
        CameraPixelFormat::Other(_) => {
            // Unknown format, return as-is
            frame.bytes.clone()
        }
    }
}

/// Average multiple face templates together.
fn average_face_templates(samples: &[Vec<u8>]) -> Vec<u8> {
    if samples.is_empty() {
        return vec![0u8; FACE_TEMPLATE_SIZE];
    }

    let mut avg = vec![0u32; FACE_TEMPLATE_SIZE];
    for sample in samples {
        for (i, &byte) in sample.iter().enumerate().take(FACE_TEMPLATE_SIZE) {
            avg[i] += byte as u32;
        }
    }

    let count = samples.len() as u32;
    avg.iter().map(|&v| (v / count) as u8).collect()
}

/// Compute normalized cross-correlation between two face templates.
/// Returns a value between 0.0 (no match) and 1.0 (perfect match).
fn round_f64(value: f64) -> f64 {
    if value.is_sign_negative() {
        (value - 0.5) as i64 as f64
    } else {
        (value + 0.5) as i64 as f64
    }
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

fn normalized_cross_correlation(a: &[u8], b: &[u8]) -> f64 {
    let len = a.len().min(b.len()).min(FACE_TEMPLATE_SIZE);
    if len == 0 {
        return 0.0;
    }

    let mut sum_a = 0.0f64;
    let mut sum_b = 0.0f64;
    let mut sum_aa = 0.0f64;
    let mut sum_bb = 0.0f64;
    let mut sum_ab = 0.0f64;

    for i in 0..len {
        let va = a[i] as f64;
        let vb = b[i] as f64;
        sum_a += va;
        sum_b += vb;
        sum_aa += va * va;
        sum_bb += vb * vb;
        sum_ab += va * vb;
    }

    let n = len as f64;
    let numerator = n * sum_ab - sum_a * sum_b;
    let denominator = sqrt_f64(n * sum_aa - sum_a * sum_a) * sqrt_f64(n * sum_bb - sum_b * sum_b);

    if denominator == 0.0 {
        return 0.0;
    }

    (numerator / denominator).clamp(0.0, 1.0)
}

pub struct V4l2PairedCameraBiometricReader {
    pub selection: V4l2PairedCameraSelection,
}

impl V4l2PairedCameraBiometricReader {
    pub fn new(selection: V4l2PairedCameraSelection) -> Self {
        Self { selection }
    }

    pub fn capture_from_probe(probe: &V4l2CameraProbe) -> CameraFrame {
        CameraFrame {
            width: probe.current_format.width,
            height: probe.current_format.height,
            stride: probe.current_format.stride,
            format: probe.current_format.pixel_format,
            bytes: Vec::new(),
        }
    }

    fn capture_from_device(
        probe: &V4l2CameraProbe,
        timeout_ms: u32,
    ) -> Result<CameraFrame, CameraBiometricError> {
        V4l2CameraDevice::new(probe.info.devnode.clone())
            .capture_frame(timeout_ms)
            .map_err(|e| CameraBiometricError::Provider(e.to_string()))
    }

    fn default_liveness_state(passed: bool) -> edgerun_biometrics::BiometricState {
        default_face_biometric_state(passed, false, true)
    }
}

impl CameraBiometricReader for V4l2PairedCameraBiometricReader {
    fn reader_info(&self) -> Result<CameraReaderInfo, CameraBiometricError> {
        Ok(CameraReaderInfo {
            provider: "v4l2-camera".into(),
            reader_name: self.selection.group_key.clone(),
            supports_face_detection: self.selection.rgb.is_some()
                || self.selection.infrared.is_some(),
            supports_face_matching: false,
            supports_liveness_detection: self.selection.rgb.is_some()
                && self.selection.infrared.is_some(),
            hardware_protected_match: false,
        })
    }

    fn capture(
        &mut self,
        _purpose: CameraBiometricPurpose,
        timeout_ms: u32,
    ) -> Result<CameraCapture, CameraBiometricError> {
        let probe = self
            .selection
            .rgb
            .as_ref()
            .or(self.selection.infrared.as_ref())
            .ok_or(CameraBiometricError::UnsupportedOperation(
                "no capture-capable stream available",
            ))?;
        Ok(CameraCapture {
            frame: Self::capture_from_device(probe, timeout_ms)?,
            quality: default_capture_quality(),
            face_bounds: None,
            state: default_face_biometric_state(false, false, true),
        })
    }

    fn begin_enrollment(
        &mut self,
        request: &edgerun_camera_biometrics::CameraEnrollRequest,
    ) -> Result<CameraEnrollmentSession, CameraBiometricError> {
        edgerun_camera_biometrics::validate_camera_enroll_request(request)?;
        Ok(CameraEnrollmentSession {
            session_id: request.label.clone(),
            label: request.label.clone(),
            samples_required: request.samples_required,
            samples_collected: 0,
            require_liveness: request.require_liveness,
            require_hardware_match: request.require_hardware_match,
        })
    }

    fn enroll_step(
        &mut self,
        session_id: &str,
        capture: &CameraCapture,
    ) -> Result<CameraEnrollProgress, CameraBiometricError> {
        Ok(CameraEnrollProgress {
            session: CameraEnrollmentSession {
                session_id: session_id.to_string(),
                label: String::new(),
                samples_required: 1,
                samples_collected: 1,
                require_liveness: false,
                require_hardware_match: false,
            },
            complete: false,
            template_id: None,
            last_quality: capture.quality,
            face_detected: false,
            liveness_detected: false,
        })
    }

    fn finish_enrollment(
        &mut self,
        session_id: &str,
    ) -> Result<CameraTemplateRecord, CameraBiometricError> {
        Ok(CameraTemplateRecord {
            template_id: session_id.to_string(),
            label: session_id.to_string(),
            enrolled_at_unix_ms: 0,
            last_verified_unix_ms: None,
        })
    }

    fn verify_capture(
        &mut self,
        _request: &CameraVerifyRequest,
        _capture: &CameraCapture,
    ) -> Result<CameraVerification, CameraBiometricError> {
        Ok(CameraVerification {
            matched: false,
            template_id: None,
            liveness_detected: false,
            face_detected: false,
            state: default_face_biometric_state(false, false, true),
        })
    }

    fn list_templates(&self) -> Result<Vec<CameraTemplateRecord>, CameraBiometricError> {
        Ok(Vec::new())
    }

    fn delete_template(&mut self, _template_id: &str) -> Result<(), CameraBiometricError> {
        Err(CameraBiometricError::UnsupportedOperation(
            "template storage is not implemented in the paired V4L2 backend",
        ))
    }
}

impl PairedCameraBiometricReader for V4l2PairedCameraBiometricReader {
    fn supported_stream_roles(&self) -> Result<Vec<CameraStreamRole>, CameraBiometricError> {
        let mut roles = Vec::new();
        if self.selection.rgb.is_some() {
            roles.push(CameraStreamRole::Rgb);
        }
        if self.selection.infrared.is_some() {
            roles.push(CameraStreamRole::Infrared);
        }
        if self.selection.depth.is_some() {
            roles.push(CameraStreamRole::Depth);
        }
        if roles.is_empty() {
            roles.push(CameraStreamRole::Unknown);
        }
        Ok(roles)
    }

    fn capture_paired(
        &mut self,
        _purpose: CameraBiometricPurpose,
        timeout_ms: u32,
    ) -> Result<PairedCameraFrame, CameraBiometricError> {
        Ok(PairedCameraFrame {
            rgb: match &self.selection.rgb {
                Some(probe) => Some(Self::capture_from_device(probe, timeout_ms)?),
                None => None,
            },
            infrared: match &self.selection.infrared {
                Some(probe) => Some(Self::capture_from_device(probe, timeout_ms)?),
                None => None,
            },
            depth: match &self.selection.depth {
                Some(probe) => Some(Self::capture_from_device(probe, timeout_ms)?),
                None => None,
            },
        })
    }

    fn evaluate_liveness(
        &mut self,
        challenge: &CameraLivenessChallenge,
        capture: &PairedCameraFrame,
    ) -> Result<CameraLivenessResult, CameraBiometricError> {
        validate_liveness_challenge(challenge)?;
        let face_present_in_rgb = capture.rgb.is_some();
        let face_present_in_infrared = capture.infrared.is_some();
        let paired_capture = face_present_in_rgb && face_present_in_infrared;
        let passed = match challenge.kind {
            edgerun_camera_biometrics::CameraLivenessChallengeKind::PassivePresence => {
                if challenge.require_rgb && !face_present_in_rgb {
                    false
                } else {
                    !challenge.require_infrared || face_present_in_infrared
                }
            }
            _ => false,
        };
        let mut observations = Vec::new();
        if let Some(rgb) = &capture.rgb {
            observations.push(CameraLivenessFrameObservation {
                role: CameraStreamRole::Rgb,
                face_detected: true,
                quality: default_capture_quality(),
                face_bounds: None,
            });
            let _ = rgb;
        }
        if let Some(infrared) = &capture.infrared {
            observations.push(CameraLivenessFrameObservation {
                role: CameraStreamRole::Infrared,
                face_detected: true,
                quality: default_capture_quality(),
                face_bounds: None,
            });
            let _ = infrared;
        }
        if let Some(depth) = &capture.depth {
            observations.push(CameraLivenessFrameObservation {
                role: CameraStreamRole::Depth,
                face_detected: true,
                quality: default_capture_quality(),
                face_bounds: None,
            });
            let _ = depth;
        }
        Ok(CameraLivenessResult {
            passed,
            evidence: CameraLivenessEvidence {
                challenge: challenge.clone(),
                observations,
                paired_capture,
                motion_detected: false,
                blink_detected: false,
                face_present_in_rgb,
                face_present_in_infrared,
            },
            state: Self::default_liveness_state(passed),
        })
    }
}

// ---------------------------------------------------------------------------
// CapabilityProvider implementations
// ---------------------------------------------------------------------------

use edgerun_capabilities::{CapabilityDescriptor, CapabilityProvider};
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityEventKind, CapabilityModality, CapabilityOperation, CapabilityRole,
};

impl CapabilityProvider for V4l2CameraBiometricReader {
    fn descriptor(&self) -> CapabilityDescriptor {
        let instance = self.device.devnode.to_string_lossy().to_string();
        CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: instance.as_bytes().to_vec(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Visual as i32],
            event_kinds: vec![CapabilityEventKind::Visual as i32],
            operations: vec![
                CapabilityOperation::Query as i32,
                CapabilityOperation::Capture as i32,
            ],
            default_constraints: Vec::new(),
            provider_name: "v4l2-camera".into(),
            provider_instance_id: instance,
            signature: None,
        }
    }
}

impl CapabilityProvider for V4l2PairedCameraBiometricReader {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: self.selection.group_key.as_bytes().to_vec(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: {
                let mut mods = vec![CapabilityModality::Visual as i32];
                if self.selection.infrared.is_some() {
                    mods.push(CapabilityModality::Biometric as i32);
                }
                mods
            },
            event_kinds: vec![
                CapabilityEventKind::Visual as i32,
                CapabilityEventKind::Biometric as i32,
            ],
            operations: vec![
                CapabilityOperation::Query as i32,
                CapabilityOperation::Capture as i32,
            ],
            default_constraints: Vec::new(),
            provider_name: "v4l2-camera".into(),
            provider_instance_id: self.selection.group_key.clone(),
            signature: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn parse_c_string_stops_at_nul() {
        assert_eq!(parse_c_string(b"uvcvideo\0junk"), "uvcvideo");
    }

    #[test]
    fn discover_camera_devices_filters_video_nodes() {
        let root = temp_root("v4l2-discovery");
        fs::write(root.join("video0"), b"").unwrap();
        fs::write(root.join("video2"), b"").unwrap();
        fs::write(root.join("tty0"), b"").unwrap();
        let devices = discover_camera_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 2);
        assert!(devices[0].devnode.ends_with("video0"));
        assert!(devices[1].devnode.ends_with("video2"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pixel_format_mapping_works() {
        assert_eq!(
            pixel_format_from_v4l2(V4L2_PIX_FMT_MJPEG),
            CameraPixelFormat::Mjpeg
        );
        assert_eq!(
            pixel_format_from_v4l2(V4L2_PIX_FMT_YUYV),
            CameraPixelFormat::Yuyv
        );
        assert_eq!(
            pixel_format_from_v4l2(V4L2_PIX_FMT_Y16),
            CameraPixelFormat::Gray8
        );
        assert_eq!(
            pixel_format_from_v4l2(0x12345678),
            CameraPixelFormat::Other(0x12345678)
        );
    }

    #[test]
    fn default_quality_is_good() {
        assert_eq!(default_capture_quality(), CameraCaptureQuality::Good);
    }

    #[test]
    fn infrared_inference_uses_name_and_format_heuristics() {
        assert_eq!(
            infer_infrared_capability("Integrated IR Camera", CameraPixelFormat::Mjpeg),
            V4l2InfraredCapability::InfraredLikely
        );
        assert_eq!(
            infer_infrared_capability("Monochrome Sensor", CameraPixelFormat::Gray8),
            V4l2InfraredCapability::MonochromeLikely
        );
        assert_eq!(
            infer_infrared_capability("RGB Camera", CameraPixelFormat::Rgb24),
            V4l2InfraredCapability::NotDetected
        );
    }

    #[test]
    fn stream_role_inference_uses_name_and_format_heuristics() {
        assert_eq!(
            infer_stream_role("Integrated IR Camera", CameraPixelFormat::Gray8),
            V4l2StreamRole::InfraredLikely
        );
        assert_eq!(
            infer_stream_role("Depth Sensor", CameraPixelFormat::Gray8),
            V4l2StreamRole::DepthLikely
        );
        assert_eq!(
            infer_stream_role("RGB Camera", CameraPixelFormat::Rgb24),
            V4l2StreamRole::RgbLikely
        );
    }

    #[test]
    fn query_info_initially_marks_infrared_unknown() {
        let info = V4l2CameraInfo {
            devnode: PathBuf::from("/dev/video0"),
            driver: "uvcvideo".into(),
            card: "Test Camera".into(),
            bus_info: "usb-1".into(),
            version: 1,
            capabilities: V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_STREAMING,
            device_caps: V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_STREAMING,
            infrared: V4l2InfraredCapability::Unknown,
        };
        assert_eq!(info.infrared, V4l2InfraredCapability::Unknown);
        assert!(info.supports_video_capture());
        assert!(info.supports_streaming());
    }

    #[test]
    fn group_camera_probes_groups_same_bus_and_card() {
        let probe_a = V4l2CameraProbe {
            info: V4l2CameraInfo {
                devnode: PathBuf::from("/dev/video0"),
                driver: "uvcvideo".into(),
                card: "BRIO".into(),
                bus_info: "usb-1-3".into(),
                version: 1,
                capabilities: V4L2_CAP_VIDEO_CAPTURE,
                device_caps: V4L2_CAP_VIDEO_CAPTURE,
                infrared: V4l2InfraredCapability::InfraredLikely,
            },
            current_format: V4l2FrameFormat {
                width: 640,
                height: 480,
                stride: 640,
                image_size: 640 * 480,
                pixel_format: CameraPixelFormat::Gray8,
            },
            available_formats: Vec::new(),
            stream_role: V4l2StreamRole::InfraredLikely,
        };
        let mut probe_b = probe_a.clone();
        probe_b.info.devnode = PathBuf::from("/dev/video1");
        probe_b.stream_role = V4l2StreamRole::RgbLikely;
        probe_b.info.infrared = V4l2InfraredCapability::NotDetected;
        let groups = group_camera_probes(vec![probe_b.clone(), probe_a.clone()]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].devices.len(), 2);
        assert!(groups[0].devices[0].info.devnode.ends_with("video0"));
        assert!(groups[0].devices[1].info.devnode.ends_with("video1"));
    }

    #[test]
    fn select_paired_camera_prefers_rgb_and_mono_ir() {
        let rgb = V4l2CameraProbe {
            info: V4l2CameraInfo {
                devnode: PathBuf::from("/dev/video0"),
                driver: "uvcvideo".into(),
                card: "BRIO".into(),
                bus_info: "usb-1".into(),
                version: 1,
                capabilities: V4L2_CAP_VIDEO_CAPTURE,
                device_caps: V4L2_CAP_VIDEO_CAPTURE,
                infrared: V4l2InfraredCapability::NotDetected,
            },
            current_format: V4l2FrameFormat {
                width: 1280,
                height: 720,
                stride: 0,
                image_size: 1024,
                pixel_format: CameraPixelFormat::Mjpeg,
            },
            available_formats: Vec::new(),
            stream_role: V4l2StreamRole::RgbLikely,
        };
        let ir = V4l2CameraProbe {
            info: V4l2CameraInfo {
                devnode: PathBuf::from("/dev/video2"),
                driver: "uvcvideo".into(),
                card: "BRIO".into(),
                bus_info: "usb-1".into(),
                version: 1,
                capabilities: V4L2_CAP_VIDEO_CAPTURE,
                device_caps: V4L2_CAP_VIDEO_CAPTURE,
                infrared: V4l2InfraredCapability::MonochromeLikely,
            },
            current_format: V4l2FrameFormat {
                width: 340,
                height: 340,
                stride: 340,
                image_size: 340 * 340,
                pixel_format: CameraPixelFormat::Gray8,
            },
            available_formats: Vec::new(),
            stream_role: V4l2StreamRole::MonochromeLikely,
        };
        let selection = select_paired_camera_from_group(&V4l2CameraGroup {
            key: "usb-1::BRIO".into(),
            card: "BRIO".into(),
            bus_info: "usb-1".into(),
            devices: vec![rgb.clone(), ir.clone()],
        })
        .unwrap();
        assert_eq!(
            selection.rgb.unwrap().info.devnode,
            PathBuf::from("/dev/video0")
        );
        assert_eq!(
            selection.infrared.unwrap().info.devnode,
            PathBuf::from("/dev/video2")
        );
    }
}
