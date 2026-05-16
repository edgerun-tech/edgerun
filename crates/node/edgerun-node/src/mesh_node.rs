//! Mesh-connected edgerun Node.
//!
//! Ties together:
//! - `Node` (local audit log + legacy command processing + capability grants)
//! - `MeshLink` (network transport)
//! - `MeshRouter` (routing, discovery)
//!
//! This is compatibility mesh glue around node-local command streams. Authoritative
//! network work should converge on `edgerun-work` admissions, routes, channels,
//! receipts, and proofs.
//!
//! ## Event Loop
//!
//! ```text
//! tick() {
//!   // 1. Read inbound frames from network
//!   for frame in mesh_link.drain_inbound_data_frames() {
//!     // 2. Route: is this frame for us?
//!     if frame.header.dest == our_node_id {
//!       // 3. Decode as legacy command envelope
//!       if let Some(command) = decode_command(&frame) {
//!         // 4. Process through node (validate + record local audit result)
//!         let _ = node.process_command(&command);
//!       }
//!     } else {
//!       // Forward to next-hop
//!       if let Some(next_hop) = router.next_hop_for(&frame.header.dest) {
//!         mesh_link.queue_frame(frame_with_dest(frame, next_hop));
//!       }
//!     }
//!   }
//!
//!   // 5. Send queued outbound frames
//!   mesh_link.drain_pending_frames(&mut router)?;
//! }
//! ```

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::module_path;

use crate::command_dispatch::{ControllerSet, dispatch_command};
use crate::{Node, NodeConfig};
use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_mesh::{LocalNode, MeshFrame, MeshLink, MeshRouter};
use edgerun_protocols::core_protocol::collections::{HashMap, HashSet};
use edgerun_protocols::core_protocol::command::CommandExecutionContext;
use edgerun_protocols::core_protocol::protocol::{CommandEnvelope, EventEnvelope};
use edgerun_protocols::core_protocol::wire_stream::{
    command_full_wire_bytes, decode_command_full_wire_bytes,
};
use edgerun_storage::NodeStore;

/// A mesh-connected edgerun node.
///
/// Combines the node's local audit log, legacy command processing, and
/// capability grant store with the mesh network transport layer.
pub struct MeshNode {
    /// The node's local audit log and command processor.
    node: Node,
    /// Durable storage required before commands may affect node state.
    store: Option<NodeStore>,
    /// Controller projection used by storage-backed command dispatch.
    controllers: ControllerSet,
    /// In-memory replay hint backed by the durable replay index.
    replay_cache: HashMap<Vec<u8>, (Vec<u8>, i64)>,
    /// Delegation revocations projected from durable events.
    revoked_delegations: HashSet<Vec<u8>>,
    /// The mesh network transport.
    mesh_link: MeshLink,
    /// The mesh router for discovery and routing.
    router: MeshRouter,
    /// Hardware signer for signing outbound mesh frames.
    signer: Arc<dyn MeshSigner>,
}

impl MeshNode {
    /// Creates a new mesh-connected node from native construction input.
    ///
    /// Initializes:
    /// - The node's local audit log with the genesis event
    /// - The mesh network link (raw sockets on UP interfaces)
    /// - The mesh router with discovery
    pub fn from_config(config: NodeConfig, signer: Box<dyn MeshSigner>) -> Result<Self, String> {
        let identity = signer.node_id();
        let signer_arc: Arc<dyn MeshSigner> = Arc::from(signer);
        let initial_controllers = config_controllers_to_ids(&config);
        let node = Node::from_config(config, Arc::clone(&signer_arc))
            .map_err(|e| format!("failed to create node: {}", e))?;
        let mut mesh_link = MeshLink::new();
        mesh_link.set_local_node_id(identity);
        let router = MeshRouter::new(LocalNode::new(identity));

        Ok(Self {
            node,
            store: None,
            controllers: ControllerSet::new(initial_controllers),
            replay_cache: HashMap::new(),
            revoked_delegations: HashSet::new(),
            mesh_link,
            router,
            signer: signer_arc,
        })
    }

    /// Creates a mesh node with durable storage enabled.
    ///
    /// Command ingress is inert until a `NodeStore` is attached through this
    /// constructor or `attach_store`.
    pub fn from_config_with_store(
        config: NodeConfig,
        signer: Box<dyn MeshSigner>,
        store: NodeStore,
    ) -> Result<Self, String> {
        let mut node = Self::from_config(config, signer)?;
        node.attach_store(store);
        Ok(node)
    }

    /// Attaches durable storage and enables command dispatch.
    pub fn attach_store(&mut self, store: NodeStore) {
        self.store = Some(store);
    }

