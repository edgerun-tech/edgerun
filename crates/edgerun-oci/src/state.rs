//! Container state management — persistence, loading, and FIFO helpers.
//!
//! Handles the JSON state files stored in the state directory.
//! When running as root: `/run/edgerun-oci/<id>/`
//! When running rootless: `$XDG_RUNTIME_DIR/edgerun-oci/<id>/` or `$HOME/.local/state/edgerun-oci/<id>/`

use crate::libc;
use crate::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicPtr, Ordering};

/// Base directory for container state when running as root.
pub const STATE_DIR: &str = "/run/edgerun-oci";

/// Custom state directory override (set via `set_state_dir`).
static CUSTOM_STATE_DIR: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Check if the current process is running as root (UID 0).
pub fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

/// Check if we are running in a rootless user namespace.
///
/// This is set by the re-exec in `main()` when a non-root user starts `ert`.
/// The `_ERT_ROOTLESS_CHILD` env var is set after `clone3(CLONE_NEWUSER|CLONE_NEWNS)`
/// and `setresuid(0)`, so even though `geteuid()` returns 0, we're still rootless.
pub fn is_rootless() -> bool {
    std::env::var("_ERT_ROOTLESS_CHILD").is_ok()
}

/// Check if runtime behavior should use rootless assumptions.
pub fn is_rootless_mode() -> bool {
    !is_root() || is_rootless()
}

/// Resolve the default state directory based on whether we're rootless.
///
/// Root: `/run/edgerun-oci`
/// Rootless: `$XDG_RUNTIME_DIR/edgerun-oci` or `$HOME/.local/state/edgerun-oci`
fn default_state_dir() -> String {
    if is_root() && !is_rootless() {
        return STATE_DIR.into();
    }
    // Rootless: prefer XDG_RUNTIME_DIR, fall back to HOME/.local/state
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        return format!("{}/edgerun-oci", xdg);
    }
    if let Ok(home) = std::env::var("HOME") {
        return format!("{}/.local/state/edgerun-oci", home);
    }
    // Last resort: temp dir
    format!("{}/edgerun-oci", std::env::temp_dir().display())
}

/// Override the state directory path.
///
/// This must be called before any container operations.
/// The provided string is leaked intentionally — it's used for the lifetime
/// of the process.
pub fn set_state_dir(dir: &str) {
    let boxed = Box::new(dir.to_string());
    let ptr = Box::into_raw(boxed) as *mut std::ffi::c_void;
    // Free the old one if any
    let old = CUSTOM_STATE_DIR.swap(ptr, Ordering::Relaxed);
    if !old.is_null() {
        drop(unsafe { Box::from_raw(old as *mut String) });
    }
}

#[cfg(test)]
fn clear_state_dir_override() {
    let old = CUSTOM_STATE_DIR.swap(std::ptr::null_mut(), Ordering::Relaxed);
    if !old.is_null() {
        drop(unsafe { Box::from_raw(old as *mut String) });
    }
}

/// Resolve the state directory base path.
fn state_dir_base() -> std::borrow::Cow<'static, str> {
    let ptr = CUSTOM_STATE_DIR.load(Ordering::Relaxed);
    if !ptr.is_null() {
        let s = unsafe { &*(ptr as *const String) };
        std::borrow::Cow::Borrowed(s.as_str())
    } else {
        std::borrow::Cow::Owned(default_state_dir())
    }
}

/// Return the current container state root directory.
pub fn state_root_dir() -> PathBuf {
    let base = state_dir_base();
    PathBuf::from(base.as_ref())
}

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

/// Container state matching the OCI runtime spec JSON format.
#[derive(Debug, Clone)]
pub struct ContainerState {
    pub oci_version: String,
    pub id: String,
    pub status: String, // "creating" | "created" | "running" | "stopped"
    pub pid: Option<u32>,
    pub bundle: String,
    pub annotations: Option<BTreeMap<String, String>>,
}

/// Return the state directory for a container.
pub fn container_state_dir(id: &str) -> PathBuf {
    let base = state_dir_base();
    Path::new(base.as_ref()).join(id)
}

