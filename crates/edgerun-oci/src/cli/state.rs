//! State command implementation.
//!
//! Outputs container state JSON to stdout.

use std::io;

use crate::state::load_state;

pub fn cmd_state(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "--root path is not valid UTF-8")
        })?);
    }

    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;

    // Check if process is still alive
    let mut updated_state = state.clone();
    if let Some(pid) = state.pid {
        if !crate::cli::is_process_alive(pid) && state.status == "running" {
            updated_state.status = "stopped".to_string();
            let _ = crate::state::save_state(&updated_state, id);
        }
    }

    // Output matching rspecs.State JSON format
    let pid_val = updated_state.pid.unwrap_or(0);
    let output = edgerun_json::json!({
        "ociVersion": updated_state.oci_version,
        "id": updated_state.id,
        "status": updated_state.status,
        "pid": pid_val,
        "bundle": updated_state.bundle,
        "annotations": updated_state.annotations.unwrap_or_default(),
    });
    println!("{}", edgerun_json::to_string_pretty(&output).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?);
    Ok(())
}
