//! Delete command implementation.
//!
/// Cleans up container state and optionally kills running process.

use std::fs;
use std::io;
use std::os::raw::c_int;

use crate::state::{load_state, delete_state, fifo_path};
use crate::cli::{parse_delete_args, is_process_alive};
use crate::hooks::{ContainerState, execute_poststop_hooks};
use crate::json::{OciSpec, parse_oci_spec};

pub fn cmd_delete(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let (force, id) = parse_delete_args(args);
    let id = id.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    // Load hooks from bundle's config.json if available
    let (poststop_hooks, cgroup_path) = {
        let state = load_state(&id).ok();
        let (hooks, cgroup) = state.as_ref().and_then(|s| {
            if s.bundle.is_empty() {
                return None;
            }
            let config_path = std::path::Path::new(&s.bundle).join("config.json");
            let data = fs::read(&config_path).ok()?;
            let spec: OciSpec = parse_oci_spec(&data).ok()?;
            let hooks = spec.linux.as_ref()
                .and_then(|l| l.hooks.as_ref())
                .and_then(|h| h.poststop.clone())
                .unwrap_or_default();
            let cgroup = spec.linux.as_ref()
                .and_then(|l| l.cgroups_path.clone())
                .unwrap_or_else(|| "/edgerun".into());
            Some((hooks, cgroup))
        }).unwrap_or_default();
        (hooks, cgroup)
    };

    if let Ok(state) = load_state(&id) {
        if let Some(pid) = state.pid {
            let alive = is_process_alive(pid);
            if alive && state.status == "running" && !force {
                return Err(io::Error::new(io::ErrorKind::InvalidInput,
                    format!("container {} is still running, use --force", id)));
            }
            if alive {
                unsafe { libc::kill(pid as c_int, libc::SIGKILL) };
                // Wait briefly for exit
                unsafe { libc::usleep(50000) };
            }
        }

        // Run poststop hooks with container state
        let hook_state = ContainerState {
            version: state.oci_version,
            id: state.id,
            status: "stopped".to_string(),
            pid: state.pid.unwrap_or(0),
            bundle: state.bundle,
            annotations: state.annotations.unwrap_or_default(),
        };
        execute_poststop_hooks(Some(&poststop_hooks), &hook_state);
    }

    // Clean up state dir and FIFO
    delete_state(&id);

    // Clean up FIFO
    let fifo = fifo_path(&id);
    let _ = fs::remove_file(&fifo);

    // Clean up cgroup directory
    if !cgroup_path.is_empty() {
        let cgroup_dir = std::path::Path::new("/sys/fs/cgroup").join(
            cgroup_path.trim_start_matches('/'),
        );
        if cgroup_dir.exists() {
            let _ = fs::remove_dir_all(&cgroup_dir);
        }
    }

    Ok(())
}
