//! Delete command implementation.
//!
/// Cleans up container state and optionally kills running process.

use std::io;
use std::os::raw::c_int;

use crate::state::{load_state, delete_state};
use crate::cli::{parse_delete_args, is_process_alive};

pub fn cmd_delete(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let (force, id) = parse_delete_args(args);
    let id = id.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    if let Ok(state) = load_state(id) {
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
    }

    // Clean up state dir and FIFO
    delete_state(id);
    Ok(())
}
