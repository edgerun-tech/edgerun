//! edgerun Node Daemon (edgerund)
//!
//! Runs a single-writer stream node with mesh networking, command processing,
//! and capability discovery.

// Existing modules
mod app_package_wire_codec;
mod assurance;
mod capabilities;
mod capacity;
mod command_dispatch;
mod command_dispatch_event;
mod command_dispatch_payload;
mod command_dispatch_result;
mod command_query_wire_codec;
mod command_result_wire_codec;
// mod metering;      // TODO: file missing — not needed for interface boundary
// mod running_workloads; // TODO: file missing — not needed for interface boundary
// mod workload_policy;  // TODO: file missing — not needed for interface boundary
mod hardware;
mod ingress;
mod init;
mod server_resource_dispatch;
mod server_resource_wire_codec;
mod server_resources;
mod session;
mod stream_append;

// Extracted modules
mod cli;
mod config;
mod daemon;
mod health;
mod init_cmd;
mod mesh_store_provider;
mod peer_reconnect;
mod query_engine;
mod signer;
mod status_cmd;
mod store_task;
mod tcp_server;

mod provisioning_listener;

mod types;

fn main() {
    cli::main()
}
