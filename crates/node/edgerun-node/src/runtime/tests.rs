use super::*;
use crate::storage::MemoryRuntimeStorage;
use crate::storage::{
    BrowserRuntimeStorage, BrowserRuntimeStorageHost, RuntimeStorageDecision, RuntimeStorageSurface,
};
use alloc::vec;
use edgerun_protocols::wire::{
    app_run_prompt_decision_id, from_bytes, package_cache_id, AppRunPromptDecisionRecord,
    CapabilityRequest, PackageCacheRecord, RuntimeDomainConfig, RuntimeMailbox,
    APP_RUN_DECISION_CANCEL, APP_RUN_DECISION_VERIFY_AND_CACHE, CAPABILITY_KIND_NETWORK,
    CAPABILITY_KIND_SIGNING, CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_READ,
    CAPABILITY_OPERATION_RECEIVE, CAPABILITY_OPERATION_SIGN, CAPABILITY_OPERATION_WRITE,
    CAPABILITY_STATUS_INVALID_REQUEST, HTTP_METHOD_GET, PACKAGE_CACHE_STATE_VERIFIED,
    ROUTE_SCHEME_HTTPS, RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED, RUNTIME_EVENT_CAPABILITY_DENIED,
    RUNTIME_EVENT_CAPABILITY_EXECUTED, RUNTIME_EVENT_CAPABILITY_GRANTED,
    RUNTIME_EVENT_CAPABILITY_SESSION_OPENED, RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
    RUNTIME_EVENT_PACKAGE_VERIFIED, RUNTIME_EVENT_STORAGE_BOUND, RUNTIME_NETWORK_BINDING_FETCH,
    RUNTIME_PROTOCOL_DNS_UDP, RUNTIME_PROTOCOL_HTTPS, RUNTIME_PROTOCOL_SMTP,
    RUNTIME_STORAGE_BACKING_BROWSER, RUNTIME_STORAGE_BACKING_MEMORY,
};
use edgerun_work::capability_packet::{
    capability_envelope, capability_invocation_id, CAPABILITY_CONTENT_OBJECT,
    CAPABILITY_OPERATION_OBJECT_PUT, CAPABILITY_PACKET_INVOKE,
};
use edgerun_work::codec::blake3_hash;

#[derive(Default)]
struct TestAppSigner {
    keys: BTreeMap<[u8; 32], Vec<u8>>,
}

impl TestAppSigner {
    fn insert(&mut self, app_id: [u8; 32], public_key: Vec<u8>) {
        self.keys.insert(app_id, public_key);
    }
}

impl RuntimeAppSigner for TestAppSigner {
    fn app_public_key(&mut self, app_id: &[u8; 32]) -> Result<Option<Vec<u8>>, RuntimeError> {
        Ok(self.keys.get(app_id).cloned())
    }

    fn sign_app_payload(
        &mut self,
        app_id: &[u8; 32],
        payload: &[u8],
    ) -> Result<Vec<u8>, RuntimeError> {
        if !self.keys.contains_key(app_id) {
            return Err(RuntimeError::SigningDenied);
        }
        let mut signature = Vec::new();
        signature.extend_from_slice(&sha256(payload));
        signature.extend_from_slice(&sha256(app_id));
        Ok(signature)
    }
}

#[derive(Default)]
struct TestBrowserStorage {
    objects: BTreeMap<(Vec<u8>, [u8; 32]), Vec<u8>>,
}

impl BrowserRuntimeStorageHost for TestBrowserStorage {
    fn read_object(
        &mut self,
        namespace: &[u8],
        key: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        Ok(self.objects.get(&(namespace.to_vec(), *key)).cloned())
    }

    fn write_object(
        &mut self,
        namespace: &[u8],
        key: &[u8; 32],
        value: &[u8],
    ) -> Result<(), RuntimeError> {
        self.objects
            .insert((namespace.to_vec(), *key), value.to_vec());
        Ok(())
    }
}

fn grant_network_route<S>(
    runtime: &mut RuntimeKernel<S>,
    app_id: [u8; 32],
    release_id: [u8; 32],
    host: &[u8],
    time: u64,
) where
    S: crate::storage::RuntimeStorage,
{
    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_NETWORK,
        CAPABILITY_OPERATION_RECEIVE,
        2,
        sha256(host),
        sha256(b"https-route"),
        time,
        100,
        b"user-signature-network".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), time)
        .expect("network grant");
    runtime
        .bind_network_provider(
            runtime_network_binding(
                &grant,
                sha256(b"https-provider"),
                sha256(b"https-listener"),
                RUNTIME_NETWORK_BINDING_FETCH,
                RUNTIME_PROTOCOL_HTTPS,
                443,
                host.to_vec(),
                sha256(b"GET"),
            ),
            time,
        )
        .expect("network binding");
}

fn grant_storage_provider<S>(
    runtime: &mut RuntimeKernel<S>,
    app_id: [u8; 32],
    release_id: [u8; 32],
    namespace: &[u8],
    operation: u16,
    time: u64,
    backing_kind: u16,
) -> RuntimeCapabilityGrant
where
    S: crate::storage::RuntimeStorage,
{
    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        operation,
        2,
        sha256(namespace),
        sha256(b"storage-constraints"),
        time,
        time + 100,
        b"user-signature-storage".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), time)
        .expect("storage grant");
    runtime
        .bind_storage_provider(
            runtime_storage_binding(
                &grant,
                namespace.to_vec(),
                sha256(b"storage-provider"),
                sha256(b"object-store"),
                backing_kind,
            ),
            time,
        )
        .expect("storage binding");
    grant
}

fn first_run_decision(
    profile_id: [u8; 32],
    app: &RuntimeAppInstall,
    package_sha256: [u8; 32],
    decision: u16,
    decided_at: u64,
) -> AppRunPromptDecisionRecord {
    AppRunPromptDecisionRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        decision_id: app_run_prompt_decision_id(
            profile_id,
            app.app_id,
            app.release_id,
            package_sha256,
            app.manifest_sha256,
            decision,
            decided_at,
        ),
        profile_id,
        app_id: app.app_id,
        release_id: app.release_id,
        package_sha256,
        manifest_sha256: app.manifest_sha256,
        retrieval_cost: 128,
        decision,
        decided_at,
        user_signature: b"user-choice".to_vec(),
    }
}

