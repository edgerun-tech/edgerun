//! Start command implementation.
//!
/// Sends the start signal to a created container via the FIFO.

use std::fs;
use std::io::Write;
use std::io;

use crate::state::{load_state, save_state, fifo_path};

pub fn cmd_start(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    let id = args.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    let mut state = load_state(id)?;
    if state.status != "created" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("container {} is not in 'created' state (status: {})", id, state.status)));
    }

    // Open the FIFO for writing — this unblocks the child's blocking read
    let fifo = fifo_path(id);
    let mut fifo_file = fs::File::create(&fifo)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to open start FIFO: {}", e)))?;
    let _ = fifo_file.write_all(b"go\n");
    let _ = fifo_file.flush();
    // Keep the FIFO open briefly to ensure the reader gets the data
    std::thread::sleep(std::time::Duration::from_millis(100));

    state.status = "running".to_string();
    save_state(&state, id)?;
    Ok(())
}
