//! State command implementation.
//!
//! Outputs container state JSON to stdout.

use crate::prelude::*;
use core::fmt::Write as _;
use std::io;

use crate::cli::json;
use crate::state::load_state;

pub fn cmd_state(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let id = crate::cli::parse_container_id_args(args, "state")?;
    let id = id.as_str();

    let state = load_state(id)?;

    // Check if process is still alive
    let mut updated_state = state.clone();
    if let Some(pid) = state.pid {
        if !crate::cli::is_process_alive(pid) && state.status == "running" {
            updated_state.status = "stopped".to_string();
            let _ = crate::state::save_state(&updated_state, id);
        }
    }

    println!("{}", state_output_json(&updated_state));
    Ok(())
}

fn state_output_json(state: &crate::state::ContainerState) -> String {
    let mut out = String::new();
    out.push_str("{\n  \"ociVersion\": ");
    json::write_string(&mut out, &state.oci_version);
    out.push_str(",\n  \"id\": ");
    json::write_string(&mut out, &state.id);
    out.push_str(",\n  \"status\": ");
    json::write_string(&mut out, &state.status);
    out.push_str(",\n  \"pid\": ");
    write!(&mut out, "{}", state.pid.unwrap_or(0)).expect("writing to String cannot fail");
    out.push_str(",\n  \"bundle\": ");
    json::write_string(&mut out, &state.bundle);
    out.push_str(",\n  \"annotations\": {");
    if let Some(annotations) = &state.annotations {
        for (index, (key, value)) in annotations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            json::write_string(&mut out, key);
            out.push_str(": ");
            json::write_string(&mut out, value);
        }
        if !annotations.is_empty() {
            out.push('\n');
            out.push_str("  ");
        }
    }
    out.push_str("}\n}");
    out
}