fn verified_package_cache(
    profile_id: [u8; 32],
    app: &RuntimeAppInstall,
    package_sha256: [u8; 32],
    verified_at: u64,
) -> PackageCacheRecord {
    PackageCacheRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        cache_id: package_cache_id(
            profile_id,
            app.app_id,
            app.release_id,
            package_sha256,
            app.manifest_sha256,
        ),
        profile_id,
        app_id: app.app_id,
        release_id: app.release_id,
        package_sha256,
        manifest_sha256: app.manifest_sha256,
        code_sha256: app.code_sha256,
        cached_bytes: 4096,
        state: PACKAGE_CACHE_STATE_VERIFIED,
        verified_at,
        source_admission_hash: sha256(b"admission"),
    }
}

#[test]
fn runtime_records_routes_dispatches_and_brokers_storage() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let route = runtime_http_route(
        app_id,
        release_id,
        ROUTE_SCHEME_HTTPS,
        b"example.com".to_vec(),
        b"/app/".to_vec(),
    );
    let runtime_projection = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        vec![route.clone()],
        vec![b"private".to_vec()],
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(runtime_projection, 1)
        .expect("record app");
    grant_network_route(&mut runtime, app_id, release_id, b"example.com", 2);
    runtime.grant_http_route(route, 2).expect("route");
    let dispatch = runtime
        .dispatch_http(
            runtime_http_request(
                HTTP_METHOD_GET,
                ROUTE_SCHEME_HTTPS,
                b"example.com".to_vec(),
                b"/app/index".to_vec(),
                [0; 32],
                Vec::new(),
            ),
            3,
        )
        .expect("dispatch");
    assert_eq!(dispatch.app_id, app_id);

    let write_grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"private"),
        sha256(b"memory-storage"),
        4,
        100,
        b"user-signature-write".to_vec(),
    );
    runtime
        .grant_runtime_capability(write_grant.clone(), 4)
        .expect("write grant");
    runtime
        .bind_storage_provider(
            runtime_storage_binding(
                &write_grant,
                b"private".to_vec(),
                sha256(b"memory-provider"),
                sha256(b"private-object-store"),
                RUNTIME_STORAGE_BACKING_MEMORY,
            ),
            4,
        )
        .expect("write storage binding");

    let read_grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_READ,
        2,
        sha256(b"private"),
        sha256(b"memory-storage"),
        5,
        100,
        b"user-signature-read".to_vec(),
    );
    runtime
        .grant_runtime_capability(read_grant.clone(), 5)
        .expect("read grant");
    runtime
        .bind_storage_provider(
            runtime_storage_binding(
                &read_grant,
                b"private".to_vec(),
                sha256(b"memory-provider"),
                sha256(b"private-object-store"),
                RUNTIME_STORAGE_BACKING_MEMORY,
            ),
            5,
        )
        .expect("read storage binding");

    let value = b"app private value".to_vec();
    let write_request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        app_id,
        release_id,
        sha256(b"state-key"),
        sha256(&value),
        b"private".to_vec(),
        value.clone(),
        b"nonce".to_vec(),
    );
    let write_request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(write_request));
    let write_response_bytes = runtime
        .invoke_storage_wire(&write_request_bytes, b"runtime-storage", 4)
        .expect("write");
    let write_response =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&write_response_bytes)
            .expect("wire response");
    let write_response = match write_response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(write_response.status, CAPABILITY_STATUS_OK);

    let read_request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_READ,
        2,
        app_id,
        release_id,
        sha256(b"state-key"),
        sha256(&value),
        b"private".to_vec(),
        Vec::new(),
        b"nonce2".to_vec(),
    );
    let read_request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(read_request));
    let read_response_bytes = runtime
        .invoke_storage_wire(&read_request_bytes, b"runtime-storage", 5)
        .expect("read");
    let read_response =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&read_response_bytes)
            .expect("wire response");
    let read_response = match read_response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(read_response.status, CAPABILITY_STATUS_OK);
    assert_eq!(read_response.payload, value);
    let peer_app_id = sha256(b"peer-app");
    let peer_release_id = sha256(b"peer-release");
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                peer_app_id,
                peer_release_id,
                sha256(b"peer-code"),
                sha256(b"peer-developer"),
                sha256(b"peer-manifest"),
                Vec::new(),
                vec![b"peer-private".to_vec()],
            ),
            6,
        )
        .expect("record peer app");
    let message = runtime_app_message(app_id, peer_app_id, 7, b"hello peer".to_vec());
    let accepted = runtime
        .dispatch_app_message(message.clone(), 7)
        .expect("app message");
    assert_eq!(accepted, message);

    assert_eq!(runtime.events().len(), 13);
    for (seq, entry) in runtime.events().iter().enumerate() {
        assert_eq!(entry.event.seq, seq as u64);
        if seq == 0 {
            assert_eq!(entry.event.previous_event_sha256, [0; 32]);
        } else {
            assert_eq!(
                entry.event.previous_event_sha256,
                runtime.events()[seq - 1].event_sha256
            );
        }
    }
}

#[test]
fn runtime_brokers_browser_object_storage_provider() {
    let app_id = sha256(b"browser-app");
    let release_id = sha256(b"browser-release");
    let namespace = b"browser-private".to_vec();
    let app = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        vec![namespace.clone()],
    );
    let storage = BrowserRuntimeStorage::new(TestBrowserStorage::default());
    let mut runtime = RuntimeKernel::new(storage, sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(app, 1)
        .expect("record app");
    grant_storage_provider(
        &mut runtime,
        app_id,
        release_id,
        &namespace,
        CAPABILITY_OPERATION_WRITE,
        2,
        RUNTIME_STORAGE_BACKING_BROWSER,
    );
    grant_storage_provider(
        &mut runtime,
        app_id,
        release_id,
        &namespace,
        CAPABILITY_OPERATION_READ,
        3,
        RUNTIME_STORAGE_BACKING_BROWSER,
    );

    let key = sha256(b"browser-key");
    let value = b"browser durable value".to_vec();
    let write_request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        app_id,
        release_id,
        key,
        sha256(&value),
        namespace.clone(),
        value.clone(),
        b"browser-write".to_vec(),
    );
    let write_request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(write_request));
    let write_response_bytes = runtime
        .invoke_storage_wire(&write_request_bytes, b"browser-storage", 4)
        .expect("browser write");
    let write_response =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&write_response_bytes)
            .expect("wire response");
    let write_response = match write_response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(write_response.status, CAPABILITY_STATUS_OK);

    let read_request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_READ,
        2,
        app_id,
        release_id,
        key,
        sha256(&value),
        namespace,
        Vec::new(),
        b"browser-read".to_vec(),
    );
    let read_request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(read_request));
    let read_response_bytes = runtime
        .invoke_storage_wire(&read_request_bytes, b"browser-storage", 5)
        .expect("browser read");
    let read_response =
        from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&read_response_bytes)
            .expect("wire response");
    let read_response = match read_response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(read_response.status, CAPABILITY_STATUS_OK);
    assert_eq!(read_response.payload, value);
    assert_eq!(
        runtime.events().last().unwrap().event.event_kind,
        RUNTIME_EVENT_CAPABILITY_EXECUTED
    );
}

