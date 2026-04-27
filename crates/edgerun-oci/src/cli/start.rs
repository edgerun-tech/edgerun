//! Start command implementation.
//!
//! Signals the container child to start via FIFO, runs cgroups and poststart hooks,
//! updates state to "running", then returns immediately.
//! The container continues running in the background.
//! Poststop and cleanup only happen in the `delete` command.

use crate::prelude::*;
use std::fs;
use std::io;

use crate::json::parse_oci_spec;
use crate::lifecycle::{
    run_poststart_hooks, setup_container_cgroups, signal_start, update_state_running,
};
use crate::state::load_state;

pub fn cmd_start(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;
    if state.status != "created" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "container {} is not in 'created' state (status: {})",
                id, state.status
            ),
        ));
    }

    let pid = state.pid.unwrap_or(0);
    let bundle = state.bundle.clone();

    // Load spec for cgroups and poststart hooks
    let config_path = std::path::Path::new(&bundle).join("config.json");
    let spec = if let Ok(data) = fs::read(&config_path) {
        parse_oci_spec(&data).ok()
    } else {
        None
    };

    // Cgroups — MUST happen before signal_start so limits are in place
    // when the workload begins executing.
    if let Some(ref spec) = spec {
        if let Some(ref linux) = spec.linux {
            if let Some(ref resources) = linux.resources {
                let raw_cgroup_path = linux.cgroups_path.as_deref().unwrap_or("");
                let rootless = !crate::state::is_root();
                let cgroup_path =
                    crate::rootless::resolve_container_cgroup_path(rootless, raw_cgroup_path)
                        .unwrap_or_else(|e| {
                            let _ = std::fs::write(
                                "/dev/kmsg",
                                format!("edgerun: cgroup resolution failed: {}", e),
                            );
                            raw_cgroup_path.to_string()
                        });
                setup_container_cgroups(pid, resources, &cgroup_path);
            }
        }
    }

    // Signal the FIFO to unblock the child
    signal_start(id)?;

    // Poststart hooks (runtime namespace)
    if let Some(ref spec) = spec {
        run_poststart_hooks(spec, id, pid)?;
    }

    // Update state to "running"
    update_state_running(id, pid)?;

    // Return immediately — the container runs in the background.
    // Poststop and cleanup happen in `delete`.

    Ok(())
}
