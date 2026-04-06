//! Shared sysfs utilities for Linux backend crates.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};

use std::time::{SystemTime, UNIX_EPOCH};

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn ioctl(fd: c_int, request: libc::c_ulong, ...) -> c_int;
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
// Parsing helpers
// ---------------------------------------------------------------------------

#[must_use]
pub fn parse_hex_u16(text: Option<String>) -> Option<u16> {
    text.and_then(|v| u16::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

#[must_use]
pub fn parse_hex_u32(text: Option<String>) -> Option<u32> {
    text.and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}

/// Parse a hex u32 from a `&str` directly (useful when chaining with `and_then`
/// on an `Option<String>` where you want to avoid an extra allocation).
#[must_use]
pub fn parse_hex_u32_from_str(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    u32::from_str_radix(trimmed, 16).ok()
}

#[must_use]
pub fn parse_hex_u8(text: Option<String>) -> Option<u8> {
    text.and_then(|v| u8::from_str_radix(v.trim_start_matches("0x"), 16).ok())
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
pub fn open_ioctl_socket() -> Result<c_int, String> {
    let fd = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(format!(
            "failed to open ioctl socket: {}",
            io::Error::last_os_error()
        ));
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
pub unsafe fn ioctl_call(fd: c_int, request: libc::c_ulong, arg: *mut std::ffi::c_void) -> c_int {
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

/// Create a unique temporary directory under the system temp dir.
/// `prefix` is the test-name portion (e.g. `"lifegraph-linux-pci"`).
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
