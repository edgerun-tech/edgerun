use alloc::vec::Vec;

use edgerun_protocols::wire::{
    CAPABILITY_OPERATION_SIGN, CapabilityRequest, CapabilityResponseProofRecord,
    RUNTIME_PROTOCOL_ACME, RUNTIME_PROTOCOL_DNS_TCP, RUNTIME_PROTOCOL_DNS_UDP,
    RUNTIME_PROTOCOL_HTTP, RUNTIME_PROTOCOL_HTTPS, RUNTIME_PROTOCOL_IMAP, RUNTIME_PROTOCOL_IMAPS,
    RUNTIME_PROTOCOL_LMTP, RUNTIME_PROTOCOL_PROXY, RUNTIME_PROTOCOL_SMTP,
    RUNTIME_PROTOCOL_SUBMISSION, RUNTIME_PROTOCOL_TFTP, RuntimeAppInstall, RuntimeAppMessage,
    RuntimeCapabilityDeclaration, RuntimeDeploymentConfig, RuntimeDomainConfig, RuntimeHttpRequest,
    RuntimeHttpRoute, RuntimeIdentityRoute, RuntimeProtocolBinding, SDK_WIRE_ABI_VERSION,
    SigningCapabilityInputRecord, SigningResponsePayloadRecord, StorageWriteReceiptRecord,
};

pub fn runtime_app_install(
    app_id: [u8; 32],
    release_id: [u8; 32],
    code_sha256: [u8; 32],
    developer_id: [u8; 32],
    manifest_sha256: [u8; 32],
    declared_routes: Vec<RuntimeHttpRoute>,
    storage_namespaces: Vec<Vec<u8>>,
) -> RuntimeAppInstall {
    RuntimeAppInstall {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        release_id,
        code_sha256,
        developer_id,
        manifest_sha256,
        declared_routes,
        storage_namespaces,
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
    }
}

pub fn runtime_app_install_with_capabilities(
    app_id: [u8; 32],
    release_id: [u8; 32],
    code_sha256: [u8; 32],
    developer_id: [u8; 32],
    manifest_sha256: [u8; 32],
    declared_routes: Vec<RuntimeHttpRoute>,
    storage_namespaces: Vec<Vec<u8>>,
    provided_capabilities: Vec<RuntimeCapabilityDeclaration>,
    required_capabilities: Vec<RuntimeCapabilityDeclaration>,
) -> RuntimeAppInstall {
    RuntimeAppInstall {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        release_id,
        code_sha256,
        developer_id,
        manifest_sha256,
        declared_routes,
        storage_namespaces,
        provided_capabilities,
        required_capabilities,
    }
}

pub fn runtime_capability_declaration(
    capability_kind: u16,
    operation: u16,
    min_assurance: u16,
    scope_sha256: [u8; 32],
    label: impl Into<Vec<u8>>,
    context: impl Into<Vec<u8>>,
) -> RuntimeCapabilityDeclaration {
    RuntimeCapabilityDeclaration {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        capability_kind,
        operation,
        min_assurance,
        scope_sha256,
        label: label.into(),
        context: context.into(),
    }
}

pub fn runtime_http_route(
    app_id: [u8; 32],
    release_id: [u8; 32],
    scheme: u16,
    host: impl Into<Vec<u8>>,
    path_prefix: impl Into<Vec<u8>>,
) -> RuntimeHttpRoute {
    RuntimeHttpRoute {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        release_id,
        scheme,
        host: host.into(),
        path_prefix: path_prefix.into(),
    }
}

pub fn runtime_http_request(
    method: u16,
    scheme: u16,
    host: impl Into<Vec<u8>>,
    path: impl Into<Vec<u8>>,
    header_sha256: [u8; 32],
    body: Vec<u8>,
) -> RuntimeHttpRequest {
    RuntimeHttpRequest {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        method,
        scheme,
        host: host.into(),
        path: path.into(),
        header_sha256,
        body_sha256: sha256(&body),
        body,
    }
}

pub fn runtime_app_message(
    from_app_id: [u8; 32],
    to_app_id: [u8; 32],
    message_kind: u16,
    payload: Vec<u8>,
) -> RuntimeAppMessage {
    RuntimeAppMessage {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        from_app_id,
        to_app_id,
        message_kind,
        payload_sha256: sha256(&payload),
        payload,
    }
}

pub fn runtime_identity_route(
    identity_id: [u8; 32],
    runtime_id: [u8; 32],
    node_id: [u8; 64],
    valid_from: u64,
    valid_until: u64,
    route_hint: Vec<u8>,
) -> RuntimeIdentityRoute {
    RuntimeIdentityRoute {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        identity_id,
        runtime_id,
        node_id,
        valid_from,
        valid_until,
        route_hint,
    }
}

