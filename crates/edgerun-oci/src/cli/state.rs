//! State command implementation.
//!
//! Outputs container state JSON to stdout.

use crate::prelude::*;
use std::io;

use crate::state::load_state;

pub fn cmd_state(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

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
    let annotations = edgerun_json::JsonValue::Object(edgerun_json::Map::from(
        updated_state
            .annotations
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| (key, edgerun_json::JsonValue::String(value)))
            .collect::<Vec<_>>(),
    ));
    let output = edgerun_json::json!({
        "ociVersion": updated_state.oci_version,
        "id": updated_state.id,
        "status": updated_state.status,
        "pid": pid_val,
        "bundle": updated_state.bundle,
        "annotations": annotations,
    });
    println!(
        "{}",
        edgerun_json::to_string_pretty(&output)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    );
    Ok(())
}
