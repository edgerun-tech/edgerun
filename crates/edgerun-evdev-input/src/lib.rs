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

        pub fn open<P>(self, _path: P) -> io::Result<File> {
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
        pub fn file_name(&self) -> PathBuf {
            PathBuf::from("")
        }

        #[must_use]
        pub fn path(&self) -> PathBuf {
            PathBuf::from("")
        }
    }

    pub fn read_to_string<P>(_path: P) -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::NotFound))
    }

    pub fn read_dir<P>(_path: P) -> io::Result<ReadDir> {
        Err(io::Error::new(io::ErrorKind::NotFound))
    }

    pub fn canonicalize(path: PathBuf) -> io::Result<PathBuf> {
        Ok(path)
    }
}

#[cfg(target_os = "none")]
pub mod io {
    use core::fmt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ErrorKind {
        Interrupted,
        NotFound,
        WouldBlock,
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
        pub const fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.kind {
                ErrorKind::Interrupted => f.write_str("interrupted"),
                ErrorKind::NotFound => f.write_str("not found"),
                ErrorKind::WouldBlock => f.write_str("operation would block"),
                ErrorKind::Other => f.write_str("I/O error"),
            }
        }
    }

    impl core::error::Error for Error {}

    pub type Result<T> = core::result::Result<T, Error>;

    pub trait Read {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    }

    impl Read for crate::fs::File {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {
            Err(Error::new(ErrorKind::WouldBlock))
        }
    }
}

#[cfg(target_os = "none")]
pub mod mem {
    pub use core::mem::*;
}

#[cfg(target_os = "none")]
pub mod os {
    pub mod fd {
        pub type RawFd = i32;

        pub trait AsRawFd {
            fn as_raw_fd(&self) -> RawFd;
        }

        impl AsRawFd for crate::fs::File {
            fn as_raw_fd(&self) -> RawFd {
                -1
            }
        }
    }

    pub mod raw {
        #[allow(non_camel_case_types)]
        pub type c_int = i32;
    }
}

#[cfg(target_os = "none")]
pub mod path {
    pub use edgerun_linux_sysfs::path::{Path, PathBuf};
}

#[cfg(target_os = "none")]
pub mod time {
    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
    pub struct Instant(Duration);

    impl Instant {
        #[must_use]
        pub const fn now() -> Self {
            Self(Duration::from_secs(0))
        }
    }

    impl core::ops::Add<Duration> for Instant {
        type Output = Self;

        fn add(self, rhs: Duration) -> Self::Output {
            Self(self.0.saturating_add(rhs))
        }
    }

    impl core::ops::Sub for Instant {
        type Output = Duration;

        fn sub(self, rhs: Self) -> Self::Output {
            self.0.saturating_sub(rhs.0)
        }
    }
}

#[cfg(target_os = "none")]
pub mod str {
    pub use core::str::*;
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

#[cfg(target_os = "none")]
pub mod libc {
    pub const F_GETFL: i32 = 3;
    pub const F_SETFL: i32 = 4;
    pub const O_NONBLOCK: i32 = 0x800;

    #[repr(C)]
    pub struct pollfd {
        pub fd: i32,
        pub events: i16,
        pub revents: i16,
    }

    pub unsafe fn ioctl<T>(_fd: i32, _request: usize, _arg: T) -> i32 {
        -1
    }

    pub unsafe fn fcntl(_fd: i32, _cmd: i32, _arg: i32) -> i32 {
        -1
    }