/// Return the path to the state JSON file.
pub fn state_file_path(id: &str) -> PathBuf {
    container_state_dir(id).join("state.json")
}

/// Return the path to the effective runtime spec captured for this container.
pub fn runtime_spec_path(id: &str) -> PathBuf {
    container_state_dir(id).join("runtime-config.json")
}

/// Return the path to the start FIFO file.
pub fn fifo_path(id: &str) -> PathBuf {
    container_state_dir(id).join("start.fifo")
}

/// Save container state to disk.
pub fn save_state(state: &ContainerState, id: &str) -> io::Result<()> {
    let dir = container_state_dir(id);
    fs::create_dir_all(&dir)?;
    let json = state_to_json_string_pretty(state);
    fs::write(state_file_path(id), json)?;
    Ok(())
}

/// Save the effective runtime OCI spec used to create the container.
pub fn save_runtime_spec(spec: &crate::spec::OciSpec, id: &str) -> io::Result<()> {
    let dir = container_state_dir(id);
    fs::create_dir_all(&dir)?;
    let json = crate::spec::spec_to_json_string_pretty(spec);
    fs::write(runtime_spec_path(id), json)?;
    Ok(())
}

/// Load container state from disk.
pub fn load_state(id: &str) -> io::Result<ContainerState> {
    let data = fs::read_to_string(state_file_path(id))?;
    load_state_from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn load_state_from_str(data: &str) -> Result<ContainerState, String> {
    Ok(ContainerState {
        oci_version: required_json_string_field(data, "ociVersion")?,
        id: required_json_string_field(data, "id")?,
        status: required_json_string_field(data, "status")?,
        pid: optional_json_u32_field(data, "pid")?,
        bundle: required_json_string_field(data, "bundle")?,
        annotations: optional_json_string_map_field(data, "annotations")?,
    })
}

fn state_to_json_string_pretty(state: &ContainerState) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"ociVersion\": ");
    write_json_string(&mut out, &state.oci_version);
    out.push_str(",\n  \"id\": ");
    write_json_string(&mut out, &state.id);
    out.push_str(",\n  \"status\": ");
    write_json_string(&mut out, &state.status);
    if let Some(pid) = state.pid {
        out.push_str(",\n  \"pid\": ");
        write!(&mut out, "{pid}").expect("writing to String cannot fail");
    }
    out.push_str(",\n  \"bundle\": ");
    write_json_string(&mut out, &state.bundle);
    if let Some(annotations) = &state.annotations {
        out.push_str(",\n  \"annotations\": {");
        for (index, (key, value)) in annotations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            write_json_string(&mut out, key);
            out.push_str(": ");
            write_json_string(&mut out, value);
        }
        if !annotations.is_empty() {
            out.push('\n');
            out.push_str("  ");
        }
        out.push('}');
    }
    out.push_str("\n}");
    out
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn required_json_string_field(data: &str, name: &str) -> Result<String, String> {
    match field_value_span(data, name)? {
        Some((start, end)) => parse_string_value(&data[start..end]),
        None => Err(format!("missing field `{name}`")),
    }
}

fn optional_json_u32_field(data: &str, name: &str) -> Result<Option<u32>, String> {
    let Some((start, end)) = field_value_span(data, name)? else {
        return Ok(None);
    };
    let value = data[start..end].trim();
    if value == "null" {
        return Ok(None);
    }
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("field `{name}` is not a u32"))?;
    Ok(Some(parsed))
}

fn optional_json_string_map_field(
    data: &str,
    name: &str,
) -> Result<Option<BTreeMap<String, String>>, String> {
    let Some((start, end)) = field_value_span(data, name)? else {
        return Ok(None);
    };
    let value = data[start..end].trim();
    if value == "null" {
        return Ok(None);
    }
    parse_string_map(value).map(Some)
}