#[test]
fn runtime_routes_storage_object_put_through_admitted_capability_envelope() {
    let runtime_id = sha256(b"runtime");
    let app_id = sha256(b"envelope-app");
    let release_id = sha256(b"envelope-release");
    let namespace = b"private".to_vec();
    let app = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        vec![namespace.clone()],
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), runtime_id);
    runtime
        .record_app_runtime_projection(app, 1)
        .expect("record app");
    let grant = grant_storage_provider(
        &mut runtime,
        app_id,
        release_id,
        &namespace,
        CAPABILITY_OPERATION_WRITE,
        2,
        RUNTIME_STORAGE_BACKING_MEMORY,
    );
    let capability_id = sha256(b"object-store");
    let session = runtime_capability_session(
        &grant,
        capability_id,
        runtime_id,
        sha256(b"admission"),
        sha256(b"route"),
        50,
    );
    runtime
        .open_capability_session(session.clone(), 3)
        .expect("open storage session");

    let value = b"session-bound object".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        app_id,
        release_id,
        sha256(b"object-key"),
        sha256(&value),
        namespace,
        value.clone(),
        b"envelope-write".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
    let envelope = capability_envelope(
        session.session_id,
        capability_invocation_id(
            session.session_id,
            CAPABILITY_OPERATION_OBJECT_PUT,
            0,
            blake3_hash(&request_bytes),
        ),
        capability_id,
        app_id,
        runtime_id,
        CAPABILITY_PACKET_INVOKE,
        CAPABILITY_OPERATION_OBJECT_PUT,
        CAPABILITY_CONTENT_OBJECT,
        0,
        4,
        request_bytes,
    );

    let response_bytes = runtime
        .invoke_storage_envelope(&envelope, b"memory-storage", 4)
        .expect("invoke envelope");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_OK);
    assert_eq!(
        runtime.events().last().unwrap().event.event_kind,
        RUNTIME_EVENT_CAPABILITY_EXECUTED
    );
}

#[test]
fn runtime_denies_storage_envelope_without_admitted_session() {
    let runtime_id = sha256(b"runtime");
    let app_id = sha256(b"envelope-app");
    let release_id = sha256(b"envelope-release");
    let namespace = b"private".to_vec();
    let app = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        vec![namespace.clone()],
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), runtime_id);
    runtime
        .record_app_runtime_projection(app, 1)
        .expect("record app");
    grant_storage_provider(
        &mut runtime,
        app_id,
        release_id,
        &namespace,
        CAPABILITY_OPERATION_WRITE,
        2,
        RUNTIME_STORAGE_BACKING_MEMORY,
    );

    let value = b"session-bound object".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        app_id,
        release_id,
        sha256(b"object-key"),
        sha256(&value),
        namespace,
        value,
        b"envelope-write".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
    let envelope = capability_envelope(
        sha256(b"missing-session"),
        capability_invocation_id(
            sha256(b"missing-session"),
            CAPABILITY_OPERATION_OBJECT_PUT,
            0,
            blake3_hash(&request_bytes),
        ),
        sha256(b"object-store"),
        app_id,
        runtime_id,
        CAPABILITY_PACKET_INVOKE,
        CAPABILITY_OPERATION_OBJECT_PUT,
        CAPABILITY_CONTENT_OBJECT,
        0,
        4,
        request_bytes,
    );

    let response_bytes = runtime
        .invoke_storage_envelope(&envelope, b"memory-storage", 4)
        .expect("denial response");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_POLICY_DENIED);
    assert_eq!(
        runtime.events().last().unwrap().event.event_kind,
        RUNTIME_EVENT_CAPABILITY_DENIED
    );
}

#[test]
fn runtime_rejects_storage_write_payload_hash_mismatch() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let namespace = b"private".to_vec();
    let app = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        vec![namespace.clone()],
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(app, 1)
        .expect("record app");
    grant_storage_provider(
        &mut runtime,
        app_id,
        release_id,
        &namespace,
        CAPABILITY_OPERATION_WRITE,
        2,
        RUNTIME_STORAGE_BACKING_MEMORY,
    );

    let request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        app_id,
        release_id,
        sha256(b"object-key"),
        sha256(b"not the payload"),
        namespace,
        b"payload".to_vec(),
        b"bad-write".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
    let response_bytes = runtime
        .invoke_storage_wire(&request_bytes, b"memory-storage", 3)
        .expect("invalid response");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_INVALID_REQUEST);
    assert_eq!(response.payload, b"payload_sha256_mismatch".to_vec());
}

#[test]
fn runtime_rejects_session_without_admission_hash() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let namespace = b"private".to_vec();
    let app = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        vec![namespace.clone()],
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(app, 1)
        .expect("record app");
    let grant = grant_storage_provider(
        &mut runtime,
        app_id,
        release_id,
        &namespace,
        CAPABILITY_OPERATION_WRITE,
        2,
        RUNTIME_STORAGE_BACKING_MEMORY,
    );
    let session = runtime_capability_session(
        &grant,
        sha256(b"object-store"),
        sha256(b"provider"),
        [0; 32],
        sha256(b"route"),
        50,
    );

    assert_eq!(
        runtime.open_capability_session(session, 3),
        Err(RuntimeError::CapabilityDenied)
    );
    assert_eq!(
        runtime.events().last().unwrap().event.event_kind,
        RUNTIME_EVENT_CAPABILITY_SESSION_OPENED
    );
    assert_eq!(
        runtime.events().last().unwrap().event.status,
        CAPABILITY_STATUS_POLICY_DENIED
    );
}

#[test]
fn runtime_event_chain_verifier_rejects_broken_audit_chain() {
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                sha256(b"app"),
                sha256(b"release"),
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record app");
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                sha256(b"other-app"),
                sha256(b"other-release"),
                sha256(b"other-code"),
                sha256(b"other-developer"),
                sha256(b"other-manifest"),
                Vec::new(),
                Vec::new(),
            ),
            2,
        )
        .expect("record second app");

    assert!(verify_runtime_event_chain(runtime.events()));
    let mut broken = runtime.events().to_vec();
    broken[1].event.previous_event_sha256 = sha256(b"not previous");
    assert!(!verify_runtime_event_chain(&broken));

    let mut broken_payload = runtime.events().to_vec();
    broken_payload[0].event.payload = b"tampered".to_vec();
    assert!(!verify_runtime_event_chain(&broken_payload));
}

