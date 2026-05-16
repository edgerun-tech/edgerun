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
use edgerun_json::ToJson;

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
    let json = edgerun_json::to_string_pretty(&state.to_json())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
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
    let tape = edgerun_json::parse_json_tape(data).map_err(|error| error.to_string())?;
    let root = tape
        .root(data)
        .ok_or_else(|| "container state missing JSON root".to_string())?;
    let annotations = root.get_object_fields("annotations").map(|fields| {
        fields
            .into_iter()
            .filter_map(|(key, value)| {
                value
                    .as_str()
                    .map(|value| (key.to_string(), value.to_string()))
            })
            .collect()
    });
    Ok(ContainerState {
        oci_version: root
            .required_string("ociVersion")
            .map_err(|_| "missing field `ociVersion`".to_string())?,
        id: root
            .required_string("id")
            .map_err(|_| "missing field `id`".to_string())?,
        status: root
            .required_string("status")
            .map_err(|_| "missing field `status`".to_string())?,
        pid: root.get_u32("pid"),
        bundle: root
            .required_string("bundle")
            .map_err(|_| "missing field `bundle`".to_string())?,
        annotations,
    })
}

edgerun_json::impl_json_struct! {
    ContainerState {
        required {
            oci_version: "ociVersion" => String,
            id: "id" => String,
            status: "status" => String,
            bundle: "bundle" => String,
        }
        optional {
            pid: "pid" => u32,
            annotations: "annotations" => BTreeMap<String, String>,
        }
    }
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
