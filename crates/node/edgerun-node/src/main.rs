//! edgerun Node Daemon (edged)
//!
//! Runs a single-writer stream node with mesh networking, command processing,
//! and capability discovery.

extern crate alloc;

#[cfg(not(target_arch = "wasm32"))]
pub use edgerun_node::{node_debug, node_error, node_info, node_trace, node_warn};

#[cfg(not(target_arch = "wasm32"))]
mod bind_check;
#[cfg(not(target_arch = "wasm32"))]
mod bootstrap;
#[cfg(not(target_arch = "wasm32"))]
mod capacity;
#[cfg(not(target_arch = "wasm32"))]
mod ingress;
#[cfg(not(target_arch = "wasm32"))]
mod init;
#[cfg(not(target_arch = "wasm32"))]
mod protocol_signer;

#[cfg(not(target_arch = "wasm32"))]
mod cli;
#[cfg(not(target_arch = "wasm32"))]
mod health;
#[cfg(not(target_arch = "wasm32"))]
mod init_cmd;
#[cfg(not(target_arch = "wasm32"))]
mod signer;
#[cfg(not(target_arch = "wasm32"))]
mod status_cmd;

#[cfg(not(target_arch = "wasm32"))]
mod provisioning_listener;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    cli::main()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("edged is a native daemon; browser builds use the edgerun-node library wasm ABI")
}
