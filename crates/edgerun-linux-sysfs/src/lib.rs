//! Shared sysfs utilities for Linux backend crates.

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
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

#[cfg(target_os = "none")]
pub mod collections {
    pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
}

#[cfg(target_os = "none")]
pub mod env {
    use crate::path::PathBuf;

    #[must_use]
    pub fn temp_dir() -> PathBuf {
        PathBuf::from("/tmp")
    }
}

#[cfg(target_os = "none")]
pub mod ffi {
    pub use core::ffi::*;
}

#[cfg(target_os = "none")]
pub mod fs {
    use crate::{io, path::PathBuf};
    use alloc::string::String;

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

    pub fn read_link<P>(_path: P) -> io::Result<PathBuf> {
        Err(io::Error::new(io::ErrorKind::NotFound))
    }

    pub fn canonicalize<P: Into<PathBuf>>(path: P) -> io::Result<PathBuf> {
        Ok(path.into())
    }

    pub fn create_dir_all<P>(_path: P) -> io::Result<()> {
        Ok(())
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

        #[must_use]
        pub const fn from_raw_os_error(_code: i32) -> Self {
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

#[cfg(target_os = "none")]
pub mod os {
    pub mod raw {
        #[allow(non_camel_case_types)]
        pub type c_char = i8;
        #[allow(non_camel_case_types)]
        pub type c_int = i32;
        #[allow(non_camel_case_types)]
        pub type c_short = i16;
        #[allow(non_camel_case_types)]
        pub type c_ulong = usize;
    }
}

#[cfg(target_os = "none")]
pub mod path {
    use alloc::string::{String, ToString};

    #[derive(Debug)]
    pub struct Path;

    impl Path {
        #[must_use]
        pub fn new(_path: &str) -> &'static Self {
            static PATH: Path = Path;
            &PATH
        }

        #[must_use]
        pub fn file_name(&self) -> Option<PathBuf> {
            None
        }

        #[must_use]
        pub fn join<P>(&self, _path: P) -> PathBuf {
            PathBuf::from("")
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
        pub fn join<P>(&self, _path: P) -> Self {
            Self(String::new())
        }

        #[must_use]
        pub fn exists(&self) -> bool {
            false
        }

        #[must_use]
        pub fn is_dir(&self) -> bool {
            false
        }

        #[must_use]
        pub fn file_name(&self) -> Option<PathBuf> {
            None
        }

        #[must_use]
        pub fn to_string_lossy(&self) -> String {
            self.0.clone()
        }

        #[must_use]
        pub fn to_str(&self) -> Option<&str> {
            Some(&self.0)
        }

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

    impl From<&PathBuf> for PathBuf {
        fn from(value: &PathBuf) -> Self {
            value.clone()
        }
    }

    impl core::ops::Deref for PathBuf {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            Path::new("")
        }
    }

    pub struct Display<'a>(&'a PathBuf);

    impl core::fmt::Display for Display<'_> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(&self.0.0)
        }
    }
}

#[cfg(target_os = "none")]
pub mod time {
    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SystemTime(Duration);

    pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

    impl SystemTime {
        #[must_use]
        pub const fn now() -> Self {
            UNIX_EPOCH
        }

        pub fn duration_since(
            &self,
            earlier: SystemTime,
        ) -> core::result::Result<Duration, Duration> {
            self.0.checked_sub(earlier.0).ok_or(earlier.0)
        }
    }
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
use std::collections::HashMap;
#[cfg(not(target_os = "none"))]
use std::fs;
#[cfg(not(target_os = "none"))]
use std::io;
use std::os::raw::{c_char, c_int, c_ulong};
use std::path::{Path, PathBuf};

use std::time::{SystemTime, UNIX_EPOCH};

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// File reading
// ---------------------------------------------------------------------------

/// Read a sysfs file and return the trimmed contents, or `None` on error.
#[must_use]
pub fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|v| v.trim().to_string())
}

// ---------------------------------------------------------------------------
// Parsing helpers — delegates hex parsing to edgerun-encoding
// ---------------------------------------------------------------------------