fn field_value_span(data: &str, name: &str) -> Result<Option<(usize, usize)>, String> {
    let bytes = data.as_bytes();
    let mut index = skip_ws(bytes, 0);
    if bytes.get(index) != Some(&b'{') {
        return Err("container state root must be a JSON object".into());
    }
    index += 1;
    loop {
        index = skip_ws(bytes, index);
        match bytes.get(index) {
            Some(b'}') => return Ok(None),
            Some(b'"') => {}
            _ => return Err("expected JSON object key".into()),
        }
        let key_start = index;
        let key = parse_json_string_at(data, &mut index)?;
        index = skip_ws(bytes, index);
        if bytes.get(index) != Some(&b':') {
            return Err("expected `:` after JSON object key".into());
        }
        index += 1;
        index = skip_ws(bytes, index);
        let value_start = index;
        skip_json_value(data, &mut index)?;
        let value_end = index;
        if key == name {
            return Ok(Some((value_start, value_end)));
        }
        if key_start == index {
            return Err("JSON parser made no progress".into());
        }
        index = skip_ws(bytes, index);
        match bytes.get(index) {
            Some(b',') => index += 1,
            Some(b'}') => return Ok(None),
            _ => return Err("expected `,` or `}` after JSON object value".into()),
        }
    }
}

fn parse_string_map(data: &str) -> Result<BTreeMap<String, String>, String> {
    let bytes = data.as_bytes();
    let mut index = skip_ws(bytes, 0);
    if bytes.get(index) != Some(&b'{') {
        return Err("annotations must be a JSON object".into());
    }
    index += 1;
    let mut out = BTreeMap::new();
    loop {
        index = skip_ws(bytes, index);
        match bytes.get(index) {
            Some(b'}') => return Ok(out),
            Some(b'"') => {}
            _ => return Err("expected annotation key".into()),
        }
        let key = parse_json_string_at(data, &mut index)?;
        index = skip_ws(bytes, index);
        if bytes.get(index) != Some(&b':') {
            return Err("expected `:` after annotation key".into());
        }
        index += 1;
        index = skip_ws(bytes, index);
        let value = parse_json_string_at(data, &mut index)?;
        out.insert(key, value);
        index = skip_ws(bytes, index);
        match bytes.get(index) {
            Some(b',') => index += 1,
            Some(b'}') => return Ok(out),
            _ => return Err("expected `,` or `}` after annotation value".into()),
        }
    }
}

fn parse_string_value(data: &str) -> Result<String, String> {
    let mut index = skip_ws(data.as_bytes(), 0);
    let value = parse_json_string_at(data, &mut index)?;
    if skip_ws(data.as_bytes(), index) != data.len() {
        return Err("unexpected bytes after JSON string".into());
    }
    Ok(value)
}

fn parse_json_string_at(data: &str, index: &mut usize) -> Result<String, String> {
    let bytes = data.as_bytes();
    if bytes.get(*index) != Some(&b'"') {
        return Err("expected JSON string".into());
    }
    *index += 1;
    let mut out = String::new();
    while let Some(byte) = bytes.get(*index).copied() {
        match byte {
            b'"' => {
                *index += 1;
                return Ok(out);
            }
            b'\\' => {
                *index += 1;
                let escaped = bytes
                    .get(*index)
                    .copied()
                    .ok_or_else(|| "truncated JSON escape".to_string())?;
                *index += 1;
                match escaped {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{08}'),
                    b'f' => out.push('\u{0c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let code = parse_hex_u16(bytes, index)?;
                        let ch = char::from_u32(code as u32)
                            .ok_or_else(|| "invalid JSON unicode escape".to_string())?;
                        out.push(ch);
                    }
                    _ => return Err("invalid JSON escape".into()),
                }
            }
            0x00..=0x1f => return Err("unescaped control byte in JSON string".into()),
            _ => {
                let rest = data
                    .get(*index..)
                    .ok_or_else(|| "invalid string boundary".to_string())?;
                let ch = rest
                    .chars()
                    .next()
                    .ok_or_else(|| "truncated JSON string".to_string())?;
                out.push(ch);
                *index += ch.len_utf8();
            }
        }
    }
    Err("unterminated JSON string".into())
}

