use std::path::PathBuf;
use std::sync::Arc;

use edgerun_hardware_signing::NodeID;
use edgerun_node::command_dispatch::{
    ControllerSet, create_node_genesis_payload, dispatch_command_with_protocol_signer,
};
use edgerun_node::runtime::RuntimeKernel;
use edgerun_node::storage::MemoryRuntimeStorage;
use edgerun_node::stream_append::append_signed_stream_event_blocking_with_protocol_signer;
use edgerun_protocols::core_protocol::collections::{HashMap, HashSet};
use edgerun_protocols::core_protocol::command::{CommandExecutionContext, command_hash};
use edgerun_protocols::core_protocol::protocol::{
    CommandDecision, CommandEnvelope, CommandType, EventType, IdentityRef, NodeRef, ObjectKind,
    ProtocolRecord, Signature, Timestamp,
};
use edgerun_protocols::core_protocol::util::{
    bytes_to_hex, now_protocol_timestamp, now_unix_secs_i64,
};
use edgerun_protocols::keygen::generate_ephemeral_node_identity;
use edgerun_protocols::sign::{ProtocolSigner, SignableProtocolFamily};
use edgerun_protocols::wire::{
    ROUTE_SCHEME_HTTPS, RUNTIME_EVENT_APP_INSTALLED, SdkWireRecord, from_bytes, sdk_wire_bytes,
};
use edgerun_sdk::browser_authoring::{
    BrowserAppArtifact, BrowserAppRoute, BrowserAppSpec, build_publishable_app,
};
use edgerun_storage::{BlobKeySource, NodeStore, NodeStoreConfig};

fn tmp_data_root(test_name: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "edgerun_node_authority_flow_{}_{}_{}",
        test_name,
        std::process::id(),
        n
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create temp store");
    path
}

fn open_store(data_root: PathBuf, node_id: &[u8]) -> NodeStore {
    let config = NodeStoreConfig {
        data_root,
        blob_key_source: Arc::new(BlobKeySource::Software {
            private_key_bytes: vec![0x42; 32],
        }),
        node_identity: node_id.to_vec(),
    };
    NodeStore::open(&config).expect("open node store")
}

fn empty_exec_ctx() -> CommandExecutionContext {
    CommandExecutionContext {
        has_local_session: false,
        has_user_presence: false,
        accepted_assurance_claims: Vec::new(),
        transport_class: None,
        location_classes: Vec::new(),
        target_stream_id: None,
        target_view_type: None,
        target_domain: None,
        execution_class: None,
        storage_class: None,
    }
}

fn signed_query_command(target_node_id: &[u8; 64]) -> CommandEnvelope {
    let issuer = generate_ephemeral_node_identity();
    let now_secs = now_unix_secs_i64();
    let mut command = CommandEnvelope {
        envelope_version: 1,
        command_id: vec![0x10, 0x20, 0x30, 0x40],
        target_node: Some(NodeRef {
            node_id: target_node_id.to_vec(),
        }),
        issuer: Some(IdentityRef {
            identity_id: issuer.node_id.to_vec(),
            identity_kind: Some(edgerun_protocols::core_protocol::crypto::IDENTITY_KIND_NODE),
            key_hint: Some(issuer.node_id.to_vec()),
        }),
        command_type: CommandType::Query as i32,
        command_version: 1,
        issued_at: Some(Timestamp {
            seconds: now_secs.saturating_sub(1),
            nanos: 0,
        }),
        not_before: None,
        expires_at: Some(Timestamp {
            seconds: now_secs.saturating_add(60),
            nanos: 0,
        }),
        idempotency_key: Vec::new(),
        payload: None,
        delegation_chain: Vec::new(),
        requested_assurance: None,
        command_metadata: None,
        signatures: Vec::new(),
        app_intent: Vec::new(),
    };
    let signed = issuer
        .signer
        .sign_protocol_record(
            &ProtocolRecord::CommandEnvelope(command.clone()),
            SignableProtocolFamily::CommandEnvelope,
        )
        .expect("sign command");
    command.signatures.push(Signature {
        algorithm: signed.signature.algorithm,
        value: signed.signature.value,
    });
    command
}

fn append_genesis(store: &mut NodeStore, node: &edgerun_protocols::keygen::EphemeralNodeIdentity) {
    let payload_ref = create_node_genesis_payload(store, &node.node_id, &NodeID(node.node_id), &[]);
    let stored_payload = store
        .get_object(&payload_ref)
        .expect("read genesis payload object")
        .expect("genesis payload object present");
    assert!(!stored_payload.content.is_empty());

    let genesis = append_signed_stream_event_blocking_with_protocol_signer(
        store,
        &node.node_id,
        &node.signer,
        edgerun_storage::EventDraft {
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: Some(now_protocol_timestamp()),
            payload_object: Some(payload_ref.clone()),
            ..Default::default()
        },
    )
    .expect("append signed genesis");

    assert_eq!(genesis.seq, 0);
    assert_eq!(genesis.event_type, EventType::NodeGenesis as i32);
    assert_eq!(genesis.payload_object, Some(payload_ref));
    assert!(genesis.signature.is_some());
    edgerun_storage::verify_event(&genesis, &node.node_id).expect("verify genesis signature");
}