#[must_use]
pub fn parse_hex_u16(text: Option<String>) -> Option<u16> {
    text.as_deref()
        .and_then(|v| edgerun_encoding::hex::parse_hex_int(v.trim_start_matches("0x")))
}

#[must_use]
pub fn parse_hex_u32(text: Option<String>) -> Option<u32> {
    text.as_deref()
        .and_then(|v| edgerun_encoding::hex::parse_hex_int(v.trim_start_matches("0x")))
}

/// Parse a hex u32 from a `&str` directly (useful when chaining with `and_then`
/// on an `Option<String>` where you want to avoid an extra allocation).
#[must_use]
pub fn parse_hex_u32_from_str(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    edgerun_encoding::hex::parse_hex_int(trimmed.strip_prefix("0x").unwrap_or(trimmed))
}

#[must_use]
pub fn parse_hex_u8(text: Option<String>) -> Option<u8> {
    text.as_deref()
        .and_then(|v| edgerun_encoding::hex::parse_hex_int(v.trim_start_matches("0x")))
}

#[must_use]
pub fn parse_u32(text: Option<String>) -> Option<u32> {
    text.and_then(|v| v.parse::<u32>().ok())
}

#[must_use]
pub fn parse_i32(text: Option<String>) -> Option<i32> {
    text.and_then(|v| v.parse::<i32>().ok())
}

#[must_use]
pub fn parse_u8(text: Option<String>) -> Option<u8> {
    text.and_then(|v| v.parse::<u8>().ok())
}

#[must_use]
pub fn parse_u64(text: Option<String>) -> Option<u64> {
    text.and_then(|v| v.parse::<u64>().ok())
}

