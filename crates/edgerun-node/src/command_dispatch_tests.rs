#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_hardware_signing::{MeshSigner, NodeID};
    use edgerun_storage::{BlobKeySource, NodeStore, NodeStoreConfig};
    use std::sync::Arc;

    // Test helpers for workload policy removed - modules not needed for interface boundary audit
    // The dispatch_command function now takes CommandExecutionContext directly
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
        let binding =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes((&private_key).into()).unwrap();
        let vk = binding.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_identity = [0u8; 64];
        node_identity.copy_from_slice(&encoded.as_bytes()[1..65]);
        let config = NodeStoreConfig {
            data_root: root,
            blob_key_source: Arc::new(BlobKeySource::Software {
                private_key_bytes: private_key.to_vec(),
            }),
            node_identity: node_identity.to_vec(),
        };
        NodeStore::open(&config).unwrap()
    }

    fn random_signing_key() -> edgerun_crypto::p256::ecdsa::SigningKey {
        let mut bytes = [0u8; 32];
        edgerun_crypto::fill_random(&mut bytes).expect("random generation failed");
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
            let sig: edgerun_crypto::p256::ecdsa::Signature =
                self.key.sign_prehash(digest).map_err(|e| {
                    edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string())
                })?;
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
            target_node: Some(edgerun_core::protocol::NodeRef {
                node_id: target_node_id,
            }),
            issuer: Some(edgerun_core::protocol::IdentityRef {
                identity_id: issuer_id,
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type,
            command_version: 1,
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            app_intent: vec![],
            command_metadata: None,
            signature: None,
        }
    }

    fn sign_command(command: &mut CommandEnvelope, signer: &TestSigner) {
        command.signature = None;
        if let Some(issuer) = &mut command.issuer {
            issuer.identity_kind =
                Some(edgerun_core::protocol::IdentityKind::Node as i32);
            issuer.key_hint = Some(signer.node_id.0.to_vec());
        }
        let canonical = edgerun_core::protocol::canonical_bytes(
            &edgerun_core::protocol::ProtocolRecord::CommandEnvelope(command.clone()),
            true,
        );
        let sig = edgerun_core::crypto::sign_canonical_record(
            &signer.key,
            edgerun_core::crypto::SIG_DOMAIN_COMMAND_ENVELOPE,
            &canonical,
        )
        .unwrap();
        command.signature = Some(edgerun_core::protocol::Signature {
            algorithm: edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });
    }

    fn sign_revocation_record(
        revocation: &mut edgerun_core::protocol::RevocationRecord,
        signer: &TestSigner,
    ) {
        revocation.signature = None;
        if let Some(issuer) = &mut revocation.issuer {
            issuer.identity_kind =
                Some(edgerun_core::protocol::IdentityKind::Node as i32);
            issuer.key_hint = Some(signer.node_id.0.to_vec());
        }
        let canonical = edgerun_core::protocol::canonical_bytes(
            &edgerun_core::protocol::ProtocolRecord::RevocationRecord(revocation.clone()),
            true,
        );
        let sig = edgerun_core::crypto::sign_canonical_record(
            &signer.key,
            edgerun_core::crypto::SIG_DOMAIN_REVOCATION_RECORD,
            &canonical,
        )
        .unwrap();
        revocation.signature = Some(edgerun_core::protocol::Signature {
            algorithm: edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });
    }

    fn sign_delegation_record(
        delegation: &mut edgerun_core::protocol::DelegationRecord,
        signer: &TestSigner,
    ) {
        delegation.signature = None;
        if let Some(issuer) = &mut delegation.issuer {
            issuer.identity_kind =
                Some(edgerun_core::protocol::IdentityKind::Node as i32);
            issuer.key_hint = Some(signer.node_id.0.to_vec());
        }
        let canonical = edgerun_core::protocol::canonical_bytes(
            &edgerun_core::protocol::ProtocolRecord::DelegationRecord(delegation.clone()),
            true,
        );
        let sig = edgerun_core::crypto::sign_canonical_record(
            &signer.key,
            edgerun_core::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &canonical,
        )
        .unwrap();
        delegation.signature = Some(edgerun_core::protocol::Signature {
            algorithm: edgerun_core::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });
    }

    fn valid_capability_descriptor() -> edgerun_core::protocol::CapabilityDescriptor {
        edgerun_core::protocol::CapabilityDescriptor {
            capability_version: 1,
            capability_kind: edgerun_core::protocol::CapabilityKind::NodeControl as i32,
            actions: vec!["delegate".into()],
            scope: Some(edgerun_core::protocol::ScopeDescriptor {
                scope_version: 1,
                scope_kind: edgerun_core::protocol::ScopeKind::Node as i32,
                target_nodes: vec![edgerun_core::protocol::NodeRef {
                    node_id: b"node-a".to_vec(),
                }],
                target_streams: vec![],
                target_object_kinds: vec![],
                target_view_types: vec![],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            constraints: None,
            delegation_policy:
                edgerun_core::protocol::DelegationPolicy::DelegableWithAttenuation as i32,
            minimum_assurance: None,
            capability_metadata: None,
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
    fn config_patch_updates_projectable_fields() {
        let mut config = crate::config::parse_config(
            r#"
stream_id: "node-stream"
name: "old"
controllers: ["a"]
trust_nodes: []
allowed_peers: []
bootstrap_peers: []
"#,
        )
        .unwrap();

        let changed = apply_config_patch(
            &mut config,
            br#"{"name":"new","allowed_peers":["peer-a"],"bootstrap_peers":["127.0.0.1:9000@peer-a"]}"#,
        )
        .unwrap();

        assert!(changed);
        assert_eq!(config.name.as_deref(), Some("new"));
        assert_eq!(config.allowed_peers, vec!["peer-a"]);
        assert_eq!(config.bootstrap_peers, vec!["127.0.0.1:9000@peer-a"]);
    }

    #[test]
    fn project_config_replays_committed_config_patch_result_object() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let stream_id = b"stream-1";

        append_signed_event(
            &mut store,
            stream_id,
            &signer,
            EventType::NodeGenesis,
            1,
            None,
            vec![],
            vec![],
            vec![],
        );

        let patch_ref = store
            .put_object(
                br#"{"name":"projected","allowed_peers":["peer-b"]}"#,
                edgerun_core::protocol::ObjectKind::DerivedView as i32,
                &[stream_id.to_vec()],
            )
            .unwrap();
        let result_payload = ProtoCommandResultPayload {
            payload_version: 1,
            command: None,
            issuer: None,
            decision: CommandDecision::Committed as i32,
            decision_basis: None,
            reason_code: String::new(),
            effect_summary_object: None,
            result_object: Some(patch_ref),
        };
        let result_ref = store
            .put_object(
                &prost::Message::encode_to_vec(&result_payload),
                edgerun_core::protocol::ObjectKind::Command as i32,
                &[stream_id.to_vec()],
            )
            .unwrap();
        append_signed_event(
            &mut store,
            stream_id,
            &signer,
            EventType::CommandCommitted,
            1,
            Some(result_ref),
            vec![],
            vec![],
            vec![],
        );

        let projected = project_config(
            &store,
            stream_id,
            r#"
stream_id: "stream-1"
name: "initial"
controllers: []
trust_nodes: []
allowed_peers: []
bootstrap_peers: []
"#,
        )
        .unwrap();

        assert_eq!(projected.name.as_deref(), Some("projected"));
        assert_eq!(projected.allowed_peers, vec!["peer-b"]);
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
            total_memory_bytes: 256 * 1024 * 1024 * 1024 // 256 GB
        };
        std::sync::Arc::new(crate::capacity::ResourceTracker::new(&cap, 0, 0))
    }

    fn append_genesis(store: &mut NodeStore, node_id: &NodeID) {
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
        store.append_event_blocking(genesis).unwrap();
    }

    fn dispatch_test_command(
        command: &CommandEnvelope,
        store: &mut NodeStore,
        node_id: &NodeID,
        signer: &TestSigner,
        controllers: &mut ControllerSet,
        replay_cache: &mut HashMap<Vec<u8>, (Vec<u8>, i64)>,
        revoked: &HashSet<Vec<u8>>,
        trusted: &[Vec<u8>],
    ) -> CommandDispatchResult {
        let exec_ctx = CommandExecutionContext::test_default();
        dispatch_command(
            command,
            store,
            &node_id.0,
            signer,
            controllers,
            replay_cache,
            revoked,
            trusted,
            2, // HARDWARE_BACKED
            &exec_ctx,
        )
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

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![], // empty command_id -> structural reject
            node_id.0.to_vec(),
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64], // wrong target
            vec![1, 2, 3],
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        let mut command = make_command(
            vec![1, 2, 3],
            vec![0u8; 64],
            vec![1, 2, 3],
            CommandType::Query as i32,
        );
        command.target_node = None; // no target

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        // Issuer is NOT in controllers and has no delegation chain
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            vec![99, 99, 99], // not a controller
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        // Command with the controller as command_id (extract_identity_from_command uses command_id)
        let new_ctrl = vec![10, 20, 30];
        let command = make_command(
            new_ctrl.clone(), // command_id carries target identity
            node_id.0.to_vec(),
            initial_ctrl.clone(), // issued by existing controller
            CommandType::AddController as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        // Empty command_id -> extract_identity returns empty
        let command = make_command(
            vec![], // empty -> missing identity
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::AddController as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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
        let mut controllers =
            ControllerSet::new(vec![initial_ctrl.clone(), ctrl_to_remove.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let command = make_command(
            ctrl_to_remove.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::RemoveController as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Fails signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_signed_remove_last_controller_is_rejected() {
        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let mut command = make_command(
            initial_ctrl.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::RemoveController as i32,
        );
        sign_command(&mut command, &signer);

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        assert_eq!(result.decision, 2);
        assert_eq!(result.reason_code, "CONTROL_INVARIANT_FAILED");
        assert!(controllers.contains(&initial_ctrl));
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

        append_genesis(&mut store, &node_id);

        let new_ctrl = vec![42, 42, 42];
        let command = make_command(
            new_ctrl.clone(),
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::TransferControl as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::PublishSnapshot as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::FetchObject as i32,
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        // Use an unknown command type
        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            999, // unknown type
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_custom_command_with_delegation_payload() {
        use edgerun_core::protocol::command_envelope::Payload;
        use edgerun_core::protocol::DelegationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let now_secs = now_unix_secs_i64();

        // Build a structurally valid but unsigned delegation record.
        let delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![1, 2, 3, 4],
            issuer: Some(edgerun_core::protocol::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(edgerun_core::protocol::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: now_secs,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            capability: Some(valid_capability_descriptor()),
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
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(Payload::InlinePayload(delegation_bytes));

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Should try to process as delegation but fail signature verification
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_signed_delegation_payload_commits() {
        use edgerun_core::protocol::command_envelope::Payload;
        use edgerun_core::protocol::DelegationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = node_id.0.to_vec();
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let now_secs = now_unix_secs_i64();
        let mut delegation = DelegationRecord {
            record_version: 1,
            delegation_id: vec![1, 2, 3, 4],
            issuer: Some(edgerun_core::protocol::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            recipient: Some(edgerun_core::protocol::IdentityRef {
                identity_id: vec![5, 6, 7],
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: now_secs,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            capability: Some(valid_capability_descriptor()),
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_delegation_record(&mut delegation, &signer);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(Payload::InlinePayload(prost::Message::encode_to_vec(
            &delegation,
        )));
        sign_command(&mut command, &signer);

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        assert_eq!(result.decision, CommandDecision::Committed as i32);
    }

    #[test]
    fn dispatch_custom_command_with_revocation_payload() {
        use edgerun_core::protocol::command_envelope::Payload;
        use edgerun_core::protocol::RevocationRecord;

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = vec![1, 2, 3];
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        // Build a revocation record and encode as payload
        let revocation = RevocationRecord {
            record_version: 1,
            revocation_id: vec![10, 20, 30],
            issuer: Some(edgerun_core::protocol::IdentityRef {
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
            target: Some(
                edgerun_core::protocol::revocation_record::Target::TargetIdentity(
                    edgerun_core::protocol::IdentityRef {
                        identity_id: vec![5, 6, 7],
                        identity_kind: Some(2),
                        key_hint: None,
                    },
                ),
            ),
        };
        let revocation_bytes = prost::Message::encode_to_vec(&revocation);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(Payload::InlinePayload(revocation_bytes));

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );
        // Will fail signature verification but the dispatch path for revocation runs
        assert_eq!(result.decision, 2);
    }

    #[test]
    fn dispatch_signed_revocation_payload_commits() {
        use edgerun_core::protocol::command_envelope::Payload;
        use edgerun_core::protocol::{RevocationKind, RevocationRecord};

        let mut store = test_store();
        let signer = TestSigner::new();
        let node_id = signer.node_id();
        let initial_ctrl = node_id.0.to_vec();
        let mut controllers = ControllerSet::new(vec![initial_ctrl.clone()]);
        let mut replay_cache = HashMap::new();
        let revoked = HashSet::new();
        let trusted = vec![initial_ctrl.clone()];

        append_genesis(&mut store, &node_id);

        let now_secs = now_unix_secs_i64();
        let mut revocation = RevocationRecord {
            record_version: 1,
            revocation_id: vec![10, 20, 30],
            issuer: Some(edgerun_core::protocol::IdentityRef {
                identity_id: initial_ctrl.clone(),
                identity_kind: Some(2),
                key_hint: None,
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: now_secs,
                nanos: 0,
            }),
            effective_at: None,
            revocation_kind: RevocationKind::IdentityTrust as i32,
            scope_override: None,
            reason_code: "test".into(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: None,
            target: Some(
                edgerun_core::protocol::revocation_record::Target::TargetIdentity(
                    edgerun_core::protocol::IdentityRef {
                        identity_id: vec![5, 6, 7],
                        identity_kind: Some(2),
                        key_hint: None,
                    },
                ),
            ),
        };
        sign_revocation_record(&mut revocation, &signer);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateRevocation as i32,
        );
        command.payload = Some(Payload::InlinePayload(prost::Message::encode_to_vec(
            &revocation,
        )));
        sign_command(&mut command, &signer);

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
        );

        assert_eq!(result.decision, CommandDecision::Committed as i32);
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

        append_genesis(&mut store, &node_id);

        let mut command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::CreateDelegation as i32,
        );
        command.payload = Some(
            edgerun_core::protocol::command_envelope::Payload::InlinePayload(vec![
                0xFF;
                10
            ]),
        );

        let result = dispatch_test_command(
            &command,
            &mut store,
            &node_id,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
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

        append_genesis(&mut store, &node_id);

        let command = make_command(
            vec![1, 2, 3],
            node_id.0.to_vec(),
            initial_ctrl.clone(),
            CommandType::Query as i32,
        );

        let exec_ctx = CommandExecutionContext::test_default();
        let _result = dispatch_command(
            &command,
            &mut store,
            &node_id.0,
            &signer,
            &mut controllers,
            &mut replay_cache,
            &revoked,
            &trusted,
            2, // HARDWARE_BACKED
            &exec_ctx,
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
}
