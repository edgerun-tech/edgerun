//! edgerun Node Daemon (edgerund)
//!
//! Runs a single-writer stream node with mesh networking, command processing,
//! and capability discovery.

extern crate alloc;

pub use edgerun_node::{node_debug, node_error, node_info, node_trace, node_warn};

mod bind_check;
mod bootstrap;
mod capacity;
mod command_dispatch;
mod command_dispatch_event;
mod command_dispatch_result;
mod ingress;
mod init;
mod protocol_signer;
mod stream_append;

mod cli;
mod health;
mod init_cmd;
mod signer;
mod status_cmd;

mod provisioning_listener;

fn main() {
    cli::main()
}