    /// Runs one tick of the event loop.
    ///
    /// 1. Reads inbound frames from the network
    /// 2. Routes frames: if for us, process; otherwise forward
    /// 3. Sends queued outbound frames
    ///
    /// Returns the number of frames processed.
    pub fn tick(&mut self) -> Result<usize, String> {
        let mut processed = 0;
        let our_id = self.node.identity();

        // 1. Read inbound frames
        let frames = self.mesh_link.drain_inbound_data_frames();
        for frame in frames {
            // 2. Route: is this frame for us?
            if frame.header.dest == our_id {
                // Decode and process
                if let Some(command) = Self::decode_command(&frame) {
                    self.dispatch_stored_command(&command);
                    processed += 1;
                }
            } else {
                // Forward to next-hop
                if let Some(next_hop) = self.router.next_hop_for(&frame.header.dest) {
                    // Re-queue with next-hop destination for routing
                    let fwd_frame = Self::frame_with_dest(frame, next_hop);
                    self.mesh_link.queue_frame(fwd_frame);
                    processed += 1;
                }
            }
        }

        // 3. Send pending outbound frames (sign and transmit)
        self.mesh_link
            .drain_pending_frames(&mut self.router)
            .map_err(|e| format!("send failed: {}", e))?;

        Ok(processed)
    }

    /// Queues a command to be sent to a remote node.
    /// The command will be signed and sent on the next tick.
    pub fn send_command(&mut self, dest: NodeID, command: &CommandEnvelope) {
        let buf = command_full_wire_bytes(command);
        let mut frame = MeshFrame::from_payload(dest, buf);
        self.sign_frame(&mut frame);
        self.mesh_link.queue_frame(frame);
    }

    /// Signs a mesh frame using the node's hardware signer.
    fn sign_frame(&mut self, frame: &mut MeshFrame) {
        use edgerun_protocols::core_protocol::crypto::SIG_DOMAIN_MESH_FRAME;
        frame.header.src = self.node.identity();
        let preimage = frame.signed_preimage();
        let record_hash = edgerun_protocols::core_protocol::crypto::sha256(&preimage);
        let sig_input = edgerun_protocols::core_protocol::crypto::signature_input(
            SIG_DOMAIN_MESH_FRAME,
            &record_hash,
        );
        let digest = edgerun_protocols::core_protocol::crypto::sha256(&sig_input);
        let mut digest_bytes = [0u8; 32];
        digest_bytes.copy_from_slice(&digest);
        match self.signer.sign_digest(&digest_bytes) {
            Ok(sig) => {
                frame.signature = sig;
            }
            Err(e) => {
                crate::node_warn!("failed to sign mesh frame: {}", e);
                // Frame goes out unsigned — receiver will reject, but we don't block
            }
        }
    }

    /// Drains all pending outbound frames and returns them as raw wire data.
    /// These can be delivered to another node's `deliver_inbound_frame`.
    pub fn drain_outbound_frames(&mut self) -> Result<Vec<Vec<u8>>, String> {
        Ok(self
            .mesh_link
            .drain_pending_frames_raw()
            .into_iter()
            .map(|frame| frame.to_wire())
            .collect())
    }

    /// Delivers an inbound wire frame to this node for processing.
    /// The frame will be decoded, verified, and processed.
    pub fn deliver_inbound_frame(&mut self, wire: &[u8]) -> Result<usize, String> {
        let frame = MeshFrame::from_wire(wire).ok_or("invalid wire frame")?;
        self.mesh_link.inject_inbound_frame(frame);
        self.tick()
    }

    /// Returns the node's identity.
    pub fn identity(&self) -> NodeID {
        self.node.identity()
    }

    /// Returns the node's local audit stream projection.
    pub fn events(&self) -> &[EventEnvelope] {
        self.node.events()
    }

    /// Returns whether durable storage is attached.
    pub fn storage_ready(&self) -> bool {
        self.store.is_some()
    }

    /// Decodes a mesh frame as a command envelope.
    fn decode_command(frame: &MeshFrame) -> Option<CommandEnvelope> {
        decode_command_full_wire_bytes(&frame.payload[..]).ok()
    }

    fn dispatch_stored_command(&mut self, command: &CommandEnvelope) {
        let Some(store) = self.store.as_mut() else {
            crate::node_warn!("dropping inbound command before node storage is attached");
            return;
        };

        let trusted_roots = config_controllers_to_ids(self.node.config());
        let exec_ctx = CommandExecutionContext::test_default();
        let _ = dispatch_command(
            command,
            store,
            &self.node.identity().0,
            self.signer.as_ref(),
            &mut self.controllers,
            &mut self.replay_cache,
            &self.revoked_delegations,
            &trusted_roots,
            0,
            &exec_ctx,
        );
    }

    /// Creates a new frame with the given destination, preserving the payload.
    fn frame_with_dest(mut frame: MeshFrame, dest: NodeID) -> MeshFrame {
        frame.header.dest = dest;
        frame
    }

    /// Returns the mesh router for direct access.
    pub fn router_mut(&mut self) -> &mut MeshRouter {
        &mut self.router
    }
}

fn config_controllers_to_ids(config: &NodeConfig) -> Vec<Vec<u8>> {
    config
        .controllers
        .iter()
        .filter_map(|controller| {
            edgerun_protocols::core_protocol::util::hex_to_bytes(controller).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests;
