//! Start command implementation.
//!
//! Signals the container child to start via FIFO, runs cgroups and poststart hooks,
//! then waits for the process to exit and updates state to "stopped".

use std::fs;
use std::io;

use crate::state::load_state;
use crate::json::parse_oci_spec;
use crate::lifecycle::{signal_start, setup_container_cgroups, run_poststart_hooks, update_state_running, run_poststop_and_cleanup};
use crate::cli::is_process_alive;

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
    let spec = if let Ok(data) = fs::read(&config_path) {
        parse_oci_spec(&data).ok()
    } else {
        None
    };

    // Cgroups
    if let Some(ref spec) = spec {
        if let Some(ref linux) = spec.linux {
            if let Some(ref resources) = linux.resources {
                let cgroup_path = linux.cgroups_path.as_deref().unwrap_or("/edgerun");
                setup_container_cgroups(pid, resources, cgroup_path);
            }
        }
    }

    // Poststart hooks (runtime namespace)
    if let Some(ref spec) = spec {
        run_poststart_hooks(spec, id, pid)?;
    }

    // Update state to "running"
    update_state_running(id, pid)?;

    // Wait for the process to exit by polling (can't use waitpid for non-child)
    for _ in 0..300 {
        if !is_process_alive(pid) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Update state to "stopped"
    if let Ok(mut existing) = load_state(id) {
        existing.status = "stopped".to_string();
        existing.pid = Some(pid);
        let _ = crate::state::save_state(&existing, id);
    }

    // Run poststop hooks and cleanup
    if let Some(ref spec) = spec {
        let cgroup_path = spec.linux.as_ref()
            .and_then(|l| l.cgroups_path.as_ref())
            .cloned()
            .unwrap_or_else(|| "/edgerun".into());
        run_poststop_and_cleanup(id, pid, &bundle, &cgroup_path, spec);
    }

    // Don't delete state — the test harness may query it for "stopped" status
    // The `delete` command handles cleanup

    Ok(())
}
