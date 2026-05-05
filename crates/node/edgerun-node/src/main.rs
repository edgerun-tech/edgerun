//! edgerun Node Daemon (edgerund)
//!
//! Runs a single-writer stream node with mesh networking, command processing,
//! and capability discovery.

extern crate alloc;

mod capacity;
mod command_dispatch;
mod command_dispatch_event;
mod command_dispatch_result;
mod hardware;
mod ingress;
mod init;
mod protocol_signer;
mod stream_append;

// Extracted modules
mod cli;
mod config;
mod features_cmd;
mod health;
mod init_cmd;
mod signer;
mod status_cmd;

mod provisioning_listener;

fn main() {
    cli::main()
}