#[test]
fn runtime_records_app_graph_with_bound_runtime_record() {
    let app_id = sha256(b"packaged-app");
    let release_id = sha256(b"packaged-release");
    let manifest_sha256 = sha256(b"app manifest");
    let route = runtime_http_route(
        app_id,
        release_id,
        ROUTE_SCHEME_HTTPS,
        b"dash.edgerun.tech".to_vec(),
        b"/v1/order".to_vec(),
    );
    let runtime_projection = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        manifest_sha256,
        vec![route.clone()],
        vec![b"private".to_vec()],
    );
    let graph = AppGraphRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        developer_public_key: runtime_projection.developer_id,
        app_manifest_sha256: manifest_sha256,
        app_slug: b"packaged-app".to_vec(),
        runtime_install: runtime_projection,
        artifacts: vec![edgerun_protocols::wire::AppArtifactRecord {
            kind: 1,
            path: b"app.edapp".to_vec(),
            sha256: manifest_sha256,
        }],
    };
    let graph_bytes = sdk_wire_bytes(&SdkWireRecord::AppGraph(graph));
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_graph_wire_runtime_projection(&graph_bytes, 1)
        .expect("record app graph");

    assert_eq!(runtime.events().len(), 1);
    assert_eq!(
        runtime.events()[0].event.event_kind,
        RUNTIME_EVENT_APP_INSTALLED
    );

    grant_network_route(&mut runtime, app_id, release_id, b"dash.edgerun.tech", 2);
    runtime
        .grant_http_route(route, 2)
        .expect("grant declared route");
    let dispatch = runtime
        .dispatch_http(
            runtime_http_request(
                HTTP_METHOD_GET,
                ROUTE_SCHEME_HTTPS,
                b"dash.edgerun.tech".to_vec(),
                b"/v1/order/123".to_vec(),
                [0; 32],
                Vec::new(),
            ),
            3,
        )
        .expect("dispatch declared route");
    assert_eq!(dispatch.app_id, app_id);
    assert_eq!(dispatch.release_id, release_id);
}

#[test]
fn runtime_records_first_run_cache_projection() {
    let profile_id = sha256(b"profile");
    let app_id = sha256(b"network-app");
    let release_id = sha256(b"release");
    let package_sha256 = sha256(b"signed package bytes");
    let app = runtime_app_projection(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        Vec::new(),
    );
    let decision = first_run_decision(
        profile_id,
        &app,
        package_sha256,
        APP_RUN_DECISION_VERIFY_AND_CACHE,
        10,
    );
    let cache = verified_package_cache(profile_id, &app, package_sha256, 10);
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));

    runtime
        .record_app_first_run_projection(app.clone(), decision.clone(), Some(cache.clone()), 10)
        .expect("record first run");

    assert_eq!(
        runtime.app_run_decisions().get(&decision.decision_id),
        Some(&decision)
    );
    assert_eq!(runtime.package_caches().get(&cache.cache_id), Some(&cache));
    assert_eq!(runtime.events().len(), 3);
    assert_eq!(
        runtime.events()[0].event.event_kind,
        RUNTIME_EVENT_PACKAGE_VERIFIED
    );
    assert_eq!(
        runtime.events()[1].event.event_kind,
        RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED
    );
    assert_eq!(
        runtime.events()[2].event.event_kind,
        RUNTIME_EVENT_PACKAGE_CACHE_UPDATED
    );

    runtime
        .grant_runtime_capability(
            runtime_capability_grant(
                profile_id,
                sha256(b"user"),
                app_id,
                release_id,
                CAPABILITY_KIND_STORAGE,
                CAPABILITY_OPERATION_READ,
                1,
                sha256(b"private"),
                sha256(b"constraints"),
                10,
                20,
                b"user-signature".to_vec(),
            ),
            11,
        )
        .expect("verified app is runnable");
}

#[test]
fn runtime_cancel_first_run_does_not_make_app_runnable() {
    let profile_id = sha256(b"profile");
    let app = runtime_app_projection(
        sha256(b"network-app"),
        sha256(b"release"),
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        Vec::new(),
    );
    let decision = first_run_decision(
        profile_id,
        &app,
        sha256(b"signed package bytes"),
        APP_RUN_DECISION_CANCEL,
        10,
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));

    runtime
        .record_app_first_run_projection(app.clone(), decision.clone(), None, 10)
        .expect("record cancel");

    assert_eq!(
        runtime.app_run_decisions().get(&decision.decision_id),
        Some(&decision)
    );
    assert!(runtime.package_caches().is_empty());
    assert_eq!(runtime.events().len(), 2);
    assert_eq!(
        runtime.grant_runtime_capability(
            runtime_capability_grant(
                profile_id,
                sha256(b"user"),
                app.app_id,
                app.release_id,
                CAPABILITY_KIND_STORAGE,
                CAPABILITY_OPERATION_READ,
                1,
                sha256(b"private"),
                sha256(b"constraints"),
                10,
                20,
                b"user-signature".to_vec(),
            ),
            11,
        ),
        Err(RuntimeError::AppNotVerified)
    );
}

#[test]
fn runtime_rejects_first_run_cache_not_bound_to_app() {
    let profile_id = sha256(b"profile");
    let package_sha256 = sha256(b"signed package bytes");
    let app = runtime_app_projection(
        sha256(b"network-app"),
        sha256(b"release"),
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        Vec::new(),
    );
    let decision = first_run_decision(
        profile_id,
        &app,
        package_sha256,
        APP_RUN_DECISION_VERIFY_AND_CACHE,
        10,
    );
    let mut cache = verified_package_cache(profile_id, &app, package_sha256, 10);
    cache.package_sha256 = sha256(b"other package");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));

    assert_eq!(
        runtime.record_app_first_run_projection(app, decision, Some(cache), 10),
        Err(RuntimeError::InvalidWireRecord)
    );
    assert!(runtime.app_run_decisions().is_empty());
    assert!(runtime.package_caches().is_empty());
    assert_eq!(runtime.events().len(), 1);
    assert_eq!(
        runtime.events()[0].event.event_kind,
        RUNTIME_EVENT_PACKAGE_CACHE_UPDATED
    );
    assert_eq!(
        runtime.events()[0].event.status,
        CAPABILITY_STATUS_POLICY_DENIED
    );
}

#[test]
fn runtime_denies_app_message_to_unknown_app() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"private".to_vec()],
            ),
            1,
        )
        .expect("record app");
    let message = runtime_app_message(app_id, sha256(b"missing-app"), 1, b"payload".to_vec());
    assert_eq!(
        runtime.dispatch_app_message(message, 2),
        Err(RuntimeError::AppNotVerified)
    );
    assert_eq!(runtime.events().len(), 2);
    assert_eq!(
        runtime.events()[1].event.event_kind,
        RUNTIME_EVENT_APP_MESSAGE_DISPATCHED
    );
    assert_eq!(runtime.events()[1].event.status, APP_MESSAGE_STATUS_DENIED);
}

