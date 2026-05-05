//! Bridges mesh decrypted frames to the node's store task.
//!
//! The `MeshDaemon` calls the command handler callback when it receives
//! a decrypted frame that is NOT a capability envelope. Command/query decoding
//! remains disabled until the single rkyv boundary is wired here.
//!
//! There is no separate "mesh store"; once rkyv command/query decoding is
//! restored, mesh and TCP must feed the same store task and signed event stream.

use edgerun_hardware_signing::NodeID;

// ===========================================================================
// Mesh command handler — factory function for the daemon callback
// ===========================================================================

/// Creates a command handler closure for decrypted mesh frames.
pub fn make_mesh_command_handler(
    _store_tx: edgerun_rt::mpsc::Sender<crate::types::StoreRequest>,
) -> impl FnMut(NodeID, Vec<u8>) + Send + 'static {
    move |peer_id, _decrypted| {
        edgerun_log::debug!(
            "mesh payload from {:?} dropped: command/query rkyv decoder is not wired",
            peer_id
        );
    }
}