fn parse_hex_u16(bytes: &[u8], index: &mut usize) -> Result<u16, String> {
    let mut value = 0u16;
    for _ in 0..4 {
        let byte = bytes
            .get(*index)
            .copied()
            .ok_or_else(|| "truncated unicode escape".to_string())?;
        *index += 1;
        value = (value << 4)
            | match byte {
                b'0'..=b'9' => (byte - b'0') as u16,
                b'a'..=b'f' => (byte - b'a' + 10) as u16,
                b'A'..=b'F' => (byte - b'A' + 10) as u16,
                _ => return Err("invalid unicode escape".into()),
            };
    }
    Ok(value)
}

fn skip_json_value(data: &str, index: &mut usize) -> Result<(), String> {
    let bytes = data.as_bytes();
    *index = skip_ws(bytes, *index);
    match bytes.get(*index).copied() {
        Some(b'"') => parse_json_string_at(data, index).map(|_| ()),
        Some(b'{') => skip_json_object(data, index),
        Some(b'[') => skip_json_array(data, index),
        Some(b't') if data[*index..].starts_with("true") => {
            *index += 4;
            Ok(())
        }
        Some(b'f') if data[*index..].starts_with("false") => {
            *index += 5;
            Ok(())
        }
        Some(b'n') if data[*index..].starts_with("null") => {
            *index += 4;
            Ok(())
        }
        Some(b'-' | b'0'..=b'9') => {
            skip_json_number(bytes, index);
            Ok(())
        }
        _ => Err("expected JSON value".into()),
    }
}

fn skip_json_object(data: &str, index: &mut usize) -> Result<(), String> {
    let bytes = data.as_bytes();
    if bytes.get(*index) != Some(&b'{') {
        return Err("expected JSON object".into());
    }
    *index += 1;
    loop {
        *index = skip_ws(bytes, *index);
        if bytes.get(*index) == Some(&b'}') {
            *index += 1;
            return Ok(());
        }
        parse_json_string_at(data, index)?;
        *index = skip_ws(bytes, *index);
        if bytes.get(*index) != Some(&b':') {
            return Err("expected `:` after JSON object key".into());
        }
        *index += 1;
        skip_json_value(data, index)?;
        *index = skip_ws(bytes, *index);
        match bytes.get(*index) {
            Some(b',') => *index += 1,
            Some(b'}') => {
                *index += 1;
                return Ok(());
            }
            _ => return Err("expected `,` or object close".into()),
        }
    }
}

fn skip_json_array(data: &str, index: &mut usize) -> Result<(), String> {
    let bytes = data.as_bytes();
    if bytes.get(*index) != Some(&b'[') {
        return Err("expected JSON array".into());
    }
    *index += 1;
    loop {
        *index = skip_ws(bytes, *index);
        if bytes.get(*index) == Some(&b']') {
            *index += 1;
            return Ok(());
        }
        skip_json_value(data, index)?;
        *index = skip_ws(bytes, *index);
        match bytes.get(*index) {
            Some(b',') => *index += 1,
            Some(b']') => {
                *index += 1;
                return Ok(());
            }
            _ => return Err("expected `,` or array close".into()),
        }
    }
}

fn skip_json_number(bytes: &[u8], index: &mut usize) {
    while matches!(
        bytes.get(*index),
        Some(b'-' | b'+' | b'.' | b'0'..=b'9' | b'e' | b'E')
    ) {
        *index += 1;
    }
}

fn skip_ws(bytes: &[u8], mut index: usize) -> usize {
    while matches!(bytes.get(index), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        index += 1;
    }
    index
}

/// Delete container state directory and all contents.
pub fn delete_state(id: &str) {
    let _ = delete_state_with_result(id);
}

/// Delete container state directory and all contents, returning I/O errors.
pub fn delete_state_with_result(id: &str) -> io::Result<()> {
    let state_dir = container_state_dir(id);
    if !state_dir.exists() {
        return Ok(());
    }
    fs::remove_dir_all(state_dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to delete container state directory: {}", e),
        )
    })
}

/// Check if a container state exists.
pub fn state_exists(id: &str) -> bool {
    state_file_path(id).exists()
}
