//! edgerun Node Daemon (edgerund)
//!
//! Runs a single-writer stream node with mesh networking, command processing,
//! and capability discovery.

mod capacity;
mod command_dispatch;
mod command_dispatch_event;
mod command_dispatch_result;
// mod metering;      // TODO: file missing — not needed for interface boundary
// mod running_workloads; // TODO: file missing — not needed for interface boundary
// mod workload_policy;  // TODO: file missing — not needed for interface boundary
mod hardware;
mod ingress;
mod init;
mod stream_append;

// Extracted modules
mod cli;
mod config;
mod health;
mod init_cmd;
mod signer;
mod status_cmd;

mod provisioning_listener;

fn main() {
    cli::main()
}