#[test]
fn runtime_routes_unknown_local_recipient_to_mesh_identity_route() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let remote_app_id = sha256(b"remote-app");
    let remote_runtime_id = sha256(b"remote-runtime");
    let remote_node_id = [9u8; 64];
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"private".to_vec()],
            ),
            1,
        )
        .expect("record app");
    runtime
        .grant_identity_route(
            runtime_identity_route(
                remote_app_id,
                remote_runtime_id,
                remote_node_id,
                2,
                u64::MAX,
                b"mesh".to_vec(),
            ),
            2,
        )
        .expect("route");
    let message = runtime_app_message(app_id, remote_app_id, 42, b"hello remote".to_vec());
    let delivery = runtime
        .route_app_message(message.clone(), 3)
        .expect("route");
    let RuntimeMessageDelivery::Remote(routed) = delivery else {
        panic!("expected remote delivery");
    };
    assert_eq!(routed.from_runtime_id, sha256(b"runtime"));
    assert_eq!(routed.to_runtime_id, remote_runtime_id);
    assert_eq!(routed.to_node_id, remote_node_id);
    assert_eq!(routed.message, message);
    assert_eq!(
        routed.message_sha256,
        sha256(&sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message)))
    );
    assert_eq!(runtime.events().len(), 3);
    assert_eq!(
        runtime.events()[2].event.event_kind,
        RUNTIME_EVENT_APP_MESSAGE_FORWARDED
    );
    assert_eq!(
        runtime.events()[2].event.status,
        APP_MESSAGE_STATUS_FORWARDED
    );
}

#[test]
fn runtime_rejects_expired_identity_route() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let remote_app_id = sha256(b"remote-app");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record app");
    runtime
        .grant_identity_route(
            runtime_identity_route(
                remote_app_id,
                sha256(b"remote-runtime"),
                [9u8; 64],
                2,
                3,
                b"mesh".to_vec(),
            ),
            2,
        )
        .expect("route");

    let message = runtime_app_message(app_id, remote_app_id, 42, b"late".to_vec());
    assert_eq!(
        runtime.route_app_message(message, 4),
        Err(RuntimeError::IdentityRouteNotFound)
    );
    assert_eq!(runtime.events().len(), 3);
    assert_eq!(runtime.events()[2].event.status, APP_MESSAGE_STATUS_DENIED);
}

#[test]
fn runtime_identity_routing_still_delivers_local_recipient_locally() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let peer_app_id = sha256(b"peer-app");
    let peer_release_id = sha256(b"peer-release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record sender app");
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                peer_app_id,
                peer_release_id,
                sha256(b"peer-code"),
                sha256(b"peer-developer"),
                sha256(b"peer-manifest"),
                Vec::new(),
                Vec::new(),
            ),
            2,
        )
        .expect("record recipient app");
    let message = runtime_app_message(app_id, peer_app_id, 7, b"hello peer".to_vec());
    let delivery = runtime
        .route_app_message(message.clone(), 3)
        .expect("route");
    assert_eq!(delivery, RuntimeMessageDelivery::Local(message));
    assert_eq!(runtime.events().len(), 3);
    assert_eq!(
        runtime.events()[2].event.event_kind,
        RUNTIME_EVENT_APP_MESSAGE_DISPATCHED
    );
    assert_eq!(
        runtime.events()[2].event.status,
        APP_MESSAGE_STATUS_ACCEPTED
    );
}

#[test]
fn runtime_rechecks_http_route_grant_at_dispatch() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let route = runtime_http_route(
        app_id,
        release_id,
        ROUTE_SCHEME_HTTPS,
        b"example.com".to_vec(),
        b"/app/".to_vec(),
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                vec![route.clone()],
                Vec::new(),
            ),
            1,
        )
        .expect("record app");
    grant_network_route(&mut runtime, app_id, release_id, b"example.com", 2);
    runtime.grant_http_route(route, 2).expect("route");

    assert_eq!(
        runtime.dispatch_http(
            runtime_http_request(
                HTTP_METHOD_GET,
                ROUTE_SCHEME_HTTPS,
                b"example.com".to_vec(),
                b"/app/index".to_vec(),
                [0; 32],
                Vec::new(),
            ),
            101,
        ),
        Err(RuntimeError::RouteDenied)
    );
    assert_eq!(
        runtime.events().last().unwrap().event.status,
        CAPABILITY_STATUS_POLICY_DENIED
    );
}

#[test]
fn runtime_denies_storage_namespace_crossing() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"private".to_vec()],
            ),
            1,
        )
        .expect("record app");
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_READ,
        2,
        app_id,
        release_id,
        sha256(b"state-key"),
        [0; 32],
        b"other-app-private".to_vec(),
        Vec::new(),
        b"nonce".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
    let response_bytes = runtime
        .invoke_storage_wire(&request_bytes, b"runtime-storage", 2)
        .expect("denial response");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_POLICY_DENIED);
    assert_eq!(runtime.events().len(), 2);
}

#[test]
fn runtime_records_user_choice_storage_binding_and_session() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let profile_id = sha256(b"profile");
    let user_id = sha256(b"user");
    let scope = sha256(b"chat-storage");
    let constraints = sha256(b"allow-while-open");
    let provider_id = sha256(b"browser-indexeddb-provider");
    let capability_id = sha256(b"object-store/chat");
    let provider_node_id = sha256(b"provider-node");
    let admission_hash = sha256(b"admission");
    let route_commitment = sha256(b"route");

    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"chat".to_vec()],
            ),
            1,
        )
        .expect("record app");

    let grant = runtime_capability_grant(
        profile_id,
        user_id,
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        scope,
        constraints,
        2,
        100,
        b"user-signature".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), 2)
        .expect("grant");

    let binding = runtime_storage_binding(
        &grant,
        b"chat".to_vec(),
        provider_id,
        capability_id,
        RUNTIME_STORAGE_BACKING_BROWSER,
    );
    runtime
        .bind_storage_provider(binding.clone(), 3)
        .expect("storage binding");

    let session = runtime_capability_session(
        &grant,
        capability_id,
        provider_node_id,
        admission_hash,
        route_commitment,
        99,
    );
    runtime
        .open_capability_session(session.clone(), 4)
        .expect("session");

    assert_eq!(
        runtime.capability_grants().get(&grant.grant_id),
        Some(&grant)
    );
    assert_eq!(
        runtime.storage_bindings().get(&binding.binding_id),
        Some(&binding)
    );
    assert_eq!(
        runtime.capability_sessions().get(&session.session_id),
        Some(&session)
    );
    assert_eq!(
        runtime.events()[1].event.event_kind,
        RUNTIME_EVENT_CAPABILITY_GRANTED
    );
    assert_eq!(
        runtime.events()[2].event.event_kind,
        RUNTIME_EVENT_STORAGE_BOUND
    );
    assert_eq!(
        runtime.events()[3].event.event_kind,
        RUNTIME_EVENT_CAPABILITY_SESSION_OPENED
    );
}