#[test]
fn command_commit_flows_across_protocol_stream_storage_and_replay_projection() {
    let data_root = tmp_data_root("commit");
    let node = generate_ephemeral_node_identity();
    let mut store = open_store(data_root.clone(), &node.node_id);
    append_genesis(&mut store, &node);

    assert_eq!(
        store
            .validate_stream_chain_with_writer(&node.node_id, &node.node_id)
            .expect("validate genesis chain"),
        1
    );

    let command = signed_query_command(&node.node_id);
    let command_hash_hex = bytes_to_hex(&command_hash(&command).value);
    let target_hex = bytes_to_hex(&node.node_id);
    let mut controllers = ControllerSet::default();
    let mut replay_cache = HashMap::new();
    let revoked_delegations = HashSet::new();
    let exec_ctx = empty_exec_ctx();

    let result = dispatch_command_with_protocol_signer(
        &command,
        &mut store,
        &node.node_id,
        &node.node_id,
        &node.signer,
        &mut controllers,
        &mut replay_cache,
        &revoked_delegations,
        &[],
        0,
        &exec_ctx,
    );

    assert_eq!(result.event_type, EventType::CommandCommitted);
    assert_eq!(result.decision, CommandDecision::Committed as i32);
    assert_eq!(result.reason_code, "");

    let committed = store
        .get_event(&node.node_id, 1)
        .expect("read committed event")
        .expect("committed event present");
    assert_eq!(committed.event_type, EventType::CommandCommitted as i32);
    assert_eq!(committed.seq, 1);
    assert!(committed.prev_event_hash.is_some());
    assert_eq!(committed.related_commands.len(), 1);
    assert_eq!(committed.related_commands[0].command_id, command.command_id);
    assert_eq!(
        committed.related_commands[0]
            .command_hash
            .as_ref()
            .expect("related command hash")
            .value,
        command_hash(&command).value
    );
    edgerun_storage::verify_event(&committed, &node.node_id).expect("verify commit signature");

    assert_eq!(
        store
            .get_replay_entry(&target_hex, &command_hash_hex)
            .expect("read replay entry")
            .expect("replay entry present")
            .1,
        1
    );
    assert_eq!(
        store
            .validate_stream_chain_with_writer(&node.node_id, &node.node_id)
            .expect("validate committed chain"),
        2
    );

    let _ = std::fs::remove_dir_all(data_root);
}

#[test]
fn rejected_and_duplicate_commands_are_recorded_as_decision_events() {
    let data_root = tmp_data_root("reject_duplicate");
    let node = generate_ephemeral_node_identity();
    let mut store = open_store(data_root.clone(), &node.node_id);
    append_genesis(&mut store, &node);

    let mut rejected_command = signed_query_command(&node.node_id);
    rejected_command.signatures[0].value[0] ^= 0xff;

    let mut controllers = ControllerSet::default();
    let mut replay_cache = HashMap::new();
    let revoked_delegations = HashSet::new();
    let exec_ctx = empty_exec_ctx();

    let rejected = dispatch_command_with_protocol_signer(
        &rejected_command,
        &mut store,
        &node.node_id,
        &node.node_id,
        &node.signer,
        &mut controllers,
        &mut replay_cache,
        &revoked_delegations,
        &[],
        0,
        &exec_ctx,
    );

    assert_eq!(rejected.event_type, EventType::CommandRejected);
    assert_eq!(rejected.decision, CommandDecision::Rejected as i32);
    assert_eq!(rejected.reason_code, "rejected");

    let rejected_event = store
        .get_event(&node.node_id, 1)
        .expect("read rejected event")
        .expect("rejected event present");
    assert_eq!(rejected_event.event_type, EventType::CommandRejected as i32);
    edgerun_storage::verify_event(&rejected_event, &node.node_id)
        .expect("verify rejected event signature");

    let duplicate = dispatch_command_with_protocol_signer(
        &rejected_command,
        &mut store,
        &node.node_id,
        &node.node_id,
        &node.signer,
        &mut controllers,
        &mut replay_cache,
        &revoked_delegations,
        &[],
        0,
        &exec_ctx,
    );

    assert_eq!(duplicate.event_type, EventType::CommandCommitted);
    assert_eq!(duplicate.decision, CommandDecision::Committed as i32);
    assert_eq!(duplicate.reason_code, "duplicate_command");

    let duplicate_event = store
        .get_event(&node.node_id, 2)
        .expect("read duplicate event")
        .expect("duplicate event present");
    assert_eq!(
        duplicate_event.event_type,
        EventType::CommandCommitted as i32
    );
    assert_eq!(duplicate_event.seq, 2);
    assert_eq!(
        store
            .validate_stream_chain_with_writer(&node.node_id, &node.node_id)
            .expect("validate rejected duplicate chain"),
        3
    );

    let _ = std::fs::remove_dir_all(data_root);
}

