//! Start command implementation.
//!
//! Signals the container child to start via FIFO, runs cgroups and poststart hooks,
//! updates state to "running", then returns immediately.
//! The container continues running in the background.
//! Poststop and cleanup only happen in the `delete` command.

use crate::prelude::*;
use std::io;

use crate::lifecycle::start_created_container;
use crate::state::load_state;

pub fn cmd_start(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let id = crate::cli::parse_container_id_args(args, "start")?;
    let id = id.as_str();

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

    let pid = state.pid.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("container {id} state missing pid"),
        )
    })?;
    let bundle = state.bundle.clone();

    let spec = crate::cli::load_runtime_or_bundle_spec(id, &bundle)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("container {id} has no readable OCI spec to start"),
            )
        })?;

    start_created_container(&spec, id, pid)?;

    // Return immediately — the container runs in the background.
    // Poststop and cleanup happen in `delete`.

    Ok(())
}