    pub unsafe fn poll(_fds: *mut pollfd, _nfds: usize, _timeout: i32) -> i32 {
        0
    }
}

use crate::prelude::v1::*;
use edgerun_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use edgerun_input::{
    default_input_descriptor, validate_event_read_request, InputDevice, InputDeviceInfo,
    InputDeviceKind, InputEventKind, InputEventRecord,
};
use edgerun_linux_sysfs::read_trimmed;
#[cfg(not(target_os = "none"))]
use std::fs::{self, File, OpenOptions};
#[cfg(target_os = "none")]
use std::fs::{File, OpenOptions};
#[cfg(target_os = "none")]
use std::io::Read;
#[cfg(not(target_os = "none"))]
use std::io::{self, Read};
use std::os::fd::{AsRawFd, RawFd};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const EV_MSC: u16 = 0x04;
const EV_SW: u16 = 0x05;

// evdev ioctls
const EVIOCGNAME: c_int = 0x81004506u32 as c_int; // _IOC(_IOC_READ, 'E', 0x06, 256)
const EVIOCGBIT: c_int = 0x80004520u32 as c_int; // _IOC(_IOC_READ, 'E', 0x20, len)
const EVIOCGRAB: c_int = 0x40044590u32 as c_int; // _IOC(_IOC_WRITE, 'E', 0x90, 4)

// poll constants
const POLLIN: i16 = 0x001;
const POLLERR: i16 = 0x008;
const POLLHUP: i16 = 0x010;

const KEY_A: usize = 30;
const BTN_MOUSE: usize = 0x110;
const BTN_TOUCH: usize = 0x14a;
const BTN_TOOL_PEN: usize = 0x140;
const BTN_GAMEPAD: usize = 0x130;
const REL_X: usize = 0x00;
const REL_Y: usize = 0x01;
const ABS_X: usize = 0x00;
const ABS_Y: usize = 0x01;
const ABS_MT_POSITION_X: usize = 0x35;
const ABS_MT_POSITION_Y: usize = 0x36;
const SW_LID: usize = 0x00;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LinuxInputEvent {
    time: TimeVal,
    type_: u16,
    code: u16,
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvdevDeviceInfo {
    pub event_node: String,
    pub sysfs_path: PathBuf,
    pub device_name: String,
    pub physical_path: Option<String>,
    pub unique_id: Option<String>,
    pub bus_type: Option<u16>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub version: Option<u16>,
    pub modalias: Option<String>,
    pub kind: InputDeviceKind,
    pub capabilities_ev: Vec<u64>,
    pub capabilities_key: Vec<u64>,
    pub capabilities_rel: Vec<u64>,
    pub capabilities_abs: Vec<u64>,
    pub capabilities_sw: Vec<u64>,
}

#[derive(Debug)]
pub struct EvdevInputBackend {
    pub info: EvdevDeviceInfo,
    file: File,
}

fn parse_uevent_list(path: &Path) -> Vec<(String, String)> {
    let Some(raw) = fs::read_to_string(path).ok() else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

fn parse_product_id(value: &str) -> (Option<u16>, Option<u16>, Option<u16>, Option<u16>) {
    let mut parts = value.split('/');
    let bus = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    let vendor = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    let product = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    let version = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    (bus, vendor, product, version)
}

fn parse_hex_bitmap(text: &str) -> Vec<u64> {
    text.split_whitespace()
        .filter_map(|chunk| u64::from_str_radix(chunk, 16).ok())
        .collect()
}

#[cfg(test)]
fn bitmap_with_bit(bit: usize) -> Vec<u64> {
    let mut out = vec![0u64; (bit / 64) + 1];
    out[bit / 64] |= 1u64 << (bit % 64);
    out
}

fn bitmap_contains(words: &[u64], bit: usize) -> bool {
    let word_index = bit / 64;
    if word_index >= words.len() {
        return false;
    }
    let bit_index = bit % 64;
    (words[word_index] & (1u64 << bit_index)) != 0
}

/// Query device capabilities via EVIOCGBIT ioctl on an open evdev fd.
///
/// Returns the capability bitmap for the given event type (EV_KEY, EV_REL,
/// EV_ABS, etc.) directly from the kernel, as an alternative to sysfs reading.
pub fn query_capabilities_ev(fd: RawFd, ev_type: u16) -> Result<Vec<u8>, io::Error> {
    const EVIOCGBIT_BUF_LEN: usize = 32; // 256 bits per event type

    // EVIOCGBIT(ev, len) = _IOC(_IOC_READ, 'E', 0x20 + ev, len)
    // Use the constant EVIOCGBIT as base and adjust for the event type
    let request = EVIOCGBIT | (ev_type as c_int);

    let mut buf = vec![0u8; EVIOCGBIT_BUF_LEN];
    let ret = unsafe { libc::ioctl(fd, request as _, buf.as_mut_ptr()) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    let bytes_written = ret as usize;
    buf.truncate(bytes_written.min(EVIOCGBIT_BUF_LEN));
    Ok(buf)
}

fn classify_device(keys: &[u64], rel: &[u64], abs: &[u64], sw: &[u64]) -> InputDeviceKind {
    if bitmap_contains(keys, BTN_TOUCH)
        || bitmap_contains(abs, ABS_MT_POSITION_X)
        || bitmap_contains(abs, ABS_MT_POSITION_Y)
    {
        return InputDeviceKind::Touch;
    }
    if bitmap_contains(keys, BTN_TOOL_PEN) {
        return InputDeviceKind::Pen;
    }
    if bitmap_contains(keys, BTN_MOUSE)
        || bitmap_contains(rel, REL_X)
        || bitmap_contains(rel, REL_Y)
    {
        return InputDeviceKind::Pointer;
    }
    if bitmap_contains(keys, BTN_GAMEPAD) {
        return InputDeviceKind::Gamepad;
    }
    if bitmap_contains(sw, SW_LID) {
        return InputDeviceKind::Switch;
    }
    if bitmap_contains(keys, KEY_A) || bitmap_contains(abs, ABS_X) || bitmap_contains(abs, ABS_Y) {
        return InputDeviceKind::Keyboard;
    }
    InputDeviceKind::Other
}

fn event_kind(ty: u16) -> InputEventKind {
    match ty {
        EV_SYN => InputEventKind::Synchronization,
        EV_KEY => InputEventKind::Key,
        EV_REL => InputEventKind::RelativeMotion,
        EV_ABS => InputEventKind::AbsoluteMotion,
        EV_SW => InputEventKind::Switch,
        EV_MSC => InputEventKind::Misc,
        other => InputEventKind::Other(other),
    }
}

pub fn discover_evdev_devices() -> Result<Vec<EvdevDeviceInfo>, CapabilityError> {
    discover_evdev_devices_in(Path::new("/sys/class/input"))
}

pub fn discover_evdev_devices_in(root: &Path) -> Result<Vec<EvdevDeviceInfo>, CapabilityError> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| CapabilityError::Provider(e.to_string()))? {
        let entry = entry.map_err(|e| CapabilityError::Provider(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("event") {
            continue;
        }
        let sysfs_path = entry.path();
        let device_path =
            fs::canonicalize(sysfs_path.join("device")).unwrap_or(sysfs_path.join("device"));
        let device_name = read_trimmed(&device_path.join("name")).unwrap_or_else(|| name.clone());
        let physical_path = read_trimmed(&device_path.join("phys"));
        let unique_id = read_trimmed(&device_path.join("uniq")).filter(|s| !s.is_empty());
        let uevent = parse_uevent_list(&device_path.join("uevent"));
        let mut bus_type = None;
        let mut vendor_id = None;
        let mut product_id = None;
        let mut version = None;
        let mut modalias = None;
        for (key, value) in &uevent {
            match key.as_str() {
                "PRODUCT" => {
                    let (bus, vendor, product, ver) = parse_product_id(value);
                    bus_type = bus;
                    vendor_id = vendor;
                    product_id = product;
                    version = ver;
                }
                "MODALIAS" => modalias = Some(value.clone()),
                _ => {}
            }
        }
        let capabilities_ev = read_trimmed(&device_path.join("capabilities/ev"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_key = read_trimmed(&device_path.join("capabilities/key"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_rel = read_trimmed(&device_path.join("capabilities/rel"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_abs = read_trimmed(&device_path.join("capabilities/abs"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_sw = read_trimmed(&device_path.join("capabilities/sw"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let kind = classify_device(
            &capabilities_key,
            &capabilities_rel,
            &capabilities_abs,
            &capabilities_sw,
        );
        out.push(EvdevDeviceInfo {
            event_node: name,
            sysfs_path,
            device_name,
            physical_path,
            unique_id,
            bus_type,
            vendor_id,
            product_id,
            version,
            modalias,
            kind,
            capabilities_ev,
            capabilities_key,
            capabilities_rel,
            capabilities_abs,
            capabilities_sw,
        });
    }
    out.sort_by(|a, b| a.event_node.cmp(&b.event_node));
    Ok(out)
}

impl EvdevInputBackend {
    pub fn open(info: EvdevDeviceInfo) -> Result<Self, CapabilityError> {
        let path = PathBuf::from("/dev/input").join(&info.event_node);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|e| CapabilityError::Provider(format!("open {}: {e}", path.display())))?;

        // Set non-blocking mode for event loop support
        let fd = file.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL, 0) };
        if flags >= 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        }

        Ok(Self { info, file })
    }

    /// Get the file descriptor for epoll.
    pub fn fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// Read events with a timeout. Returns empty Vec if timeout expires.
    /// This is the primary event loop method — it uses poll() to wait
    /// for events with a deadline, then reads all available events.
    pub fn read_events_timeout(
        &mut self,
        max_events: usize,
        timeout: Duration,
    ) -> Result<Vec<InputEventRecord>, CapabilityError> {
        let event_size = std::mem::size_of::<LinuxInputEvent>();
        let mut buf = vec![0u8; event_size * max_events];
        let mut total_read = 0;
        let deadline = Instant::now() + timeout;

        loop {
            // Check if we've hit the deadline
            let now = Instant::now();
            if now >= deadline {
                break;
            }

            // Poll the file descriptor
            let remaining = deadline - now;
            let timeout_ms = remaining.as_millis().min(i32::MAX as u128) as i32;

            let mut pollfd = libc::pollfd {
                fd: self.file.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            };

            let ret = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };

            if ret < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue; // EINTR, retry
                }
                return Err(CapabilityError::Provider(format!(
                    "poll evdev fd {}: {}",
                    self.file.as_raw_fd(),
                    err
                )));
            }

            if ret == 0 {
                break; // Timeout
            }

            // Check for errors
            if pollfd.revents & (POLLERR | POLLHUP) != 0 {
                return Err(CapabilityError::Provider(format!(
                    "evdev fd {} error/hangup",
                    self.file.as_raw_fd()
                )));
            }

            // Read available events
            let remaining_bytes = buf.len() - total_read;
            if remaining_bytes < event_size {
                break; // Buffer full
            }

            match self
                .file
                .read(&mut buf[total_read..total_read + remaining_bytes])
            {
                Ok(0) => {
                    // EOF — device disconnected
                    return Err(CapabilityError::Provider(format!(
                        "evdev fd {} EOF (device disconnected)",
                        self.file.as_raw_fd()
                    )));
                }
                Ok(n) => {
                    total_read += n;
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        continue; // No data yet, poll again
                    }
                    return Err(CapabilityError::Provider(format!(
                        "read evdev fd {}: {}",
                        self.file.as_raw_fd(),
                        e
                    )));
                }
            }
        }

        // Parse the events we collected
        if total_read % event_size != 0 {
            return Err(CapabilityError::Provider(
                "evdev read returned partial input_event records".into(),
            ));
        }

        let mut out = Vec::new();
        for chunk in buf[..total_read].chunks_exact(event_size) {
            let event = unsafe { (chunk.as_ptr() as *const LinuxInputEvent).read_unaligned() };
            out.push(InputEventRecord {
                timestamp_sec: event.time.tv_sec,
                timestamp_usec: event.time.tv_usec,
                kind: event_kind(event.type_),
                code: event.code,
                value: event.value,
            });
        }
        Ok(out)
    }

    /// Read all currently available events without blocking.
    /// Returns empty Vec if no events are pending.
    pub fn read_events_nonblocking(
        &mut self,
        max_events: usize,
    ) -> Result<Vec<InputEventRecord>, CapabilityError> {
        self.read_events_timeout(max_events, Duration::ZERO)
    }

    /// Grab exclusive access to the device (EVIOCGRAB).
    /// While grabbed, no other process receives events from this device.
    pub fn grab(&self, grab: bool) -> Result<(), CapabilityError> {
        let grab_val: c_int = if grab { 1 } else { 0 };
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), EVIOCGRAB as _, grab_val) };
        if ret < 0 {
            Err(CapabilityError::Provider(format!(
                "EVIOCGRAB failed: {}",
                std::io::Error::last_os_error()
            )))
        } else {
            Ok(())
        }
    }

    /// Get device name via EVIOCGNAME ioctl (more reliable than sysfs).
    pub fn ioctl_device_name(&self) -> Result<String, CapabilityError> {
        let mut buf = [0u8; 256];
        let ret = unsafe { libc::ioctl(self.file.as_raw_fd(), EVIOCGNAME as _, buf.as_mut_ptr()) };
        if ret < 0 {
            return Err(CapabilityError::Provider(format!(
                "EVIOCGNAME failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        let len = ret as usize;
        let name = std::str::from_utf8(&buf[..len])
            .map_err(|e| CapabilityError::Provider(format!("invalid device name: {}", e)))?
            .trim_end_matches('\0')
            .to_string();
        Ok(name)
    }
}

impl CapabilityProvider for EvdevInputBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_input_descriptor("evdev-kernel", &self.info.event_node)
    }
}

impl InputDevice for EvdevInputBackend {
    fn input_info(&self) -> Result<InputDeviceInfo, CapabilityError> {
        Ok(InputDeviceInfo {
            provider: "evdev-kernel".into(),
            instance_id: self.info.event_node.clone(),
            display_name: self.info.device_name.clone(),
            kind: self.info.kind,
            event_node: format!("/dev/input/{}", self.info.event_node),
            physical_path: self.info.physical_path.clone(),
            unique_id: self.info.unique_id.clone(),
        })
    }

    fn read_events(&mut self, max_events: usize) -> Result<Vec<InputEventRecord>, CapabilityError> {
        validate_event_read_request(max_events)?;
        // Non-blocking: return whatever events are available right now
        self.read_events_nonblocking(max_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_linux_sysfs::temp_root;

    #[test]
    fn parse_product_id_splits_fields() {
        assert_eq!(
            parse_product_id("0003/046D/085E/0111"),
            (Some(0x0003), Some(0x046d), Some(0x085e), Some(0x0111))
        );
    }

    #[test]
    fn bitmap_parse_and_contains_work() {
        let bits = parse_hex_bitmap("3 0 10");
        assert!(bitmap_contains(&bits, 0));
        assert!(bitmap_contains(&bits, 1));
        assert!(!bitmap_contains(&bits, 2));
    }

    #[test]
    fn classify_pointer_and_touch() {
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_MOUSE), &[0b11], &[], &[]),
            InputDeviceKind::Pointer
        );
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_TOUCH), &[], &[], &[]),
            InputDeviceKind::Touch
        );
    }

    #[test]
    fn discover_devices_from_sysfs_layout() {
        let root = temp_root("evdev-input");
        let event = root.join("event0");
        fs::create_dir_all(event.join("device/capabilities")).unwrap();
        fs::write(event.join("device/name"), "Test Keyboard\n").unwrap();
        fs::write(event.join("device/phys"), "usb-test/input0\n").unwrap();
        fs::write(event.join("device/capabilities/ev"), "3\n").unwrap();
        fs::write(
            event.join("device/capabilities/key"),
            format!("{:x}\n", 1u64 << KEY_A),
        )
        .unwrap();
        let devices = discover_evdev_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_name, "Test Keyboard");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn event_kind_maps_common_types() {
        assert_eq!(event_kind(EV_KEY), InputEventKind::Key);
        assert_eq!(event_kind(EV_REL), InputEventKind::RelativeMotion);
    }

    // ----- parse_hex_bitmap -----

    #[test]
    fn parse_hex_bitmap_single_word() {
        let bits = parse_hex_bitmap("3");
        assert_eq!(bits, vec![3u64]);
    }

    #[test]
    fn parse_hex_bitmap_multiple_words() {
        let bits = parse_hex_bitmap("1 2 4");
        assert_eq!(bits, vec![1u64, 2u64, 4u64]);
    }

    #[test]
    fn parse_hex_bitmap_empty_string() {
        let bits = parse_hex_bitmap("");
        assert!(bits.is_empty());
    }

    #[test]
    fn parse_hex_bitmap_ignores_invalid_chunks() {
        let bits = parse_hex_bitmap("1 invalid 2");
        assert_eq!(bits.len(), 2);
        assert_eq!(bits[0], 1);
        assert_eq!(bits[1], 2);
    }

    #[test]
    fn parse_hex_bitmap_strips_whitespace() {
        let bits = parse_hex_bitmap("  ff  ");
        assert_eq!(bits, vec![0xffu64]);
    }

    // ----- bitmap_contains -----

    #[test]
    fn bitmap_contains_bit_at_boundary() {
        let bits = parse_hex_bitmap("1"); // bit 0
        assert!(bitmap_contains(&bits, 0));
        assert!(!bitmap_contains(&bits, 1));
    }

    #[test]
    fn bitmap_contains_bit_in_second_word() {
        let bits = parse_hex_bitmap("0 1"); // bit 64
        assert!(!bitmap_contains(&bits, 63));
        assert!(bitmap_contains(&bits, 64));
        assert!(!bitmap_contains(&bits, 65));
    }

    #[test]
    fn bitmap_contains_out_of_range_returns_false() {
        let bits = parse_hex_bitmap("0"); // only 1 word (64 bits), but value is 0
        assert!(!bitmap_contains(&bits, 128));
    }

    #[test]
    fn bitmap_contains_empty_bitmap() {
        assert!(!bitmap_contains(&[], 0));
    }

    #[test]
    fn bitmap_contains_high_bit() {
        // 0x8000000000000000 in word 1 means bit 63 of that word = global bit 127
        let bits = parse_hex_bitmap("0 8000000000000000");
        assert!(bitmap_contains(&bits, 127));
        assert!(!bitmap_contains(&bits, 126));
    }

    // ----- bitmap_with_bit -----

    #[test]
    fn bitmap_with_bit_zero() {
        let bits = bitmap_with_bit(0);
        assert_eq!(bits, vec![1u64]);
    }

    #[test]
    fn bitmap_with_bit_63() {
        let bits = bitmap_with_bit(63);
        assert_eq!(bits, vec![1u64 << 63]);
    }

    #[test]
    fn bitmap_with_bit_64() {
        let bits = bitmap_with_bit(64);
        assert_eq!(bits.len(), 2);
        assert_eq!(bits[0], 0);
        assert_eq!(bits[1], 1);
    }

    #[test]
    fn bitmap_with_bit_in_third_word() {
        let bits = bitmap_with_bit(128);
        assert_eq!(bits.len(), 3);
        assert_eq!(bits[2], 1);
    }

    // ----- classify_device -----

    #[test]
    fn classify_touch_by_btn_touch() {
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_TOUCH), &[], &[], &[]),
            InputDeviceKind::Touch
        );
    }

    #[test]
    fn classify_touch_by_abs_mt_position() {
        assert_eq!(
            classify_device(&[], &[], &bitmap_with_bit(ABS_MT_POSITION_X), &[]),
            InputDeviceKind::Touch
        );
        assert_eq!(
            classify_device(&[], &[], &bitmap_with_bit(ABS_MT_POSITION_Y), &[]),
            InputDeviceKind::Touch
        );
    }

    #[test]
    fn classify_pen() {
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_TOOL_PEN), &[], &[], &[]),
            InputDeviceKind::Pen
        );
    }

    #[test]
    fn classify_pointer_by_btn_mouse() {
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_MOUSE), &[], &[], &[]),
            InputDeviceKind::Pointer
        );
    }

    #[test]
    fn classify_pointer_by_rel_axes() {
        assert_eq!(
            classify_device(&[], &bitmap_with_bit(REL_X), &[], &[]),
            InputDeviceKind::Pointer
        );
        assert_eq!(
            classify_device(&[], &bitmap_with_bit(REL_Y), &[], &[]),
            InputDeviceKind::Pointer
        );
    }

    #[test]
    fn classify_gamepad() {
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_GAMEPAD), &[], &[], &[]),
            InputDeviceKind::Gamepad
        );
    }

    #[test]
    fn classify_switch() {
        assert_eq!(
            classify_device(&[], &[], &[], &bitmap_with_bit(SW_LID)),
            InputDeviceKind::Switch
        );
    }

    #[test]
    fn classify_keyboard_by_key_a() {
        assert_eq!(
            classify_device(&bitmap_with_bit(KEY_A), &[], &[], &[]),
            InputDeviceKind::Keyboard
        );
    }

    #[test]
    fn classify_keyboard_by_abs_x() {
        assert_eq!(
            classify_device(&[], &[], &bitmap_with_bit(ABS_X), &[]),
            InputDeviceKind::Keyboard
        );
        assert_eq!(
            classify_device(&[], &[], &bitmap_with_bit(ABS_Y), &[]),
            InputDeviceKind::Keyboard
        );
    }

    #[test]
    fn classify_other() {
        assert_eq!(classify_device(&[], &[], &[], &[]), InputDeviceKind::Other);
    }

    #[test]
    fn classify_touch_takes_priority_over_pointer() {
        // A device with both BTN_TOUCH and BTN_MOUSE should be Touch (checked first)
        assert_eq!(
            classify_device(
                &bitmap_with_bit(BTN_TOUCH),
                &bitmap_with_bit(REL_X),
                &[],
                &[]
            ),
            InputDeviceKind::Touch
        );
    }

    // ----- event_kind -----

    #[test]
    fn event_kind_syn() {
        assert_eq!(event_kind(EV_SYN), InputEventKind::Synchronization);
    }

    #[test]
    fn event_kind_abs() {
        assert_eq!(event_kind(EV_ABS), InputEventKind::AbsoluteMotion);
    }

    #[test]
    fn event_kind_sw() {
        assert_eq!(event_kind(EV_SW), InputEventKind::Switch);
    }

    #[test]
    fn event_kind_msc() {
        assert_eq!(event_kind(EV_MSC), InputEventKind::Misc);
    }

    #[test]
    fn event_kind_unknown_type() {
        assert_eq!(event_kind(0x99), InputEventKind::Other(0x99));
    }

    // ----- EvdevDeviceInfo -----

    #[test]
    fn evdev_device_info_clone_and_eq() {
        let info = EvdevDeviceInfo {
            event_node: "event0".into(),
            sysfs_path: PathBuf::from("/sys/class/input/event0"),
            device_name: "Test".into(),
            physical_path: Some("usb-0".into()),
            unique_id: Some("uid".into()),
            bus_type: Some(3),
            vendor_id: Some(0x046d),
            product_id: Some(0xc077),
            version: Some(0x0111),
            modalias: Some("usb:abc".into()),
            kind: InputDeviceKind::Pointer,
            capabilities_ev: vec![0x03],
            capabilities_key: vec![0x10000],
            capabilities_rel: vec![0x03],
            capabilities_abs: vec![],
            capabilities_sw: vec![],
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn evdev_device_info_debug_format() {
        let info = EvdevDeviceInfo {
            event_node: "event0".into(),
            sysfs_path: PathBuf::from("/sys/class/input/event0"),
            device_name: "Test".into(),
            physical_path: None,
            unique_id: None,
            bus_type: None,
            vendor_id: None,
            product_id: None,
            version: None,
            modalias: None,
            kind: InputDeviceKind::Keyboard,
            capabilities_ev: vec![],
            capabilities_key: vec![],
            capabilities_rel: vec![],
            capabilities_abs: vec![],
            capabilities_sw: vec![],
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("event0"));
        assert!(debug.contains("Test"));
    }

    // ----- discover_evdev_devices_in -----

    #[test]
    fn discover_devices_empty_root() {
        let dir = temp_root("evdev-input-empty");
        let devices = discover_evdev_devices_in(&dir).unwrap();
        assert!(devices.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_devices_skips_non_event_entries() {
        let dir = temp_root("evdev-input-skip");
        fs::create_dir_all(dir.join("mouse0")).unwrap();
        fs::create_dir_all(dir.join("js0")).unwrap();
        let devices = discover_evdev_devices_in(&dir).unwrap();
        assert!(devices.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_devices_with_minimal_sysfs() {
        let dir = temp_root("evdev-input-minimal");
        let event = dir.join("event0");
        fs::create_dir_all(event.join("device/capabilities")).unwrap();
        fs::write(event.join("device/capabilities/ev"), "1\n").unwrap();
        let devices = discover_evdev_devices_in(&dir).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].event_node, "event0");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_devices_parses_product_id() {
        let dir = temp_root("evdev-input-product");
        let event = dir.join("event0");
        fs::create_dir_all(event.join("device/capabilities")).unwrap();
        fs::write(event.join("device/uevent"), "PRODUCT=0003/046d/c077/0111\n").unwrap();
        fs::write(event.join("device/capabilities/ev"), "0\n").unwrap();
        let devices = discover_evdev_devices_in(&dir).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].bus_type, Some(0x0003));
        assert_eq!(devices[0].vendor_id, Some(0x046d));
        assert_eq!(devices[0].product_id, Some(0xc077));
        assert_eq!(devices[0].version, Some(0x0111));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_devices_parses_modalias() {
        let dir = temp_root("evdev-input-modalias");
        let event = dir.join("event0");
        fs::create_dir_all(event.join("device/capabilities")).unwrap();
        fs::write(
            event.join("device/uevent"),
            "MODALIAS=input:b0003v046Dp085Ee0110\n",
        )
        .unwrap();
        fs::write(event.join("device/capabilities/ev"), "0\n").unwrap();
        let devices = discover_evdev_devices_in(&dir).unwrap();
        assert_eq!(
            devices[0].modalias,
            Some("input:b0003v046Dp085Ee0110".into())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_devices_sorts_by_event_node() {
        let dir = temp_root("evdev-input-sort");
        for name in &["event2", "event0", "event1"] {
            let entry = dir.join(name);
            fs::create_dir_all(entry.join("device/capabilities")).unwrap();
            fs::write(entry.join("device/capabilities/ev"), "0\n").unwrap();
        }
        let devices = discover_evdev_devices_in(&dir).unwrap();
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].event_node, "event0");
        assert_eq!(devices[1].event_node, "event1");
        assert_eq!(devices[2].event_node, "event2");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_devices_nonexistent_root() {
        let devices = discover_evdev_devices_in(Path::new("/nonexistent/sysfs")).unwrap();
        assert!(devices.is_empty());
    }

    #[test]
    fn discover_devices_with_full_layout() {
        let root = temp_root("evdev-input");
        let event = root.join("event0");
        fs::create_dir_all(event.join("device/capabilities")).unwrap();
        fs::write(event.join("device/name"), "Test Keyboard\n").unwrap();
        fs::write(event.join("device/phys"), "usb-test/input0\n").unwrap();
        fs::write(event.join("device/capabilities/ev"), "3\n").unwrap();
        fs::write(
            event.join("device/capabilities/key"),
            format!("{:x}\n", 1u64 << KEY_A),
        )
        .unwrap();
        let devices = discover_evdev_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_name, "Test Keyboard");
        assert_eq!(devices[0].physical_path, Some("usb-test/input0".into()));
        assert_eq!(devices[0].kind, InputDeviceKind::Keyboard);
        let _ = fs::remove_dir_all(root);
    }

    // ----- parse_product_id -----

    #[test]
    fn parse_product_id_partial_fields() {
        let (bus, vendor, product, version) = parse_product_id("0003/046D");
        assert_eq!(bus, Some(0x0003));
        assert_eq!(vendor, Some(0x046d));
        assert_eq!(product, None);
        assert_eq!(version, None);
    }

    #[test]
    fn parse_product_id_empty_string() {
        let (bus, vendor, product, version) = parse_product_id("");
        assert_eq!(bus, None);
        assert_eq!(vendor, None);
        assert_eq!(product, None);
        assert_eq!(version, None);
    }

    #[test]
    fn parse_product_id_invalid_hex() {
        let (bus, _vendor, _product, _version) = parse_product_id("zzz/046D/085E/0111");
        assert_eq!(bus, None);
    }
}
