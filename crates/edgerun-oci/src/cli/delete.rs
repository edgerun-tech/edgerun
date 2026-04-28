//! Delete command implementation.
//!
//! Kills the container process if running, runs poststop hooks, and cleans up.

use crate::prelude::*;
use std::fs;
use std::io;
use std::time::Duration;

use crate::cli::process_tree::{signal_tree, wait_tree_dead};
use crate::cli::{is_process_alive, parse_delete_args};
use crate::json::parse_oci_spec;
use crate::lifecycle::run_poststop_and_cleanup;
use crate::state::{delete_state, fifo_path, load_state, runtime_spec_path};

pub fn cmd_delete(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let (force, id) = parse_delete_args(args);
    let id =
        id.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container ID required"))?;

    let state = load_state(id).ok();
    let (pid, bundle, _cgroup_path) = if let Some(ref s) = state {
        let p = s.pid.unwrap_or(0);
        let b = s.bundle.clone();
        let mut st = s.status.clone();
        if p > 0 && st == "running" && !is_process_alive(p) {
            st = "stopped".to_string();
        }

        // OCI spec: delete MUST generate an error if container is not stopped
        // unless --force is used
        if st != "stopped" && !force {
            if st == "running" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("container {} is still running, use --force", id),
                ));
            }
            if st == "created" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "container {} is not stopped (status: {}), use --force",
                        id, st
                    ),
                ));
            }
        }

        // Kill if alive (force or non-stopped)
        if p > 0 && is_process_alive(p) && (force || st != "stopped") {
            signal_tree(p, libc::SIGKILL);
            let _ = wait_tree_dead(p, Duration::from_secs(2));
        }

        (p, b, String::new())
    } else {
        (0, String::new(), String::new())
    };

    // Load spec for poststop hooks and cgroup path
    let spec = if !bundle.is_empty() {
        load_runtime_or_bundle_spec(id, &bundle)
    } else {
        None
    };

    // Poststop hooks + cgroup cleanup
    if let Some(ref spec) = spec {
        let linux = spec.linux.clone().unwrap_or_default();
        let cgroup = linux
            .cgroups_path
            .clone()
            .unwrap_or_else(|| "/edgerun".into());
        run_poststop_and_cleanup(id, pid, &bundle, &cgroup, spec);
    }

    // Clean up state dir
    delete_state(id);

    // Clean up FIFO
    let _ = fs::remove_file(fifo_path(id));

    Ok(())
}

fn load_runtime_or_bundle_spec(id: &str, bundle: &str) -> Option<crate::json::OciSpec> {
    fs::read(runtime_spec_path(id))
        .ok()
        .and_then(|data| parse_oci_spec(&data).ok())
        .or_else(|| {
            let config_path = std::path::Path::new(bundle).join("config.json");
            fs::read(config_path)
                .ok()
                .and_then(|data| parse_oci_spec(&data).ok())
        })
}