#[test]
fn runtime_rejects_binding_before_grant_is_live() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"chat".to_vec()],
            ),
            1,
        )
        .expect("record app");

    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"chat-storage"),
        sha256(b"constraints"),
        10,
        20,
        b"user-signature".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), 2)
        .expect("grant");
    let binding = runtime_storage_binding(
        &grant,
        b"chat".to_vec(),
        sha256(b"provider"),
        sha256(b"capability"),
        RUNTIME_STORAGE_BACKING_BROWSER,
    );

    assert_eq!(
        runtime.bind_storage_provider(binding, 9),
        Err(RuntimeError::CapabilityDenied)
    );
    assert_eq!(
        runtime.events().last().unwrap().event.status,
        CAPABILITY_STATUS_POLICY_DENIED
    );
}

#[test]
fn runtime_rejects_tampered_runtime_record_ids() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"chat".to_vec()],
            ),
            1,
        )
        .expect("record app");

    let mut grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"chat-storage"),
        sha256(b"constraints"),
        2,
        20,
        b"user-signature".to_vec(),
    );
    grant.grant_id = sha256(b"tampered-grant-id");
    assert_eq!(
        runtime.grant_runtime_capability(grant, 2),
        Err(RuntimeError::CapabilityDenied)
    );

    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"chat-storage"),
        sha256(b"constraints"),
        2,
        20,
        b"user-signature".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), 2)
        .expect("grant");

    let mut binding = runtime_storage_binding(
        &grant,
        b"chat".to_vec(),
        sha256(b"provider"),
        sha256(b"capability"),
        RUNTIME_STORAGE_BACKING_BROWSER,
    );
    let valid_binding = binding.clone();
    binding.binding_id = sha256(b"tampered-binding-id");
    assert_eq!(
        runtime.bind_storage_provider(binding, 3),
        Err(RuntimeError::CapabilityDenied)
    );
    runtime
        .bind_storage_provider(valid_binding, 3)
        .expect("binding");

    let mut session = runtime_capability_session(
        &grant,
        sha256(b"capability"),
        sha256(b"provider-node"),
        sha256(b"admission"),
        sha256(b"route"),
        20,
    );
    session.session_id = sha256(b"tampered-session-id");
    assert_eq!(
        runtime.open_capability_session(session, 4),
        Err(RuntimeError::CapabilityDenied)
    );
}

#[test]
fn runtime_rejects_storage_binding_backed_by_network_grant() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"chat".to_vec()],
            ),
            1,
        )
        .expect("record app");

    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_NETWORK,
        CAPABILITY_OPERATION_RECEIVE,
        2,
        sha256(b"chat-storage"),
        sha256(b"constraints"),
        2,
        20,
        b"user-signature".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), 2)
        .expect("grant");
    let binding = runtime_storage_binding(
        &grant,
        b"chat".to_vec(),
        sha256(b"provider"),
        sha256(b"capability"),
        RUNTIME_STORAGE_BACKING_BROWSER,
    );

    assert_eq!(
        runtime.bind_storage_provider(binding, 3),
        Err(RuntimeError::CapabilityDenied)
    );
    assert_eq!(
        runtime.events().last().unwrap().event.status,
        CAPABILITY_STATUS_POLICY_DENIED
    );
}

#[test]
fn runtime_rejects_session_outside_grant_window() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record app");

    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"scope"),
        sha256(b"constraints"),
        10,
        20,
        b"user-signature".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), 2)
        .expect("grant");
    let session = runtime_capability_session(
        &grant,
        sha256(b"capability"),
        sha256(b"provider-node"),
        sha256(b"admission"),
        sha256(b"route"),
        20,
    );

    assert_eq!(
        runtime.open_capability_session(session.clone(), 9),
        Err(RuntimeError::CapabilityDenied)
    );
    assert_eq!(
        runtime.open_capability_session(session, 21),
        Err(RuntimeError::CapabilityDenied)
    );
}

#[test]
fn runtime_rejects_session_without_matching_provider_binding() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                vec![b"chat".to_vec()],
            ),
            1,
        )
        .expect("record app");

    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"chat-storage"),
        sha256(b"constraints"),
        2,
        20,
        b"user-signature".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant.clone(), 2)
        .expect("grant");
    let binding = runtime_storage_binding(
        &grant,
        b"chat".to_vec(),
        sha256(b"provider"),
        sha256(b"bound-capability"),
        RUNTIME_STORAGE_BACKING_BROWSER,
    );
    runtime
        .bind_storage_provider(binding, 3)
        .expect("storage binding");
    let session = runtime_capability_session(
        &grant,
        sha256(b"other-capability"),
        sha256(b"provider-node"),
        sha256(b"admission"),
        sha256(b"route"),
        20,
    );

    assert_eq!(
        runtime.open_capability_session(session, 4),
        Err(RuntimeError::CapabilityDenied)
    );
}

#[test]
fn runtime_brokers_app_signing_without_exposing_private_key() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut app_signer = TestAppSigner::default();
    app_signer.insert(app_id, b"app-public-key".to_vec());
    let mut runtime = RuntimeKernel::with_signer_and_app_signer(
        MemoryRuntimeStorage::default(),
        UnsignedRuntimeSigner::new(sha256(b"runtime")),
        app_signer,
    );
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record app");
    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_SIGNING,
        CAPABILITY_OPERATION_SIGN,
        2,
        sha256(b"app-signing"),
        sha256(b"signing-policy"),
        2,
        100,
        b"user-signature-signing".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant, 2)
        .expect("signing grant");
    let payload = b"message to sign".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_SIGNING,
        CAPABILITY_OPERATION_SIGN,
        2,
        app_id,
        release_id,
        sha256(b"message-id"),
        sha256(&payload),
        b"app-signing".to_vec(),
        payload,
        b"nonce".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request.clone()));
    let response_bytes = runtime
        .invoke_signing_wire(&request_bytes, b"runtime-app-signer", 2)
        .expect("sign");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_OK);
    assert_eq!(response.capability_kind, CAPABILITY_KIND_SIGNING);
    assert_eq!(response.operation, CAPABILITY_OPERATION_SIGN);
    assert_eq!(
        response.payload,
        signing_response_payload(b"app-public-key", &{
            let mut signature = Vec::new();
            signature.extend_from_slice(&sha256(&signing_capability_input(&request)));
            signature.extend_from_slice(&sha256(&app_id));
            signature
        })
    );
    assert_eq!(runtime.events().len(), 3);
    assert_eq!(
        runtime.events()[2].event.event_kind,
        RUNTIME_EVENT_CAPABILITY_EXECUTED
    );
}

