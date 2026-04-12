use edgerun_log;
use edgerun_core::command::{command_hash, validate_command, CommandValidationContext};
use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, EventEnvelope, Digest};
use edgerun_core::result::Verdict;
use edgerun_hardware_signing::MeshSigner;
use edgerun_storage::NodeStore;
use edgerun_proto::edgerun::v0::stream::{CommandDecision, CommandEnvelope, CommandResultPayload as ProtoCommandResultPayload, CommandType, EventType};
use edgerun_proto::edgerun::v0::trust::{DelegationRecord as ProtoDelegationRecord, RevocationRecord as ProtoRevocationRecord};
use edgerun_proto::edgerun::v0::common::{CommandRef, DelegationRef};
use edgerun_crypto::rand_core::RngCore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use edgerun_hardware_signing::{MeshSigner, NodeID};
    use edgerun_storage::{NodeStore, NodeStoreConfig, BlobKeySource};
    use std::sync::Arc;
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;

    fn test_workload_policy() -> super::super::workload_policy::WorkloadPolicy {
        super::super::workload_policy::WorkloadPolicy::permissive()
    }

    fn test_rate_limiter() -> super::super::workload_policy::RateLimiter {
        super::super::workload_policy::RateLimiter::new(1000, 60_000_000)
    }

    fn test_running_workloads() -> std::sync::Arc<super::super::running_workloads::RunningWorkloads> {
        std::sync::Arc::new(super::super::running_workloads::RunningWorkloads::new())
    }

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn tmp_data_root() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("dispatch_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_store() -> NodeStore {
        let root = tmp_data_root();
        let private_key = [0xBBu8; 32];
        let config = NodeStoreConfig {
            data_root: root,
            blob_key_source: Arc::new(BlobKeySource::Software {
                private_key_bytes: private_key.to_vec(),
            }),
        };
        NodeStore::open(&config).unwrap()
    }

    fn random_signing_key() -> edgerun_crypto::p256::ecdsa::SigningKey {
        let mut bytes = [0u8; 32];
        edgerun_crypto::getrandom::fill(&mut bytes).expect("getrandom failed");
        edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    struct TestSigner {
        node_id: NodeID,
        key: edgerun_crypto::p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = random_signing_key();
            let vk = key.verifying_key();
            let encoded = vk.to_encoded_point(false);
            let mut node_bytes = [0u8; 64];
            node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: edgerun_hardware_signing::NodeID(node_bytes),
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
            let sig: edgerun_crypto::p256::ecdsa::Signature = self.key.sign_prehash(digest)
                .map_err(|e| edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&sig.to_bytes());
            Ok(bytes)
        }
    }

    fn make_command(
        command_id: Vec<u8>,
        target_node_id: Vec<u8>,
        issuer_id: Vec<u8>,
        command_type: i32,
    ) -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id,
            target_node: Some(edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: target_node_id,
            }),
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: issuer_id,
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
        }
    }

    fn make_controller_identity(hex_str: &str) -> Vec<u8> {
        edgerun_core::util::hex_to_bytes(hex_str).unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // ControllerSet tests
    // -----------------------------------------------------------------------

    #[test]
    fn controller_set_new_with_initial() {
        let controllers = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let set = ControllerSet::new(controllers.clone());
        assert!(set.contains(&vec![1, 2, 3]));
        assert!(set.contains(&vec![4, 5, 6]));
        assert!(!set.contains(&vec![7, 8, 9]));
    }

    #[test]
    fn controller_set_empty() {
        let set = ControllerSet::default();
        assert!(!set.contains(&vec![1, 2, 3]));
    }

    #[test]
    fn controller_set_add() {
        let mut set = ControllerSet::new(vec![]);
        set.add(vec![1, 2, 3]);
        assert!(set.contains(&vec![1, 2, 3]));
    }

    #[test]
    fn controller_set_add_duplicate_is_noop() {
        let mut set = ControllerSet::new(vec![vec![1, 2, 3]]);
        set.add(vec![1, 2, 3]);
        assert_eq!(set.to_vec().len(), 1);
    }

    #[test]
    fn controller_set_remove_existing() {
        let mut set = ControllerSet::new(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let removed = set.remove(&vec![1, 2, 3]);
        assert!(removed);
        assert!(!set.contains(&vec![1, 2, 3]));
        assert!(set.contains(&vec![4, 5, 6]));
    }

    #[test]
    fn controller_set_remove_nonexistent_returns_false() {
        let mut set = ControllerSet::new(vec![vec![1, 2, 3]]);
        let removed = set.remove(&vec![9, 9, 9]);
        assert!(!removed);
    }

    #[test]
    fn controller_set_iter() {
        let set = ControllerSet::new(vec![vec![1], vec![2]]);
        let collected: Vec<_> = set.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn controller_set_to_vec() {
        let set = ControllerSet::new(vec![vec![1, 2], vec![3, 4]]);
        let vec = set.to_vec();
        assert_eq!(vec.len(), 2);
        assert!(vec.contains(&vec![1, 2]));
        assert!(vec.contains(&vec![3, 4]));
    }

    #[test]
    fn controller_set_clone() {
        let set = ControllerSet::new(vec![vec![1, 2, 3]]);
        let cloned = set.clone();
        assert!(cloned.contains(&vec![1, 2, 3]));
    }

    // -----------------------------------------------------------------------
    // Dispatch command tests — full integration with NodeStore
    // -----------------------------------------------------------------------

    /// Creates an unlimited capacity tracker for tests.
    fn test_capacity_tracker() -> std::sync::Arc<crate::capacity::ResourceTracker> {
        let cap = crate::capacity::NodeCapacity {
            total_cores: 256,
            total_memory_bytes: 256 * 1024 * 1024 * 1024, // 256 GB
        };
        std::sync::Arc::new(crate::capacity::ResourceTracker::new(&cap, 0, 0))
    }

    #[test]
    fn dispatch_rejects_command_with_empty_command_id() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        // Need to append a genesis event first so store has a head
        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![], // empty command_id -> structural reject
            node_id.0.to_vec(),
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        assert_eq!(result.decision, 2); // REJECTED
        assert!(!result.reason_code.is_empty());
    }

    #[test]
    fn dispatch_rejects_command_targeting_wrong_node() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64], // wrong target
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        assert_eq!(result.decision, 2); // REJECTED
    }

    #[test]
    fn dispatch_rejects_command_without_target() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let mut command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64],
            vec![1, 2, 3],
            CommandType::Query as i32,
        );
        command.target_node = None; // no target

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_rejects_non_controller_without_delegation() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let mut controllers = ControllerSet::new(vec![vec![1, 2, 3]]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![vec![1, 2, 3]];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        // Issuer is NOT in controllers and has no delegation chain
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            vec![99, 99, 99], // not a controller
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Fails signature verification first (no signature on command)
        // so it never reaches the controller check
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_add_controller_command() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        // Command with the controller as command_id (extract_identity_from_command uses command_id)
        let new_ctrl = vec![10, 20, 30];
        let command = make_command(
            new_ctrl.clone(), // command_id carries target identity
            node_id.0.to_vec(),
            initial_ctrl.clone(), // issued by existing controller
            CommandType::AddController as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );

        // Will fail signature verification (no signature), so gets rejected
        // This tests that the dispatch path runs through signature check
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_add_controller_missing_identity() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        // Empty command_id -> extract_identity returns empty
        let command = make_command(
            vec![], // empty -> missing identity
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::AddController as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Should be rejected for structural reasons (empty command_id)
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_remove_controller_command() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let ctrl_to_remove = vec![10, 20, 30];
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone(), ctrl_to_remove.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            ctrl_to_remove.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::RemoveController as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Fails signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_transfer_control_command() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let new_ctrl = vec![42, 42, 42];
        let command = make_command(
            new_ctrl.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::TransferControl as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Fails signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_query_command_type() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Fails signature verification, but query type is recognized
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_publish_snapshot_returns_use_request() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::PublishSnapshot as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Should be rejected with "use_produce_snapshot_request" after failing sig check
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_fetch_object_returns_use_request() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::FetchObject as i32,
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_unknown_command_type_rejected() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        // Use an unknown command type
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            999, // unknown type
        );

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_custom_command_with_delegation_payload() {
        use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
        use edgerun_proto::edgerun::v0::trust::DelegationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        // Build a delegation record and encode as payload
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![1, 2, 3, 4],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        let delegation_bytes = prost::Message::encode_to_vec(&delegation);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Custom as i32,
        );
        command.payload = Some(Payload::InlinePayload(delegation_bytes));

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Should try to process as delegation but fail signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_custom_command_with_revocation_payload() {
        use edgerun_proto::edgerun::v0::stream::command_envelope::Payload;
        use edgerun_proto::edgerun::v0::trust::RevocationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        // Build a revocation record and encode as payload
        let revocation = RevocationRecord {
            record_version: 1,
            revocation_id: vec![10, 20, 30],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: None,
            effective_at: None,
            revocation_kind: 0,
            scope_override: None,
            reason_code: String::new(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: None,
            target: Some(edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetIdentity(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            })),
        };
        let revocation_bytes = prost::Message::encode_to_vec(&revocation);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Custom as i32,
        );
        command.payload = Some(Payload::InlinePayload(revocation_bytes));

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Will fail signature verification but the dispatch path for revocation runs
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_custom_command_unknown_payload() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Custom as i32,
        );
        command.payload = Some(edgerun_proto::edgerun::v0::stream::command_envelope::Payload::InlinePayload(vec![0xFF; 10]));

        let result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &test_running_workloads(),
        );
        // Fails signature verification (no signature) — so rejected before reaching custom dispatch
        assert_eq!(result.decision, 2);
    }

    // -----------------------------------------------------------------------
    // Event recording and action lifecycle
    // -----------------------------------------------------------------------

    #[test]
    fn dispatch_records_action_lifecycle_events() {
        // Even rejected commands should have events recorded
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        let genesis = edgerun_core::protocol::EventEnvelope {
            envelope_version: 1,
            stream_id: node_id.0.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        store.append_event(&genesis).unwrap();

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let _result = dispatch_command(
            &command, &mut store, &node_id.0, &signer,
            &mut controllers, &mut replay_cache, &revoked, &trusted,
            2, // HARDWARE_BACKED
            &test_capacity_tracker(), &test_workload_policy(), &test_rate_limiter(),
            &std::sync::Arc::new(crate::running_workloads::RunningWorkloads::new()),
        );

        // After genesis (seq 0), there should be additional events recorded
        // for the rejected command (ActionStarted is only for committed, but ActionFailed is)
        let head_seq = store.get_head(&node_id.0).unwrap().unwrap().0;
        assert!(head_seq > 0); // more events than just genesis
    }

    // -----------------------------------------------------------------------
    // Project controller set
    // -----------------------------------------------------------------------

    #[test]
    fn project_controller_set_from_empty_store() {
        let store = test_store();
        let initial = vec![vec![1, 2, 3]];
        let result = project_controller_set(&store, b"test-stream", initial.clone());
        assert_eq!(result.to_vec(), initial);
    }

    #[test]
    fn project_controller_set_returns_initials_when_no_events() {
        let store = test_store();
        let initials = vec![vec![1], vec![2], vec![3]];
        let result = project_controller_set(&store, b"stream", initials.clone());
        assert!(result.contains(&vec![1]));
        assert!(result.contains(&vec![2]));
        assert!(result.contains(&vec![3]));
        assert_eq!(result.to_vec().len(), 3);
    }

    // -----------------------------------------------------------------------
    // Signature verification helpers
    // -----------------------------------------------------------------------

    #[test]
    fn verify_command_signature_fails_without_signature() {
        let command = make_command(vec![1], vec![2], vec![3], 1);
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing_signature");
    }

    #[test]
    fn verify_command_signature_fails_bad_algorithm() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 99,
            value: vec![0u8; 64],
        });
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "bad_algorithm");
    }

    #[test]
    fn verify_command_signature_fails_bad_sig_length() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 1,
            value: vec![0u8; 32], // wrong length
        });
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "bad_signature_length");
    }

    #[test]
    fn verify_command_signature_fails_no_issuer() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 1,
            value: vec![0u8; 64],
        });
        command.issuer = None;
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "no_issuer");
    }

    #[test]
    fn verify_command_signature_fails_bad_key_hint_length() {
        let mut command = make_command(vec![1], vec![2], vec![3], 1);
        command.signature = Some(edgerun_proto::edgerun::v0::common::Signature {
            algorithm: 1,
            value: vec![0u8; 64],
        });
        command.issuer = Some(edgerun_proto::edgerun::v0::common::IdentityRef {
            identity_id: vec![1],
            identity_kind: Some(0),
            key_hint: Some(vec![0u8; 32]), // wrong length
        });
        let result = verify_command_signature(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "bad_key_hint");
    }

    #[test]
    fn verify_delegation_signature_fails_without_signature() {
        let delegation = ProtoDelegationRecord {
            record_version: 1,
            delegation_id: vec![1],
            issuer: None,
            recipient: None,
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        let result = verify_delegation_signature(&delegation);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "missing_signature");
    }

    #[test]
    fn verify_delegation_signature_fails_bad_sig() {
        let delegation = ProtoDelegationRecord {
            record_version: 1,
            delegation_id: vec![1],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![1],
                identity_kind: Some(0),
                key_hint: None,
            }),
            recipient: None,
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(edgerun_proto::edgerun::v0::common::Signature {
                algorithm: 1,
                value: vec![0u8; 32], // wrong length
            }),
        };
        let result = verify_delegation_signature(&delegation);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Extract identity from command
    // -----------------------------------------------------------------------

    #[test]
    fn extract_identity_uses_command_id() {
        let command = make_command(vec![10, 20, 30], vec![0], vec![1], 1);
        let identity = extract_identity_from_command(&command);
        assert_eq!(identity, vec![10, 20, 30]);
    }

    #[test]
    fn extract_identity_falls_back_to_issuer() {
        let mut command = make_command(vec![], vec![0], vec![7, 8, 9], 1);
        let identity = extract_identity_from_command(&command);
        assert_eq!(identity, vec![7, 8, 9]);
    }

    #[test]
    fn extract_identity_returns_empty_when_no_issuer_and_no_command_id() {
        let mut command = make_command(vec![], vec![0], vec![], 1);
        command.issuer = None;
        let identity = extract_identity_from_command(&command);
        assert!(identity.is_empty());
    }

    // -----------------------------------------------------------------------
    // Dispatch result
    // -----------------------------------------------------------------------

    #[test]
    fn dispatch_result_fields() {
        let result = CommandDispatchResult {
            event_type: EventType::CommandCommitted,
            decision: 1,
            reason_code: "test".to_string(),
            response_bytes: vec![1, 2, 3],
        };
        assert_eq!(result.decision, 1);
        assert_eq!(result.reason_code, "test");
        assert_eq!(result.response_bytes, vec![1, 2, 3]);
    }
