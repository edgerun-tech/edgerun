use super::*;
use crate::storage::MemoryRuntimeStorage;
use crate::storage::{RuntimeStorageDecision, RuntimeStorageSurface};
use alloc::vec;
use edgerun_protocols::wire::{
    from_bytes, CapabilityRequest, RuntimeDomainConfig, RuntimeMailbox, CAPABILITY_KIND_SIGNING,
    CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_READ, CAPABILITY_OPERATION_SIGN,
    CAPABILITY_OPERATION_WRITE, HTTP_METHOD_GET, ROUTE_SCHEME_HTTPS,
    RUNTIME_EVENT_CAPABILITY_EXECUTED, RUNTIME_PROTOCOL_DNS_UDP, RUNTIME_PROTOCOL_HTTPS,
    RUNTIME_PROTOCOL_SMTP,
};

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

#[test]
fn runtime_installs_routes_dispatches_and_brokers_storage() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let route = runtime_http_route(
        app_id,
        release_id,
        ROUTE_SCHEME_HTTPS,
        b"example.com".to_vec(),
        b"/app/".to_vec(),
    );
    let install = runtime_app_install(
        app_id,
        release_id,
        sha256(b"code"),
        sha256(b"developer"),
        sha256(b"manifest"),
        vec![route.clone()],
        vec![b"private".to_vec()],
    );
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime.install_app(install, 1).expect("install");
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
        .install_app(
            runtime_app_install(
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
        .expect("install peer");
    let message = runtime_app_message(app_id, peer_app_id, 7, b"hello peer".to_vec());
    let accepted = runtime
        .dispatch_app_message(message.clone(), 7)
        .expect("app message");
    assert_eq!(accepted, message);

    assert_eq!(runtime.events().len(), 7);
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
fn runtime_installs_app_graph_with_bound_install_record() {
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
    let install = runtime_app_install(
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
        developer_public_key: install.developer_id,
        app_manifest_sha256: manifest_sha256,
        app_slug: b"packaged-app".to_vec(),
        runtime_install: install,
        artifacts: vec![edgerun_protocols::wire::AppArtifactRecord {
            kind: 1,
            path: b"app.edapp".to_vec(),
            sha256: manifest_sha256,
        }],
    };
    let graph_bytes = sdk_wire_bytes(&SdkWireRecord::AppGraph(graph));
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .install_app_graph_wire(&graph_bytes, 1)
        .expect("install app graph");

    assert_eq!(runtime.events().len(), 1);
    assert_eq!(
        runtime.events()[0].event.event_kind,
        RUNTIME_EVENT_APP_INSTALLED
    );

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
fn runtime_denies_app_message_to_unknown_app() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .install_app(
            runtime_app_install(
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
        .expect("install");
    let message = runtime_app_message(app_id, sha256(b"missing-app"), 1, b"payload".to_vec());
    assert_eq!(
        runtime.dispatch_app_message(message, 2),
        Err(RuntimeError::AppNotInstalled)
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
        .install_app(
            runtime_app_install(
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
        .expect("install");
    runtime
        .grant_identity_route(
            runtime_identity_route(
                remote_app_id,
                remote_runtime_id,
                remote_node_id,
                2,
                0,
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
fn runtime_identity_routing_still_delivers_local_recipient_locally() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let peer_app_id = sha256(b"peer-app");
    let peer_release_id = sha256(b"peer-release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .install_app(
            runtime_app_install(
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
        .expect("install sender");
    runtime
        .install_app(
            runtime_app_install(
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
        .expect("install recipient");
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
fn runtime_denies_storage_namespace_crossing() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .install_app(
            runtime_app_install(
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
        .expect("install");
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
        .install_app(
            runtime_app_install(
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
        .expect("install");
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
    assert_eq!(runtime.events().len(), 2);
    assert_eq!(
        runtime.events()[1].event.event_kind,
        RUNTIME_EVENT_CAPABILITY_EXECUTED
    );
}

#[test]
fn runtime_denies_signing_for_wrong_release_or_missing_key() {
    let app_id = sha256(b"app");
    let release_id = sha256(b"release");
    let mut runtime = RuntimeKernel::new(MemoryRuntimeStorage::default(), sha256(b"runtime"));
    runtime
        .install_app(
            runtime_app_install(
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
        .expect("install");
    let payload = b"message to sign".to_vec();
    let request = CapabilityRequest::new(
        CAPABILITY_KIND_SIGNING,
        CAPABILITY_OPERATION_SIGN,
        2,
        app_id,
        sha256(b"wrong-release"),
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
        vec![runtime_app_install(
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
    let install = runtime_app_install_with_capabilities(
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
        vec![install],
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
        vec![runtime_app_install(
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
    let deployment_app = runtime_app_install(
        sha256(b"deployment-app"),
        sha256(b"deployment-release"),
        sha256(b"deployment-code"),
        sha256(b"deployment-developer"),
        sha256(b"deployment-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let provider_app = runtime_app_install(
        sha256(b"provider-app"),
        sha256(b"provider-release"),
        sha256(b"provider-code"),
        sha256(b"provider-developer"),
        sha256(b"provider-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let duplicate_provider_app = runtime_app_install(
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
fn runtime_installs_service_plan_apps_as_events() {
    let first = runtime_app_install(
        sha256(b"first-app"),
        sha256(b"first-release"),
        sha256(b"first-code"),
        sha256(b"first-developer"),
        sha256(b"first-manifest"),
        Vec::new(),
        Vec::new(),
    );
    let second = runtime_app_install(
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
    let installed = runtime.install_service_plan_apps(&plan, 10).unwrap();
    assert_eq!(installed, 2);
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

    let installed_again = runtime.install_service_plan_apps(&plan, 20).unwrap();
    assert_eq!(installed_again, 0);
    assert_eq!(runtime.events().len(), 2);
}