#[test]
fn sdk_authored_app_installs_in_runtime_and_is_archived_to_node_stream() {
    let data_root = tmp_data_root("sdk_app");
    let node = generate_ephemeral_node_identity();
    let mut store = open_store(data_root.clone(), &node.node_id);
    append_genesis(&mut store, &node);

    let package = build_publishable_app(BrowserAppSpec {
        slug: "dashboard",
        name: "Dashboard",
        version: "1.0.0",
        summary: "Runtime dashboard app",
        developer_seed: [7; 32],
        code_sha256: None,
        routes: vec![BrowserAppRoute {
            scheme: ROUTE_SCHEME_HTTPS,
            host: b"dash.edgerun.local",
            path_prefix: b"/app/",
        }],
        storage_namespaces: vec![b"dashboard/state".as_slice()],
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
        artifacts: vec![BrowserAppArtifact {
            path: "static/app.js",
            bytes: b"export default function dashboard() {}",
        }],
    })
    .expect("sdk builds publishable app package");

    let graph =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&package.app_graph_bytes)
            .expect("decode sdk app graph");
    assert!(matches!(graph, SdkWireRecord::AppGraph(_)));

    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), package.app_id);
    runtime
        .install_app_graph_wire(&package.app_graph_bytes, 1)
        .expect("runtime installs sdk app graph");
    assert_eq!(runtime.events().len(), 1);
    assert_eq!(
        runtime.events()[0].event.event_kind,
        RUNTIME_EVENT_APP_INSTALLED
    );

    let runtime_event = runtime.events()[0].event.clone();
    let runtime_event_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(runtime_event.clone()));
    let archived_event =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&runtime_event_bytes)
            .expect("decode runtime event");
    let archived_event = match archived_event {
        SdkWireRecord::RuntimeEvent(event) => event,
        _ => panic!("expected runtime event"),
    };
    assert_eq!(archived_event, runtime_event);
    assert_eq!(
        archived_event.payload_sha256,
        edgerun_sdk::sha256(&archived_event.payload)
    );

    let installed_payload =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&archived_event.payload)
            .expect("decode runtime install payload");
    let installed = match installed_payload {
        SdkWireRecord::RuntimeAppInstall(install) => install,
        _ => panic!("expected runtime app install payload"),
    };
    assert_eq!(installed.app_id, package.app_id);
    assert_eq!(installed.release_id, package.release_id);
    assert_eq!(installed.declared_routes.len(), 1);

    let runtime_event_ref = store
        .put_object(
            &runtime_event_bytes,
            ObjectKind::Payload as i32,
            &[node.node_id.to_vec()],
        )
        .expect("store runtime event payload object");
    let stream_event = append_signed_stream_event_blocking_with_protocol_signer(
        &store,
        &node.node_id,
        &node.signer,
        edgerun_storage::EventDraft {
            event_type: EventType::ActionCompleted as i32,
            event_version: 1,
            recorded_at: Some(now_protocol_timestamp()),
            payload_object: Some(runtime_event_ref.clone()),
            related_objects: vec![runtime_event_ref.clone()],
            ..Default::default()
        },
    )
    .expect("append sdk app install runtime event to node stream");

    assert_eq!(stream_event.seq, 1);
    assert_eq!(stream_event.event_type, EventType::ActionCompleted as i32);
    assert_eq!(stream_event.payload_object, Some(runtime_event_ref.clone()));
    edgerun_storage::verify_event(&stream_event, &node.node_id)
        .expect("verify sdk app stream event signature");

    let stored_runtime_event = store
        .get_object(&runtime_event_ref)
        .expect("read runtime event object")
        .expect("runtime event object present");
    let decoded_stored_event = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(
        &stored_runtime_event.content,
    )
    .expect("decode stored runtime event object");
    assert_eq!(
        decoded_stored_event,
        SdkWireRecord::RuntimeEvent(runtime_event)
    );
    assert_eq!(
        store
            .validate_stream_chain_with_writer(&node.node_id, &node.node_id)
            .expect("validate sdk app install stream"),
        2
    );

    let _ = std::fs::remove_dir_all(data_root);
}
