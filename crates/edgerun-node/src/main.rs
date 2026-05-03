//! edgerun Node Daemon (edgerund)
//!
//! Runs a single-writer stream node with mesh networking,
//! command processing, and capability discovery.
//!
//! ## Usage
//! ```text
//! edgerund init --config node.yaml --software          # Dev-only: in-memory key
//! edgerund run --config node.yaml --listen 0.0.0.0:8080  // Start daemon with TCP
//! edgerund status --config node.yaml                   // Show node identity
//! ```
//!
//! ## Security
//! The node's private key NEVER leaves secure hardware. The config file only
//! stores the public key (NodeID) and a reference to the hardware key handle.
//! No `.key` file is ever written.

// Existing modules
mod assurance;
mod capabilities;
mod capacity;
mod command_dispatch;
// mod metering;      // TODO: file missing — not needed for interface boundary
// mod running_workloads; // TODO: file missing — not needed for interface boundary
// mod workload_policy;  // TODO: file missing — not needed for interface boundary
mod hardware;
mod ingress;
mod init;
mod server_resources;
mod session;

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
use command_dispatch::sign_event_envelope;

fn main() {
    cli::main()
}
