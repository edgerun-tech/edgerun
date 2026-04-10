//! Start command implementation.
//!
//! Signals the container child to start, runs cgroups and poststart hooks.

use std::fs;
use std::io;

use crate::state::load_state;
use crate::json::parse_oci_spec;
use crate::lifecycle::{signal_start, setup_container_cgroups, run_poststart_hooks, update_state_running};

pub fn cmd_start(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;
    if state.status != "created" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("container {} is not in 'created' state (status: {})", id, state.status)));
    }

    let pid = state.pid.unwrap_or(0);
    let bundle = state.bundle.clone();

    // Signal the FIFO to unblock the child
    signal_start(id)?;

    // Load spec for cgroups and poststart hooks
    let config_path = std::path::Path::new(&bundle).join("config.json");
    if let Ok(data) = fs::read(&config_path) {
        if let Ok(spec) = parse_oci_spec(&data) {
            // Cgroups
            if let Some(ref linux) = spec.linux {
                if let Some(ref resources) = linux.resources {
                    let cgroup_path = linux.cgroups_path.as_deref().unwrap_or("/edgerun");
                    setup_container_cgroups(pid, resources, cgroup_path);
                }
            }

            // Poststart hooks (runtime namespace)
            run_poststart_hooks(&spec, id, pid)?;
        }
    }

    // Update state to "running"
    update_state_running(id, pid)?;

    Ok(())
}
