//! Container state management — persistence, loading, and FIFO helpers.
//!
//! Handles the JSON state files stored in `/run/edgerun-oci/<id>/`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Base directory for container state.
pub const STATE_DIR: &str = "/run/edgerun-oci";

/// Container state matching the OCI runtime spec JSON format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerState {
    #[serde(rename = "ociVersion")]
    pub oci_version: String,
    pub id: String,
    pub status: String, // "creating" | "created" | "running" | "stopped"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub bundle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

/// Return the state directory for a container.
pub fn container_state_dir(id: &str) -> PathBuf {
    Path::new(STATE_DIR).join(id)
}

/// Return the path to the state JSON file.
pub fn state_file_path(id: &str) -> PathBuf {
    container_state_dir(id).join("state.json")
}

/// Return the path to the start FIFO file.
pub fn fifo_path(id: &str) -> PathBuf {
    container_state_dir(id).join("start.fifo")
}

/// Save container state to disk.
pub fn save_state(state: &ContainerState, id: &str) -> io::Result<()> {
    let dir = container_state_dir(id);
    fs::create_dir_all(&dir)?;
    let json = edgerun_json::to_string_pretty(state).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(state_file_path(id), json)?;
    Ok(())
}

/// Load container state from disk.
pub fn load_state(id: &str) -> io::Result<ContainerState> {
    let data = fs::read_to_string(state_file_path(id))?;
    edgerun_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Delete container state directory and all contents.
pub fn delete_state(id: &str) {
    let _ = fs::remove_dir_all(container_state_dir(id));
}

/// Check if a container state exists.
pub fn state_exists(id: &str) -> bool {
    state_file_path(id).exists()
}
