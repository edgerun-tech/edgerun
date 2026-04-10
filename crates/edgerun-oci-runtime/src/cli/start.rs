//! Start command implementation.
//!
/// Sends the start signal to a created container via the FIFO,
/// then runs poststart hooks.

use std::fs;
use std::io::Write;
use std::io;

use crate::state::{load_state, save_state, fifo_path};
use crate::json::parse_oci_spec;
use crate::hooks::{ContainerState, execute_poststart_hooks};

pub fn cmd_start(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;
    if state.status != "created" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("container {} is not in 'created' state (status: {})", id, state.status)));
    }

    let pid = state.pid.unwrap_or(0);
    let bundle = state.bundle.clone();

    // Open the FIFO for writing — this unblocks the child's blocking read
    let fifo = fifo_path(id);
    let mut fifo_file = fs::File::create(&fifo)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to open start FIFO: {}", e)))?;
    let _ = fifo_file.write_all(b"go\n");
    let _ = fifo_file.flush();
    // Keep the FIFO open briefly to ensure the reader gets the data
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Load poststart hooks from bundle's config.json
    let poststart_hooks = {
        let config_path = std::path::Path::new(&bundle).join("config.json");
        if let Ok(data) = fs::read(&config_path) {
            if let Ok(spec) = parse_oci_spec(&data) {
                spec.linux.as_ref()
                    .and_then(|l| l.hooks.as_ref())
                    .and_then(|h| h.poststart.clone())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    // Run poststart hooks with running state
    let hook_state = ContainerState {
        version: state.oci_version.clone(),
        id: state.id.clone(),
        status: "running".to_string(),
        pid,
        bundle: state.bundle.clone(),
        annotations: state.annotations.clone().unwrap_or_default(),
    };

    if let Err(e) = execute_poststart_hooks(Some(&poststart_hooks), &hook_state) {
        // Kill the container if poststart hook fails
        if pid > 0 {
            unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("poststart hook failed: {}", e),
        ));
    }

    let mut updated_state = state;
    updated_state.status = "running".to_string();
    save_state(&updated_state, id)?;
    Ok(())
}