#[test]
fn runtime_denies_signing_without_matching_grant() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record app");
    let payload = b"message to sign".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_SIGNING,
        CAPABILITY_OPERATION_SIGN,
        2,
        app_id,
        release_id,
        sha256(b"message-id"),
        sha256(&payload),
        b"app-signing".to_vec(),
        payload,
        b"nonce".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
    let response_bytes = runtime
        .invoke_signing_wire(&request_bytes, b"runtime-app-signer", 2)
        .expect("denied response");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_POLICY_DENIED);
    assert_eq!(runtime.events().len(), 2);
    assert_eq!(
        runtime.events()[1].event.event_kind,
        RUNTIME_EVENT_CAPABILITY_DENIED
    );
}

#[test]
fn runtime_denies_signing_when_assurance_is_too_low() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut app_signer = TestAppSigner::default();
    app_signer.insert(app_id, b"app-public-key".to_vec());
    let mut runtime = RuntimeKernel::with_signer_and_app_signer(
        MemoryRuntimeStorage::default(),
        UnsignedRuntimeSigner::new(sha256(b"runtime")),
        app_signer,
    );
    runtime
        .record_app_runtime_projection(
            runtime_app_projection(
                app_id,
                release_id,
                sha256(b"code"),
                sha256(b"developer"),
                sha256(b"manifest"),
                Vec::new(),
                Vec::new(),
            ),
            1,
        )
        .expect("record app");
    let grant = runtime_capability_grant(
        sha256(b"profile"),
        sha256(b"user"),
        app_id,
        release_id,
        CAPABILITY_KIND_SIGNING,
        CAPABILITY_OPERATION_SIGN,
        3,
        sha256(b"app-signing"),
        sha256(b"signing-policy"),
        2,
        100,
        b"user-signature-signing".to_vec(),
    );
    runtime
        .grant_runtime_capability(grant, 2)
        .expect("signing grant");
    let payload = b"message to sign".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_SIGNING,
        CAPABILITY_OPERATION_SIGN,
        2,
        app_id,
        release_id,
        sha256(b"message-id"),
        sha256(&payload),
        b"app-signing".to_vec(),
        payload,
        b"nonce".to_vec(),
    );
    let request_bytes = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(request));
    let response_bytes = runtime
        .invoke_signing_wire(&request_bytes, b"runtime-app-signer", 2)
        .expect("denied response");
    let response = from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(&response_bytes)
        .expect("wire response");
    let response = match response {
        SdkWireRecord::CapabilityResponse(response) => response,
        _ => panic!("unexpected response"),
    };
    assert_eq!(response.status, CAPABILITY_STATUS_POLICY_DENIED);
}

#[test]
fn service_plan_derives_protocols_domains_and_mail() {
    let app_id = sha256(b"mail-app");
    let release_id = sha256(b"release");
    let route = runtime_http_route(
        app_id,
        release_id,
        ROUTE_SCHEME_HTTPS,
        b"example.com".to_vec(),
        b"/".to_vec(),
    );
    let domain = RuntimeDomainConfig {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        domain: b"example.com".to_vec(),
        authoritative_dns: true,
        mail_enabled: true,
        acme_enabled: true,
        mailboxes: vec![RuntimeMailbox {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            address: b"admin@example.com".to_vec(),
            target_app_id: app_id,
        }],
        routes: vec![route.clone()],
    };
    let config = runtime_deployment_config(
        sha256(b"runtime"),
        [203, 0, 113, 10],
        b"runtime.example.com".to_vec(),
        b"example.com".to_vec(),
        b"admin@example.com".to_vec(),
        Vec::new(),
        vec![domain],
        vec![runtime_app_projection(
            app_id,
            release_id,
            sha256(b"code"),
            sha256(b"developer"),
            sha256(b"manifest"),
            vec![route],
            vec![b"private".to_vec()],
        )],
    );
    let plan = RuntimeServicePlan::from_deployment(&config);
    assert!(plan.requires_dns());
    assert!(plan.requires_acme());
    assert!(plan
        .listeners
        .iter()
        .any(|binding| binding.protocol == RUNTIME_PROTOCOL_HTTPS && binding.port == 443));
    assert!(plan
        .listeners
        .iter()
        .any(|binding| binding.protocol == RUNTIME_PROTOCOL_SMTP && binding.port == 25));
    assert_eq!(plan.mail_domains(), vec![b"example.com".to_vec()]);
}

#[test]
fn service_plan_derives_app_capability_declarations() {
    let app_id = sha256(b"email-app");
    let release_id = sha256(b"email-release");
    let provided = runtime_capability_declaration(
        CAPABILITY_KIND_STORAGE,
        CAPABILITY_OPERATION_WRITE,
        2,
        sha256(b"mailbox-delivery-scope"),
        b"mailbox.delivery".to_vec(),
        b"node-authorized-email-app".to_vec(),
    );
    let required = runtime_capability_declaration(
        edgerun_protocols::wire::CAPABILITY_KIND_NETWORK,
        edgerun_protocols::wire::CAPABILITY_OPERATION_RECEIVE,
        2,
        sha256(b"smtp-ingress-scope"),
        b"smtp.ingress".to_vec(),
        b"node-owned-listener".to_vec(),
    );
    let runtime_projection = runtime_app_projection_with_capabilities(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        Vec::new(),
        vec![b"mailbox-state".to_vec()],
        vec![provided.clone()],
        vec![required.clone()],
    );
    let config = runtime_deployment_config(
        sha256(b"runtime"),
        [203, 0, 113, 10],
        b"runtime.example.com".to_vec(),
        b"example.com".to_vec(),
        b"admin@example.com".to_vec(),
        Vec::new(),
        Vec::new(),
        vec![runtime_projection],
    );
    let plan = RuntimeServicePlan::from_deployment(&config);
    assert_eq!(plan.provided_capabilities(), vec![provided]);
    assert_eq!(plan.required_capabilities(), vec![required]);
}