pub fn runtime_protocol_binding(
    protocol: u16,
    port: u16,
    bind_ipv4: [u8; 4],
) -> RuntimeProtocolBinding {
    RuntimeProtocolBinding {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        protocol,
        port,
        bind_ipv4,
        host: Vec::new(),
    }
}

pub fn default_protocol_bindings(bind_ipv4: [u8; 4]) -> Vec<RuntimeProtocolBinding> {
    [
        (RUNTIME_PROTOCOL_DNS_UDP, 53),
        (RUNTIME_PROTOCOL_DNS_TCP, 53),
        (RUNTIME_PROTOCOL_HTTP, 80),
        (RUNTIME_PROTOCOL_HTTPS, 443),
        (RUNTIME_PROTOCOL_SMTP, 25),
        (RUNTIME_PROTOCOL_SUBMISSION, 587),
        (RUNTIME_PROTOCOL_IMAP, 143),
        (RUNTIME_PROTOCOL_IMAPS, 993),
        (RUNTIME_PROTOCOL_LMTP, 24),
        (RUNTIME_PROTOCOL_TFTP, 69),
        (RUNTIME_PROTOCOL_PROXY, 0),
        (RUNTIME_PROTOCOL_ACME, 0),
    ]
    .into_iter()
    .map(|(protocol, port)| runtime_protocol_binding(protocol, port, bind_ipv4))
    .collect()
}

pub fn runtime_deployment_config(
    runtime_id: [u8; 32],
    public_ipv4: [u8; 4],
    hostname: impl Into<Vec<u8>>,
    origin: impl Into<Vec<u8>>,
    acme_contact: impl Into<Vec<u8>>,
    protocol_bindings: Vec<RuntimeProtocolBinding>,
    domains: Vec<RuntimeDomainConfig>,
    apps: Vec<RuntimeAppInstall>,
) -> RuntimeDeploymentConfig {
    RuntimeDeploymentConfig {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        runtime_id,
        public_ipv4,
        hostname: hostname.into(),
        origin: origin.into(),
        acme_contact: acme_contact.into(),
        protocol_bindings,
        domains,
        apps,
    }
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    edgerun_crypto::sha256(bytes)
}

pub fn storage_write_receipt_payload(payload: &[u8]) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(
        &StorageWriteReceiptRecord {
            payload_sha256: sha256(payload),
            payload_len: payload.len() as u64,
        },
    )
    .expect("storage write receipt must serialize through rkyv")
    .into_vec()
}

pub fn storage_response_proof(
    request_bytes: &[u8],
    operation: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    let record = CapabilityResponseProofRecord {
        domain: b"edgerun-runtime.storage-response.v1".to_vec(),
        request_sha256: sha256(request_bytes),
        operation_or_status: operation,
        provider: provider.to_vec(),
        payload_sha256: sha256(payload),
    };
    sha256(
        &edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&record)
            .expect("storage response proof must serialize through rkyv"),
    )
}

pub fn signing_capability_input(request: &CapabilityRequest) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(
        &SigningCapabilityInputRecord {
            domain: b"edgerun-runtime.app-signing.v1".to_vec(),
            app_id: request.app_id,
            release_id: request.release_id,
            subject_sha256: request.subject_sha256,
            payload_sha256: request.payload_sha256,
            payload: request.payload.clone(),
        },
    )
    .expect("signing capability input must serialize through rkyv")
    .into_vec()
}

pub fn signing_response_payload(public_key: &[u8], signature: &[u8]) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(
        &SigningResponsePayloadRecord {
            public_key: public_key.to_vec(),
            signature: signature.to_vec(),
        },
    )
    .expect("signing response payload must serialize through rkyv")
    .into_vec()
}

pub fn signing_response_proof(
    request_bytes: &[u8],
    provider: &[u8],
    public_key: &[u8],
    signature: &[u8],
) -> [u8; 32] {
    let mut payload = Vec::new();
    payload.extend_from_slice(&sha256(public_key));
    payload.extend_from_slice(&sha256(signature));
    let record = CapabilityResponseProofRecord {
        domain: b"edgerun-runtime.signing-response.v1".to_vec(),
        request_sha256: sha256(request_bytes),
        operation_or_status: CAPABILITY_OPERATION_SIGN,
        provider: provider.to_vec(),
        payload_sha256: sha256(&payload),
    };
    sha256(
        &edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&record)
            .expect("signing response proof must serialize through rkyv"),
    )
}

pub fn capability_denial_proof(
    request_bytes: &[u8],
    status: u16,
    provider: &[u8],
    reason: &[u8],
) -> [u8; 32] {
    let record = CapabilityResponseProofRecord {
        domain: b"edgerun-runtime.capability-denial.v1".to_vec(),
        request_sha256: sha256(request_bytes),
        operation_or_status: status,
        provider: provider.to_vec(),
        payload_sha256: sha256(reason),
    };
    sha256(
        &edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(&record)
            .expect("capability denial proof must serialize through rkyv"),
    )
}
