//! Start command implementation.
//!
//! Signals the container child to start via FIFO, runs cgroups and poststart hooks,
//! updates state to "running", then returns immediately.
//! The container continues running in the background.
//! Poststop and cleanup only happen in the `delete` command.

use crate::prelude::*;
use std::fs;
use std::io;

use crate::lifecycle::start_created_container;
use crate::spec::parse_oci_spec;
use crate::state::load_state;

pub fn cmd_start(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

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

    if let Some(ref spec) = spec {
        start_created_container(spec, id, pid)?;
    } else {
        crate::lifecycle::signal_start(id)?;
        crate::lifecycle::update_state_running(id, pid)?;
    }

    // Return immediately — the container runs in the background.
    // Poststop and cleanup happen in `delete`.

    Ok(())
}
