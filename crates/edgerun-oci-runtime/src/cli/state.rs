//! State command implementation.
//!
/// Outputs container state JSON to stdout.

use std::io;
use std::os::raw::c_int;

use crate::state::load_state;

pub fn cmd_state(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let id = args.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    let state = load_state(id)?;

    // Check if process is still alive
    let mut updated_state = state.clone();
    if let Some(pid) = state.pid {
        let alive = unsafe { libc::kill(pid as c_int, 0) == 0 };
        if !alive && state.status == "running" {
            updated_state.status = "stopped".to_string();
            let _ = crate::state::save_state(&updated_state, id);
        }
    }

    // Output matching rspecs.State JSON format
    let pid_val = updated_state.pid.unwrap_or(0);
    let output = serde_json::json!({
        "ociVersion": updated_state.oci_version,
        "id": updated_state.id,
        "status": updated_state.status,
        "pid": pid_val,
        "bundle": updated_state.bundle,
        "annotations": updated_state.annotations.unwrap_or_default(),
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