#[test]
fn same_service_plan_decides_native_and_wasm_boundaries() {
    use crate::network::{
        native_socket_binds, NodeTransportSurface, ServiceBindingDecision, TransportCarrier,
        TransportProtocol,
    };
    use crate::runtime::{decide_runtime_boundary, RuntimeBoundarySurface, RuntimeServicePlan};

    let app_id = sha256(b"portable-app");
    let release_id = sha256(b"portable-release");
    let config = runtime_deployment_config(
        sha256(b"runtime"),
        [203, 0, 113, 10],
        b"runtime.example.com".to_vec(),
        b"example.com".to_vec(),
        b"admin@example.com".to_vec(),
        Vec::new(),
        Vec::new(),
        vec![runtime_app_projection(
            app_id,
            release_id,
            sha256(b"code"),
            sha256(b"developer"),
            sha256(b"manifest"),
            Vec::new(),
            vec![b"portable-app/state".to_vec()],
        )],
    );
    let plan = RuntimeServicePlan::from_deployment(&config);

    let native = decide_runtime_boundary(&plan, RuntimeBoundarySurface::NATIVE);
    let wasm = decide_runtime_boundary(&plan, RuntimeBoundarySurface::WASM);

    assert_eq!(native.bindings.len(), wasm.bindings.len());
    assert!(native.bindings.iter().any(|decision| {
        matches!(
            decision,
            ServiceBindingDecision::Denied(binding) if binding.port == 0
        )
    }));
    assert!(native.bindings.iter().all(|decision| match decision {
        ServiceBindingDecision::NativeSocket(binding) => binding.port != 0,
        ServiceBindingDecision::Denied(binding) => binding.port == 0,
        ServiceBindingDecision::Routed(_) => false,
    }));
    let native_binds = native_socket_binds(&native.bindings);
    assert_eq!(
        native_binds.len(),
        native
            .bindings
            .iter()
            .filter(|decision| matches!(decision, ServiceBindingDecision::NativeSocket(_)))
            .count()
    );
    assert!(native_binds
        .iter()
        .all(|bind| bind.address.carrier == TransportCarrier::HostSocket));
    assert!(native_binds.iter().any(|bind| {
        bind.binding.protocol == RUNTIME_PROTOCOL_DNS_UDP
            && bind.address.protocol == TransportProtocol::Datagram
    }));
    assert!(wasm
        .bindings
        .iter()
        .all(|decision| matches!(decision, ServiceBindingDecision::Routed(_))));
    assert!(native_socket_binds(&wasm.bindings).is_empty());

    let native_namespaces: Vec<_> = native
        .storage
        .iter()
        .map(storage_decision_namespace)
        .collect();
    let wasm_namespaces: Vec<_> = wasm
        .storage
        .iter()
        .map(storage_decision_namespace)
        .collect();
    assert_eq!(native_namespaces, wasm_namespaces);
    assert_eq!(native_namespaces, vec![b"portable-app/state".to_vec()]);

    assert!(native.storage.iter().all(|decision| matches!(
        decision,
        RuntimeStorageDecision::Provider(intent)
            if intent.surface == RuntimeStorageSurface::HostDurable
    )));
    assert!(wasm.storage.iter().all(|decision| matches!(
        decision,
        RuntimeStorageDecision::Provider(intent)
            if intent.surface == RuntimeStorageSurface::BrowserDurable
    )));

    let replay = decide_runtime_boundary(&plan, RuntimeBoundarySurface::REPLAY);
    assert!(replay
        .bindings
        .iter()
        .all(|decision| matches!(decision, ServiceBindingDecision::Routed(_))));
    assert!(plan
        .requested_bindings(NodeTransportSurface::NativeSocket)
        .iter()
        .all(|intent| intent.surface == NodeTransportSurface::NativeSocket));
}

fn storage_decision_namespace(decision: &RuntimeStorageDecision) -> Vec<u8> {
    match decision {
        RuntimeStorageDecision::Provider(intent) | RuntimeStorageDecision::Denied(intent) => {
            intent.namespace.clone()
        }
    }
}

#[test]
fn service_plan_merges_node_provider_apps_after_deployment_apps() {
    let deployment_app = runtime_app_projection(
        sha256(b"deployment-app"),
        sha256(b"deployment-release"),
        sha256(b"deployment-code"),
        sha256(b"deployment-developer"),
        sha256(b"deployment-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let provider_app = runtime_app_projection(
        sha256(b"provider-app"),
        sha256(b"provider-release"),
        sha256(b"provider-code"),
        sha256(b"provider-developer"),
        sha256(b"provider-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let duplicate_provider_app = runtime_app_projection(
        deployment_app.app_id,
        sha256(b"duplicate-release"),
        sha256(b"duplicate-code"),
        sha256(b"duplicate-developer"),
        sha256(b"duplicate-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let config = runtime_deployment_config(
        sha256(b"runtime"),
        [203, 0, 113, 10],
        b"runtime.example.com".to_vec(),
        b"example.com".to_vec(),
        b"admin@example.com".to_vec(),
        Vec::new(),
        Vec::new(),
        vec![deployment_app.clone()],
    );
    let plan = RuntimeServicePlan::from_deployment_with_apps(
        &config,
        vec![provider_app.clone(), duplicate_provider_app],
    );
    assert_eq!(plan.apps.len(), 2);
    assert_eq!(plan.apps[0], deployment_app);
    assert_eq!(plan.apps[1], provider_app);
}

#[test]
fn runtime_records_service_plan_apps_as_events() {
    let first = runtime_app_projection(
        sha256(b"first-app"),
        sha256(b"first-release"),
        sha256(b"first-code"),
        sha256(b"first-developer"),
        sha256(b"first-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let second = runtime_app_projection(
        sha256(b"second-app"),
        sha256(b"second-release"),
        sha256(b"second-code"),
        sha256(b"second-developer"),
        sha256(b"second-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let config = runtime_deployment_config(
        sha256(b"runtime"),
        [203, 0, 113, 10],
        b"runtime.example.com".to_vec(),
        b"example.com".to_vec(),
        b"admin@example.com".to_vec(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let plan =
        RuntimeServicePlan::from_deployment_with_apps(&config, vec![first.clone(), second.clone()]);
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    let recorded = runtime
        .record_service_plan_app_projections(&plan, 10)
        .unwrap();
    assert_eq!(recorded, 2);
    assert_eq!(runtime.events().len(), 2);
    assert_eq!(runtime.events()[0].event.seq, 0);
    assert_eq!(
        runtime.events()[0].event.event_kind,
        RUNTIME_EVENT_APP_INSTALLED
    );
    assert_eq!(runtime.events()[1].event.seq, 1);
    assert_eq!(
        runtime.events()[1].event.event_kind,
        RUNTIME_EVENT_APP_INSTALLED
    );

    let recorded_again = runtime
        .record_service_plan_app_projections(&plan, 20)
        .unwrap();
    assert_eq!(recorded_again, 0);
    assert_eq!(runtime.events().len(), 2);
}
