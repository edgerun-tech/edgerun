//! Container lifecycle management — blocking and non-blocking execution.
//!
//! This module delegates to:
//! - `lifecycle` — Hook-aware create/start/delete with composable steps
//! - `process`   — Child process pre_exec setup
//! - `handle`    — RunningContainer handle

use std::fs;
use std::io;
use std::path::Path;

use crate::json::OciSpec;
pub use crate::lifecycle::{
    run_spec, run_spec_with_id, start_spec, start_spec_with_id,
    run_prestart_hooks, run_create_runtime_hooks, fork_container_child,
    ForkedChild, save_created_state, signal_start, setup_container_cgroups,
    run_poststart_hooks, update_state_running, into_running_container,
    run_poststop_and_cleanup,
};
pub use crate::handle::RunningContainer;

/// Delete a container and run poststop hooks.
pub fn delete_container(container: RunningContainer) {
    crate::lifecycle::delete_container(container);
}

/// Run an OCI bundle (directory containing config.json + rootfs/).
pub fn run_bundle(bundle_path: &Path) -> io::Result<std::process::ExitStatus> {
    let config_path = bundle_path.join("config.json");
    let config_data = fs::read(&config_path)?;
    let spec: OciSpec = crate::json::parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    run_spec(&spec)
}

/// Start a container from an OCI bundle without blocking.
pub fn start_bundle(bundle_path: &Path) -> io::Result<RunningContainer> {
    let config_path = bundle_path.join("config.json");
    let config_data = fs::read(&config_path)?;
    let spec: OciSpec = crate::json::parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;
    start_spec(&spec)
}