/// Parse a boolean flag. Returns `Some(true)` / `Some(false)` when the value
/// is recognised, `None` otherwise.
#[must_use]
pub fn parse_bool_flag(text: Option<String>) -> Option<bool> {
    text.as_deref().and_then(|value| match value {
        "1" | "y" | "yes" | "true" | "enabled" | "online" => Some(true),
        "0" | "n" | "no" | "false" | "disabled" | "offline" => Some(false),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// Uevent parsing
// ---------------------------------------------------------------------------

#[must_use]
pub fn parse_uevent_map(path: &Path) -> HashMap<String, String> {
    let Some(raw) = fs::read_to_string(path).ok() else {
        return HashMap::new();
    };
    raw.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

#[must_use]
pub fn link_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
}

#[must_use]
pub fn is_pci_address(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 12 && bytes[4] == b':' && bytes[7] == b':' && bytes[10] == b'.'
}

// ---------------------------------------------------------------------------
// ioctl socket
// ---------------------------------------------------------------------------

const AF_INET: c_int = 2;
const SOCK_DGRAM: c_int = 2;

/// Open a UDP datagram socket suitable for ioctl calls.
pub fn open_ioctl_socket() -> Result<c_int, std::io::Error> {
    let fd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(fd)
}

/// Close an ioctl socket file descriptor.
///
/// # Safety
/// `fd` must be a valid file descriptor returned from `open_ioctl_socket`.
pub unsafe fn close_ioctl_fd(fd: c_int) {
    close(fd);
}

/// Perform an ioctl call.
///
/// # Safety
/// `fd` must be a valid file descriptor, and `request`/`...` must match
/// the expected ioctl signature for the given fd.
pub unsafe fn ioctl_call(fd: c_int, request: c_ulong, arg: *mut std::ffi::c_void) -> c_int {
    ioctl(fd, request, arg)
}

/// Fill the first 16 bytes of an ifreq-style name buffer.
pub fn fill_ifr_name(dst: &mut [c_char; 16], name: &str) {
    for (dst, src) in dst.iter_mut().zip(name.as_bytes().iter().copied()) {
        *dst = src as c_char;
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build parent-child relationships for a list of devices.
///
/// For each device, validates that the parent reference points to a known device,
/// clears orphaned parent references, and populates child lists.
///
/// # Arguments
/// * `devices` - Mutable slice of devices to update
/// * `get_key` - Closure to extract the device's unique key
/// * `get_parent` - Closure to extract the optional parent key reference
/// * `clear_parent` - Closure to clear the parent reference on a device
/// * `set_children` - Closure to set the children list on a device
pub fn build_parent_child_relationships<T>(
    devices: &mut [T],
    get_key: impl Fn(&T) -> &str,
    get_parent: impl Fn(&T) -> Option<&str>,
    clear_parent: impl Fn(&mut T),
    set_children: impl Fn(&mut T, Vec<String>),
) {
    let known: std::collections::HashSet<String> =
        devices.iter().map(|d| get_key(d).to_string()).collect();

    // Clear orphaned parent references
    for device in &mut *devices {
        if !get_parent(device).is_some_and(|p| known.contains(p)) {
            clear_parent(device);
        }
    }

    // Build children map
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for device in &*devices {
        if let Some(parent) = get_parent(device) {
            children
                .entry(parent.to_string())
                .or_default()
                .push(get_key(device).to_string());
        }
    }

    // Assign sorted children to each device
    for device in &mut *devices {
        if let Some(ids) = children.get(get_key(device)) {
            let mut ids = ids.clone();
            ids.sort();
            set_children(device, ids);
        }
    }

    // Sort devices by key
    devices.sort_by(|a, b| get_key(a).cmp(get_key(b)));
}

/// Create a unique temporary directory under the system temp dir.
/// `prefix` is the test-name portion (e.g. `"edgerun-linux-pci"`).
#[must_use]
pub fn temp_root(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&root).unwrap();
    root
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- read_trimmed -----

    #[test]
    fn read_trimmed_returns_trimmed_content() {
        let dir = temp_root("sysfs-test");
        let file = dir.join("value");
        fs::write(&file, "  hello  \n").unwrap();
        assert_eq!(read_trimmed(&file), Some("hello".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_trimmed_returns_none_for_missing_file() {
        assert_eq!(read_trimmed(Path::new("/nonexistent/path")), None);
    }

    #[test]
    fn read_trimmed_handles_exact_numeric_string() {
        let dir = temp_root("sysfs-test");
        let file = dir.join("num");
        fs::write(&file, "42\n").unwrap();
        assert_eq!(read_trimmed(&file), Some("42".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    // ----- parse_hex_u16 -----

    #[test]
    fn parse_hex_u16_with_0x_prefix() {
        assert_eq!(parse_hex_u16(Some("0x1a2b".to_string())), Some(0x1a2b));
    }

    #[test]
    fn parse_hex_u16_without_0x_prefix() {
        assert_eq!(parse_hex_u16(Some("ff".to_string())), Some(0xff));
    }

    #[test]
    fn parse_hex_u16_with_leading_zeros() {
        assert_eq!(parse_hex_u16(Some("0x00ff".to_string())), Some(0xff));
    }

    #[test]
    fn parse_hex_u16_none_on_invalid() {
        assert_eq!(parse_hex_u16(Some("zzz".to_string())), None);
    }

    #[test]
    fn parse_hex_u16_none_on_empty_input() {
        assert_eq!(parse_hex_u16(None), None);
    }

    #[test]
    fn parse_hex_u16_max_value() {
        assert_eq!(parse_hex_u16(Some("ffff".to_string())), Some(u16::MAX));
    }

    // ----- parse_hex_u32 -----

    #[test]
    fn parse_hex_u32_with_0x_prefix() {
        assert_eq!(
            parse_hex_u32(Some("0xdeadbeef".to_string())),
            Some(0xdeadbeef)
        );
    }

    #[test]
    fn parse_hex_u32_without_0x_prefix() {
        assert_eq!(parse_hex_u32(Some("abcdef".to_string())), Some(0xabcdef));
    }

    #[test]
    fn parse_hex_u32_none_on_invalid() {
        assert_eq!(parse_hex_u32(Some("xyz".to_string())), None);
    }

    // ----- parse_hex_u32_from_str -----

    #[test]
    fn parse_hex_u32_from_str_with_0x_prefix() {
        assert_eq!(parse_hex_u32_from_str("0xff"), Some(0xff));
    }

    #[test]
    fn parse_hex_u32_from_str_without_0x_prefix() {
        assert_eq!(parse_hex_u32_from_str("abc"), Some(0xabc));
    }

    #[test]
    fn parse_hex_u32_from_str_invalid() {
        assert_eq!(parse_hex_u32_from_str("not_hex"), None);
    }

    #[test]
    fn parse_hex_u32_from_str_with_whitespace() {
        assert_eq!(parse_hex_u32_from_str("  0x42  "), Some(0x42));
    }

    // ----- parse_hex_u8 -----

    #[test]
    fn parse_hex_u8_valid() {
        assert_eq!(parse_hex_u8(Some("ff".to_string())), Some(0xff));
    }

    #[test]
    fn parse_hex_u8_with_0x_prefix() {
        assert_eq!(parse_hex_u8(Some("0x1a".to_string())), Some(0x1a));
    }

    #[test]
    fn parse_hex_u8_none_on_overflow() {
        assert_eq!(parse_hex_u8(Some("1ff".to_string())), None);
    }

    #[test]
    fn parse_hex_u8_none_on_none_input() {
        assert_eq!(parse_hex_u8(None), None);
    }

    // ----- parse_u32 -----

    #[test]
    fn parse_u32_valid() {
        assert_eq!(parse_u32(Some("12345".to_string())), Some(12345));
    }

    #[test]
    fn parse_u32_none_on_invalid() {
        assert_eq!(parse_u32(Some("abc".to_string())), None);
    }

    #[test]
    fn parse_u32_none_on_none_input() {
        assert_eq!(parse_u32(None), None);
    }

    #[test]
    fn parse_u32_zero() {
        assert_eq!(parse_u32(Some("0".to_string())), Some(0));
    }

    // ----- parse_i32 -----

    #[test]
    fn parse_i32_positive() {
        assert_eq!(parse_i32(Some("999".to_string())), Some(999));
    }

    #[test]
    fn parse_i32_negative() {
        assert_eq!(parse_i32(Some("-42".to_string())), Some(-42));
    }

    #[test]
    fn parse_i32_none_on_invalid() {
        assert_eq!(parse_i32(Some("abc".to_string())), None);
    }

    #[test]
    fn parse_i32_none_on_none_input() {
        assert_eq!(parse_i32(None), None);
    }

    // ----- parse_u8 -----

    #[test]
    fn parse_u8_valid() {
        assert_eq!(parse_u8(Some("255".to_string())), Some(255));
    }

    #[test]
    fn parse_u8_none_on_overflow() {
        assert_eq!(parse_u8(Some("256".to_string())), None);
    }

    #[test]
    fn parse_u8_none_on_negative() {
        assert_eq!(parse_u8(Some("-1".to_string())), None);
    }

    #[test]
    fn parse_u8_none_on_none_input() {
        assert_eq!(parse_u8(None), None);
    }

    // ----- parse_u64 -----

    #[test]
    fn parse_u64_valid() {
        assert_eq!(
            parse_u64(Some("18446744073709551615".to_string())),
            Some(u64::MAX)
        );
    }

    #[test]
    fn parse_u64_none_on_invalid() {
        assert_eq!(parse_u64(Some("abc".to_string())), None);
    }

    #[test]
    fn parse_u64_zero() {
        assert_eq!(parse_u64(Some("0".to_string())), Some(0));
    }

    // ----- parse_bool_flag -----

    #[test]
    fn parse_bool_flag_true_variants() {
        for val in &["1", "y", "yes", "true", "enabled", "online"] {
            assert_eq!(
                parse_bool_flag(Some(val.to_string())),
                Some(true),
                "expected true for '{}'",
                val
            );
        }
    }

    #[test]
    fn parse_bool_flag_false_variants() {
        for val in &["0", "n", "no", "false", "disabled", "offline"] {
            assert_eq!(
                parse_bool_flag(Some(val.to_string())),
                Some(false),
                "expected false for '{}'",
                val
            );
        }
    }

    #[test]
    fn parse_bool_flag_none_on_unknown() {
        assert_eq!(parse_bool_flag(Some("maybe".to_string())), None);
    }

    #[test]
    fn parse_bool_flag_none_on_none_input() {
        assert_eq!(parse_bool_flag(None), None);
    }

    // ----- parse_uevent_map -----

    #[test]
    fn parse_uevent_map_parses_key_value_pairs() {
        let dir = temp_root("sysfs-test");
        let file = dir.join("uevent");
        fs::write(&file, "DRIVER=foo\nMODALIAS=bar\n").unwrap();
        let map = parse_uevent_map(&file);
        assert_eq!(map.get("DRIVER"), Some(&"foo".to_string()));
        assert_eq!(map.get("MODALIAS"), Some(&"bar".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_uevent_map_returns_empty_on_missing_file() {
        let map = parse_uevent_map(Path::new("/nonexistent/uevent"));
        assert!(map.is_empty());
    }

    #[test]
    fn parse_uevent_map_ignores_lines_without_equals() {
        let dir = temp_root("sysfs-test");
        let file = dir.join("uevent");
        fs::write(&file, "KEY=value\nno_equals_here\n").unwrap();
        let map = parse_uevent_map(&file);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("KEY"), Some(&"value".to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_uevent_map_handles_empty_file() {
        let dir = temp_root("sysfs-test");
        let file = dir.join("uevent");
        fs::write(&file, "").unwrap();
        let map = parse_uevent_map(&file);
        assert!(map.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    // ----- link_name -----

    #[test]
    fn link_name_extracts_filename() {
        let path = PathBuf::from("/sys/class/input/event0");
        assert_eq!(link_name(&path), Some("event0".to_string()));
    }

    #[test]
    fn link_name_none_on_root() {
        assert_eq!(link_name(Path::new("/")), None);
    }

    // ----- is_pci_address -----

    #[test]
    fn is_pci_address_valid_format() {
        assert!(is_pci_address("0000:03:00.0"));
        assert!(is_pci_address("0001:0a:1f.3"));
    }

    #[test]
    fn is_pci_address_invalid_length() {
        assert!(!is_pci_address("000:03:00.0"));
        assert!(!is_pci_address("00000:03:00.0"));
    }

    #[test]
    fn is_pci_address_wrong_separators() {
        assert!(!is_pci_address("0000-03-00-0"));
        assert!(!is_pci_address("0000/03/00/0"));
    }

    #[test]
    fn is_pci_address_empty_string() {
        assert!(!is_pci_address(""));
    }

    // ----- fill_ifr_name -----

    #[test]
    fn fill_ifr_name_pads_with_zeros() {
        let mut buf = [0i8; 16];
        fill_ifr_name(&mut buf, "eth0");
        // First 4 bytes should be 'e', 't', 'h', '0'
        assert_eq!(buf[0] as u8, b'e');
        assert_eq!(buf[1] as u8, b't');
        assert_eq!(buf[2] as u8, b'h');
        assert_eq!(buf[3] as u8, b'0');
        // Remaining bytes should be zero
        for b in &buf[4..] {
            assert_eq!(*b, 0);
        }
    }

    #[test]
    fn fill_ifr_name_exact_15_chars() {
        let mut buf = [0i8; 16];
        let name = "abcdefghijklmno"; // 15 chars + null
        fill_ifr_name(&mut buf, name);
        assert_eq!(
            &buf[0..15],
            name.as_bytes()
                .iter()
                .map(|&b| b as i8)
                .collect::<Vec<i8>>()
                .as_slice()
        );
        assert_eq!(buf[15], 0);
    }

    #[test]
    fn fill_ifr_name_truncates_long_name() {
        let mut buf = [0i8; 16];
        fill_ifr_name(&mut buf, "this_is_a_very_long_interface_name");
        // Only first 16 bytes should be written
        assert_eq!(buf[0] as u8, b't');
        assert_eq!(buf[15] as u8, b'l'); // "this_is_a_very_l"
    }

    #[test]
    fn fill_ifr_name_empty_string() {
        let mut buf: [i8; 16] = [42i8; 16];
        fill_ifr_name(&mut buf, "");
        // Empty string means no iteration happens, buffer is unchanged
        for b in &buf {
            assert_eq!(*b, 42);
        }
    }

    // ----- temp_root -----

    #[test]
    fn temp_root_creates_directory() {
        let path = temp_root("test-prefix");
        assert!(path.exists());
        assert!(path.is_dir());
        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn temp_root_has_unique_name_each_call() {
        let a = temp_root("unique-test");
        let b = temp_root("unique-test");
        assert_ne!(a, b);
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }
}
