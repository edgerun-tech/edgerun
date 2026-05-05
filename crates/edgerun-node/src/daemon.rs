//! Daemon runtime entrypoint.
//!
//! The legacy daemon command/query byte path was removed. `run` remains an
//! explicit failure until the daemon is wired to the single rkyv session and
//! command boundary.

use std::net::SocketAddr;
use std::path::Path;

pub async fn cmd_run(
    config: &Path,
    listen: Option<SocketAddr>,
    health_port: Option<u16>,
    is_init: bool,
) {
    edgerun_log::error!(
        "edgerund run is unavailable until the rkyv session boundary is wired \
         (config={}, listen={:?}, health_port={:?}, init_mode={})",
        config.display(),
        listen,
        health_port,
        is_init
    );
    std::process::exit(2);
}
