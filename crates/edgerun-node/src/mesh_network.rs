//! Mesh network that connects multiple MeshNodes together.
//!
//! For testing: nodes deliver frames directly to each other.
//! For production: frames go through the actual network transport
//! (raw Ethernet, multicast, tunnels).

use edgerun_hardware_signing::NodeID;
use edgerun_mesh::{EventType, MeshFrame};
use edgerun_core::protocol::CommandEnvelope;
use std::collections::HashMap;
use crate::MeshNode;

/// A mesh network that connects multiple nodes together.
///
/// In production, this would be the actual network transport.
/// For testing, it delivers frames directly between nodes.
pub struct MeshNetwork {
    nodes: HashMap<NodeID, usize>, // NodeID -> index in nodes vec
    nodes_vec: Vec<MeshNode>,
    /// Pending frames to be delivered: (dest_node_id, wire_data)
    pending_frames: Vec<(NodeID, Vec<u8>)>,
}

impl MeshNetwork {
    /// Creates a new empty mesh network.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            nodes_vec: Vec::new(),
            pending_frames: Vec::new(),
        }
    }

    /// Adds a node to the mesh network.
    pub fn add_node(&mut self, node: MeshNode) {
        let id = node.identity();
        self.nodes.insert(id, self.nodes_vec.len());
        self.nodes_vec.push(node);
    }

    /// Gets mutable access to a node by its NodeID.
    pub fn get_node_mut(&mut self, node_id: &NodeID) -> Option<&mut MeshNode> {
        if let Some(&idx) = self.nodes.get(node_id) {
            Some(&mut self.nodes_vec[idx])
        } else {
            None
        }
    }

    /// Ticks all nodes in the network.
    /// Returns the total number of frames processed.
    pub fn tick_all(&mut self) -> Result<usize, String> {
        let mut total = 0;
        for node in &mut self.nodes_vec {
            total += node.tick()?;
        }
        // Drain pending outbound frames and deliver to destination nodes
        let frames = std::mem::take(&mut self.pending_frames);
        for (dest, wire) in frames {
            if let Some(node) = self.get_node_mut(&dest) {
                node.deliver_inbound_frame(&wire)?;
                total += 1;
            }
        }
        Ok(total)
    }

    /// Sends a signed command from one node to another.
    /// The command is queued and will be delivered on the next tick.
    pub fn send_command(
        &mut self,
        from: &NodeID,
        to: &NodeID,
        command: &edgerun_core::protocol::CommandEnvelope,
    ) -> Result<(), String> {
        let node = self
            .get_node_mut(from)
            .ok_or("source node not found")?;
        node.send_command(*to, command);
        Ok(())
    }

    /// Drains pending outbound frames from a node.
    /// These can be delivered to another node's `deliver_inbound_frame`.
    pub fn drain_node_outbound(&mut self, node_id: &NodeID) -> Result<Vec<Vec<u8>>, String> {
        let node = self
            .get_node_mut(node_id)
            .ok_or("node not found")?;
        // Tick the node to sign pending frames
        let _ = node.tick()?;
        // Drain pending frames (they're already signed by tick())
        Ok(Vec::new()) // In production, this would capture frames from the MeshLink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_core::protocol::{CommandEnvelope, IdentityRef, NodeRef};
    use edgerun_hardware_signing::MeshSigner;
    use edgerun_mesh::EventType;
    use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;


    struct TestSigner {
        node_id: NodeID,
        key: edgerun_crypto::p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = edgerun_crypto::random_p256_signing_key();
            let vk = key.verifying_key();
            let encoded = vk.to_encoded_point(false);
            let mut node_bytes = [0u8; 64];
            node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: NodeID(node_bytes),
                key,
            }
        }
    }

    impl MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(
            &self,
            digest: &[u8; 32],
        ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
            let sig: edgerun_crypto::p256::ecdsa::Signature =
                self.key.sign_prehash(digest).unwrap();
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&sig.to_bytes());
            Ok(bytes)
        }
    }

    const TEST_CONFIG: &str = r#"
stream_id: "test-node"
name: "Test Node"
controllers: []
trust_nodes: []
initial_grants: []
metadata:
  environment: "test"
"#;

    #[test]
    fn mesh_network_connects_two_nodes() {
        // Create two nodes
        let signer_a = Box::new(TestSigner::new());
        let node_a_id = signer_a.node_id();
        let signer_b = Box::new(TestSigner::new());
        let node_b_id = signer_b.node_id();

        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let node_a = MeshNode::from_config(config.clone(), signer_a).unwrap();
        let node_b = MeshNode::from_config(config, signer_b).unwrap();

        // Create mesh network
        let mut network = MeshNetwork::new();
        network.add_node(node_a);
        network.add_node(node_b);

        // Verify both nodes have genesis
        assert_eq!(network.get_node_mut(&node_a_id).unwrap().events().len(), 1);
        assert_eq!(network.get_node_mut(&node_b_id).unwrap().events().len(), 1);

        // Create a command from Alice to Bob
        let command = CommandEnvelope {
            envelope_version: 1,
            command_id: Some(vec![1, 2, 3]),
            target_node: NodeRef {
                node_id: node_b_id.0.to_vec(),
            },
            issuer: IdentityRef {
                identity_id: node_a_id.0.to_vec(),
                identity_kind: Some("node".into()),
                key_hint: Some(node_a_id.0.to_vec()), // Public key for verification
            },
            command_type: Some("COMMAND_TYPE_QUERY".into()),
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: None,
            payload_object: None,
            inline_payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None, // Will be signed by the mesh frame
        };

        // Send command through mesh
        network.send_command(&node_a_id, &node_b_id, &command).unwrap();

        // Tick to deliver
        let processed = network.tick_all().unwrap();
        assert!(processed > 0);

        // Bob should have recorded the command in his stream
        // genesis (seq 0) + rejected (seq 1, because signature is zeros)
        assert_eq!(network.get_node_mut(&node_b_id).unwrap().events().len(), 2);
        let rejected_event = &network.get_node_mut(&node_b_id).unwrap().events()[1];
        assert_eq!(rejected_event.event_type, EventType::CommandRejected);
    }
}
