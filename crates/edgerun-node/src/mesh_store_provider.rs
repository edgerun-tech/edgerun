//! Bridges mesh decrypted frames to the node's store task.
//!
//! The `MeshDaemon` calls the command handler callback when it receives
//! a decrypted frame that is NOT a capability envelope. This callback
//! decodes the raw bytes as a `CommandEnvelope` or `QueryRequest` and
//! forwards it through the existing store task channel.
//!
//! There is no separate "mesh store" — commands from mesh and TCP share
//! the same event stream. The store task stores the event first, then
//! executes. Nothing happens before the event is stored.

use edgerun_hardware_signing::NodeID;
use edgerun_mesh_daemon::OutboundQueue;
use prost::Message;

use crate::types::{StoreRequest, StoreResponse};

// ===========================================================================
// Mesh command handler — factory function for the daemon callback
// ===========================================================================

/// Creates a command handler closure that decodes encrypted mesh frames
/// and forwards them to the store task through the same channel used by TCP.
///
/// Fix #6: Mesh commands are fire-and-forget — no oneshot reply channel
/// is allocated, avoiding wasted Arc + store task computation for nobody.
///
/// The handler uses `edgerun_rt::spawn` to bridge from the blocking daemon
/// thread to the async channel, since `store_tx.send()` is async.
pub fn make_mesh_command_handler(
    store_tx: edgerun_rt::mpsc::Sender<StoreRequest>,
) -> impl FnMut(NodeID, Vec<u8>) + Send + 'static {
    move |peer_id, decrypted| {
        let tx = store_tx.clone();
        let peer_id_bytes = peer_id.0.to_vec();

        edgerun_rt::spawn(async move {
            let raw = decrypted.clone();
            if let Ok(command) =
                edgerun_core::protocol::CommandEnvelope::decode(&decrypted[..])
            {
                // Fire-and-forget — no reply channel needed.
                let _ = tx
                    .send(StoreRequest::Command {
                        raw_bytes: raw,
                        command,
                        peer_id: Some(peer_id_bytes),
                        reply_tx: None,
                    })
                    .await;
                return;
            }

            let raw = decrypted.clone();
            if let Ok(query) =
                edgerun_core::protocol::QueryRequest::decode(&decrypted[..])
            {
                // Fire-and-forget — no reply channel needed.
                let _ = tx
                    .send(StoreRequest::Query {
                        raw_bytes: raw,
                        query,
                        peer_id: Some(peer_id_bytes),
                        reply_tx: None,
                    })
                    .await;
            }
        });
    }
}

// ===========================================================================
// Mesh command bridge — store task sends outbound commands to mesh peers
// ===========================================================================

/// Bridge for sending outbound commands to mesh peers.
/// Pushes serialized protobuf onto the MeshDaemon's encrypted outbound queue.
pub struct MeshCommandBridge {
    outbound: OutboundQueue,
}

impl MeshCommandBridge {
    pub fn new(outbound: OutboundQueue) -> Self {
        Self { outbound }
    }

    /// Sends a serialized protobuf to a mesh peer.
    pub fn send(&self, peer_id: NodeID, payload: Vec<u8>) -> Result<(), String> {
        self.outbound.lock().unwrap().push_back((peer_id, payload));
        Ok(())
    }
}
