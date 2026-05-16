#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_wire::{
    app_run_prompt_decision_id, from_bytes, package_cache_id, runtime_capability_grant_id,
    runtime_capability_session_id, runtime_storage_binding_id, sdk_wire_bytes, to_bytes,
    AppRunPromptDecisionRecord, CapabilityRequest, CapabilityResponse,
    CapabilityResponseProofRecord, PackageCacheRecord, RuntimeAppInstall, RuntimeCapabilityGrant,
    RuntimeCapabilitySession, RuntimeEvent, RuntimeStorageBinding, SdkWireRecord,
    StorageWriteReceiptRecord, WireError, APP_RUN_DECISION_CANCEL, APP_RUN_DECISION_RUN_ONCE,
    APP_RUN_DECISION_VERIFY_AND_CACHE, CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_READ,
    CAPABILITY_OPERATION_WRITE, CAPABILITY_STATUS_INVALID_REQUEST, CAPABILITY_STATUS_OK,
    CAPABILITY_STATUS_POLICY_DENIED, PACKAGE_CACHE_STATE_VERIFIED,
    RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED, RUNTIME_EVENT_CAPABILITY_DENIED,
    RUNTIME_EVENT_CAPABILITY_EXECUTED, RUNTIME_EVENT_CAPABILITY_GRANTED,
    RUNTIME_EVENT_CAPABILITY_SESSION_OPENED, RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
    RUNTIME_EVENT_PACKAGE_VERIFIED, RUNTIME_EVENT_STORAGE_BOUND, RUNTIME_SESSION_STATUS_OPEN,
    SDK_WIRE_ABI_VERSION,
};
use edgerun_work::capability_packet::{
    verify_capability_envelope_shape, CapabilityEnvelope, CAPABILITY_CONTENT_OBJECT,
    CAPABILITY_OPERATION_OBJECT_GET, CAPABILITY_OPERATION_OBJECT_PUT, CAPABILITY_PACKET_INVOKE,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidWireRecord,
    AppNotVerified,
    CapabilityDenied,
    StorageDenied,
    StorageProvider(String),
}

impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidWireRecord => f.write_str("invalid runtime wire record"),
            Self::AppNotVerified => f.write_str("app is not verified in runtime state"),
            Self::CapabilityDenied => f.write_str("capability grant or binding denied"),
            Self::StorageDenied => f.write_str("storage namespace is not granted to app"),
            Self::StorageProvider(error) => write!(f, "storage provider failed: {error}"),
        }
    }
}

pub trait RuntimeStorage {
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError>;
}

pub trait BrowserRuntimeStorageHost {
    fn read_object(
        &mut self,
        namespace: &[u8],
        key: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn write_object(
        &mut self,
        namespace: &[u8],
        key: &[u8; 32],
        value: &[u8],
    ) -> Result<(), RuntimeError>;
}

pub struct BrowserRuntimeStorage<H> {
    host: H,
}

impl<H> BrowserRuntimeStorage<H> {
    pub const fn new(host: H) -> Self {
        Self { host }
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    pub fn into_host(self) -> H {
        self.host
    }
}

impl<H> RuntimeStorage for BrowserRuntimeStorage<H>
where
    H: BrowserRuntimeStorageHost,
{
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError> {
        let key: [u8; 32] = key.try_into().map_err(|_| RuntimeError::StorageDenied)?;
        self.host.read_object(namespace, &key)
    }

    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError> {
        let key: [u8; 32] = key.try_into().map_err(|_| RuntimeError::StorageDenied)?;
        self.host.write_object(namespace, &key, value)
    }
}

#[derive(Default)]
pub struct MemoryRuntimeStorage {
    objects: BTreeMap<(Vec<u8>, [u8; 32]), Vec<u8>>,
}

impl RuntimeStorage for MemoryRuntimeStorage {
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError> {
        let key: [u8; 32] = key.try_into().map_err(|_| RuntimeError::StorageDenied)?;
        Ok(self.objects.get(&(namespace.to_vec(), key)).cloned())
    }

    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError> {
        let key: [u8; 32] = key.try_into().map_err(|_| RuntimeError::StorageDenied)?;
        self.objects
            .insert((namespace.to_vec(), key), value.to_vec());
        Ok(())
    }
}

pub trait RuntimeSigner {
    fn runtime_id(&self) -> [u8; 32];
    fn sign_event_payload(&mut self, payload_sha256: &[u8; 32]) -> Vec<u8>;
}

pub struct UnsignedRuntimeSigner {
    runtime_id: [u8; 32],
}

impl UnsignedRuntimeSigner {
    pub const fn new(runtime_id: [u8; 32]) -> Self {
        Self { runtime_id }
    }
}

impl RuntimeSigner for UnsignedRuntimeSigner {
    fn runtime_id(&self) -> [u8; 32] {
        self.runtime_id
    }

    fn sign_event_payload(&mut self, _payload_sha256: &[u8; 32]) -> Vec<u8> {
        Vec::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLogEntry {
    pub event: RuntimeEvent,
    pub event_sha256: [u8; 32],
}

pub struct RuntimeKernel<S, G = UnsignedRuntimeSigner> {
    storage: S,
    signer: G,
    apps: BTreeMap<[u8; 32], RuntimeAppInstall>,
    app_run_decisions: BTreeMap<[u8; 32], AppRunPromptDecisionRecord>,
    package_caches: BTreeMap<[u8; 32], PackageCacheRecord>,
    capability_grants: BTreeMap<[u8; 32], RuntimeCapabilityGrant>,
    storage_bindings: BTreeMap<[u8; 32], RuntimeStorageBinding>,
    capability_sessions: BTreeMap<[u8; 32], RuntimeCapabilitySession>,
    events: Vec<RuntimeLogEntry>,
    previous_event_sha256: [u8; 32],
}

impl<S> RuntimeKernel<S, UnsignedRuntimeSigner>
where
    S: RuntimeStorage,
{
    pub fn new(storage: S, runtime_id: [u8; 32]) -> Self {
        Self::with_signer(storage, UnsignedRuntimeSigner::new(runtime_id))
    }
}

impl<S, G> RuntimeKernel<S, G>
where
    S: RuntimeStorage,
    G: RuntimeSigner,
{
    pub fn with_signer(storage: S, signer: G) -> Self {
        Self {
            storage,
            signer,
            apps: BTreeMap::new(),
            app_run_decisions: BTreeMap::new(),
            package_caches: BTreeMap::new(),
            capability_grants: BTreeMap::new(),
            storage_bindings: BTreeMap::new(),
            capability_sessions: BTreeMap::new(),
            events: Vec::new(),
            previous_event_sha256: [0; 32],
        }
    }

    pub fn record_app_first_run_projection(
        &mut self,
        runtime_projection: RuntimeAppInstall,
        decision: AppRunPromptDecisionRecord,
        cache: Option<PackageCacheRecord>,
        time: u64,
    ) -> Result<(), RuntimeError> {
        if !first_run_decision_matches_app(&decision, &runtime_projection) {
            self.append_event(
                time,
                RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(decision)),
            );
            return Err(RuntimeError::InvalidWireRecord);
        }

        let should_cache = decision.decision == APP_RUN_DECISION_VERIFY_AND_CACHE;
        let is_cancel = decision.decision == APP_RUN_DECISION_CANCEL;
        match (should_cache, cache.as_ref()) {
            (true, Some(cache))
                if package_cache_matches_app(cache, &runtime_projection, &decision) => {}
            (true, _) => {
                self.append_event(
                    time,
                    RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
                    CAPABILITY_STATUS_POLICY_DENIED,
                    cache
                        .as_ref()
                        .map(|cache| sdk_wire_bytes(&SdkWireRecord::PackageCache(cache.clone())))
                        .unwrap_or_default(),
                );
                return Err(RuntimeError::InvalidWireRecord);
            }
            (false, Some(cache)) => {
                self.append_event(
                    time,
                    RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
                    CAPABILITY_STATUS_POLICY_DENIED,
                    sdk_wire_bytes(&SdkWireRecord::PackageCache(cache.clone())),
                );
                return Err(RuntimeError::InvalidWireRecord);
            }
            (false, None) => {}
        }

        self.append_event(
            time,
            RUNTIME_EVENT_PACKAGE_VERIFIED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(
                runtime_projection.clone(),
            )),
        );
        self.app_run_decisions
            .insert(decision.decision_id, decision.clone());
        self.append_event(
            time,
            RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(decision)),
        );
        if let Some(cache) = cache {
            self.package_caches.insert(cache.cache_id, cache.clone());
            self.append_event(
                time,
                RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
                CAPABILITY_STATUS_OK,
                sdk_wire_bytes(&SdkWireRecord::PackageCache(cache)),
            );
        }
        if !is_cancel {
            self.apps
                .insert(runtime_projection.app_id, runtime_projection);
        }
        Ok(())
    }

    pub fn grant_runtime_capability(
        &mut self,
        grant: RuntimeCapabilityGrant,
        time: u64,
    ) -> Result<(), RuntimeError> {
        let app = self
            .apps
            .get(&grant.app_id)
            .ok_or(RuntimeError::AppNotVerified)?;
        if app.release_id != grant.release_id
            || grant.valid_until < grant.valid_from
            || grant.grant_id != expected_runtime_capability_grant_id(&grant)
        {
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_GRANTED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityGrant(grant)),
            );
            return Err(RuntimeError::CapabilityDenied);
        }
        self.capability_grants.insert(grant.grant_id, grant.clone());
        self.append_event(
            time,
            RUNTIME_EVENT_CAPABILITY_GRANTED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilityGrant(grant)),
        );
        Ok(())
    }

    pub fn bind_storage_provider(
        &mut self,
        binding: RuntimeStorageBinding,
        time: u64,
    ) -> Result<(), RuntimeError> {
        if !self.binding_matches_grant(
            binding.grant_id,
            binding.app_id,
            binding.release_id,
            binding.scope_sha256,
            CAPABILITY_KIND_STORAGE,
            time,
        ) || binding.binding_id != expected_runtime_storage_binding_id(&binding)
        {
            self.append_event(
                time,
                RUNTIME_EVENT_STORAGE_BOUND,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeStorageBinding(binding)),
            );
            return Err(RuntimeError::CapabilityDenied);
        }
        self.storage_bindings
            .insert(binding.binding_id, binding.clone());
        self.append_event(
            time,
            RUNTIME_EVENT_STORAGE_BOUND,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeStorageBinding(binding)),
        );
        Ok(())
    }

    pub fn open_capability_session(
        &mut self,
        session: RuntimeCapabilitySession,
        time: u64,
    ) -> Result<(), RuntimeError> {
        let Some(grant) = self.capability_grants.get(&session.grant_id) else {
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_SESSION_OPENED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session)),
            );
            return Err(RuntimeError::CapabilityDenied);
        };
        if grant.app_id != session.app_id
            || grant.release_id != session.release_id
            || session.status != RUNTIME_SESSION_STATUS_OPEN
            || session.capability_id == [0; 32]
            || session.provider_node_id == [0; 32]
            || session.admission_hash == [0; 32]
            || session.route_commitment == [0; 32]
            || session.session_id != expected_runtime_capability_session_id(&session)
            || !self.session_matches_binding(grant, &session)
            || !route_is_live(grant.valid_from, grant.valid_until, time)
            || session.valid_until < time
            || session.valid_until > grant.valid_until
        {
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_SESSION_OPENED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session)),
            );
            return Err(RuntimeError::CapabilityDenied);
        }
        self.capability_sessions
            .insert(session.session_id, session.clone());
        self.append_event(
            time,
            RUNTIME_EVENT_CAPABILITY_SESSION_OPENED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeCapabilitySession(session)),
        );
        Ok(())
    }

    pub fn invoke_storage_envelope(
        &mut self,
        envelope: &CapabilityEnvelope,
        provider: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, RuntimeError> {
        if !storage_envelope_shape_is_supported(envelope) {
            return Err(RuntimeError::InvalidWireRecord);
        }
        let request = decode_capability_request_wire(&envelope.payload)?;
        if !self.storage_envelope_session_is_live(envelope, &request, time) {
            let response = self.denied_storage_response(&envelope.payload, &request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)));
        }
        let response = self.invoke_storage(&envelope.payload, &request, provider, time)?;
        Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)))
    }

    pub fn events(&self) -> &[RuntimeLogEntry] {
        &self.events
    }

    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    fn invoke_storage(
        &mut self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
        time: u64,
    ) -> Result<CapabilityResponse, RuntimeError> {
        if request.capability_kind != CAPABILITY_KIND_STORAGE
            || !self.storage_request_is_bound(
                request.app_id,
                request.release_id,
                &request.context,
                request.operation,
                request.assurance,
                time,
            )
        {
            let response = self.denied_storage_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }

        let payload = match request.operation {
            CAPABILITY_OPERATION_WRITE => {
                if request.payload_sha256 != sha256(&request.payload) {
                    return Ok(CapabilityResponse {
                        abi_version: SDK_WIRE_ABI_VERSION,
                        flags: 1,
                        capability_kind: CAPABILITY_KIND_STORAGE,
                        operation: request.operation,
                        status: CAPABILITY_STATUS_INVALID_REQUEST,
                        assurance: request.assurance,
                        request_sha256: sha256(request_bytes),
                        provider: provider.to_vec(),
                        responder: self.signer.runtime_id().to_vec(),
                        payload: b"payload_sha256_mismatch".to_vec(),
                        proof: Vec::new(),
                    });
                }
                self.storage
                    .write(&request.context, &request.subject_sha256, &request.payload)?;
                storage_write_receipt_payload(&request.payload)
            }
            CAPABILITY_OPERATION_READ => {
                let value = self
                    .storage
                    .read(&request.context, &request.subject_sha256)?
                    .unwrap_or_default();
                if request.payload_sha256 != [0u8; 32] && request.payload_sha256 != sha256(&value) {
                    return Ok(CapabilityResponse {
                        abi_version: SDK_WIRE_ABI_VERSION,
                        flags: 1,
                        capability_kind: CAPABILITY_KIND_STORAGE,
                        operation: request.operation,
                        status: CAPABILITY_STATUS_INVALID_REQUEST,
                        assurance: request.assurance,
                        request_sha256: sha256(request_bytes),
                        provider: provider.to_vec(),
                        responder: self.signer.runtime_id().to_vec(),
                        payload: b"read_payload_sha256_mismatch".to_vec(),
                        proof: Vec::new(),
                    });
                }
                value
            }
            _ => {
                let response = self.denied_storage_response(request_bytes, request, provider);
                self.append_event(
                    time,
                    RUNTIME_EVENT_CAPABILITY_DENIED,
                    CAPABILITY_STATUS_POLICY_DENIED,
                    sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
                );
                return Ok(response);
            }
        };
        let proof = storage_response_proof(request_bytes, request.operation, provider, &payload);
        let response = CapabilityResponse::ok(
            sha256(request_bytes),
            CAPABILITY_KIND_STORAGE,
            request.operation,
            request.assurance,
            provider.to_vec(),
            self.signer.runtime_id().to_vec(),
            payload,
            proof.to_vec(),
        );
        self.append_event(
            time,
            RUNTIME_EVENT_CAPABILITY_EXECUTED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
        );
        Ok(response)
    }

    fn storage_envelope_session_is_live(
        &self,
        envelope: &CapabilityEnvelope,
        request: &CapabilityRequest,
        time: u64,
    ) -> bool {
        let Some(session) = self.capability_sessions.get(&envelope.session_id) else {
            return false;
        };
        session.capability_id == envelope.capability_id
            && session.provider_node_id == envelope.target_node_id
            && envelope.target_node_id == self.signer.runtime_id()
            && session.app_id == request.app_id
            && session.release_id == request.release_id
            && session.status == RUNTIME_SESSION_STATUS_OPEN
            && session.valid_until >= time
            && match envelope.operation {
                CAPABILITY_OPERATION_OBJECT_GET => request.operation == CAPABILITY_OPERATION_READ,
                CAPABILITY_OPERATION_OBJECT_PUT => request.operation == CAPABILITY_OPERATION_WRITE,
                _ => false,
            }
    }

    fn storage_request_is_bound(
        &self,
        app_id: [u8; 32],
        release_id: [u8; 32],
        namespace: &[u8],
        operation: u16,
        assurance: u16,
        time: u64,
    ) -> bool {
        let declared = self.apps.get(&app_id).is_some_and(|app| {
            app.release_id == release_id
                && app
                    .storage_namespaces
                    .iter()
                    .any(|declared| declared.as_slice() == namespace)
        });
        declared
            && self.storage_bindings.values().any(|binding| {
                binding.app_id == app_id
                    && binding.release_id == release_id
                    && binding.namespace.as_slice() == namespace
                    && self.grant_allows_binding(
                        binding.grant_id,
                        app_id,
                        release_id,
                        binding.scope_sha256,
                        CAPABILITY_KIND_STORAGE,
                        operation,
                        assurance,
                        time,
                    )
            })
    }

    fn grant_allows_binding(
        &self,
        grant_id: [u8; 32],
        app_id: [u8; 32],
        release_id: [u8; 32],
        scope_sha256: [u8; 32],
        capability_kind: u16,
        operation: u16,
        assurance: u16,
        time: u64,
    ) -> bool {
        self.capability_grants.get(&grant_id).is_some_and(|grant| {
            grant.app_id == app_id
                && grant.release_id == release_id
                && grant.scope_sha256 == scope_sha256
                && grant.capability_kind == capability_kind
                && grant.operation == operation
                && assurance >= grant.min_assurance
                && route_is_live(grant.valid_from, grant.valid_until, time)
        })
    }

    fn binding_matches_grant(
        &self,
        grant_id: [u8; 32],
        app_id: [u8; 32],
        release_id: [u8; 32],
        scope_sha256: [u8; 32],
        capability_kind: u16,
        time: u64,
    ) -> bool {
        self.capability_grants.get(&grant_id).is_some_and(|grant| {
            grant.app_id == app_id
                && grant.release_id == release_id
                && grant.scope_sha256 == scope_sha256
                && grant.capability_kind == capability_kind
                && route_is_live(grant.valid_from, grant.valid_until, time)
        })
    }

    fn session_matches_binding(
        &self,
        grant: &RuntimeCapabilityGrant,
        session: &RuntimeCapabilitySession,
    ) -> bool {
        self.storage_bindings.values().any(|binding| {
            grant.capability_kind == CAPABILITY_KIND_STORAGE
                && binding.grant_id == grant.grant_id
                && binding.capability_id == session.capability_id
        })
    }

    fn denied_storage_response(
        &self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
    ) -> CapabilityResponse {
        let reason = b"policy_denied".to_vec();
        let proof = capability_denial_proof(
            request_bytes,
            CAPABILITY_STATUS_POLICY_DENIED,
            provider,
            &reason,
        );
        CapabilityResponse::denied(
            sha256(request_bytes),
            request.capability_kind,
            request.operation,
            request.assurance,
            provider.to_vec(),
            self.signer.runtime_id().to_vec(),
            reason,
            proof.to_vec(),
        )
    }

    fn append_event(&mut self, time: u64, event_kind: u16, status: u16, payload: Vec<u8>) {
        let payload_sha256 = sha256(&payload);
        let mut event = RuntimeEvent::unsigned_payload(
            self.events.len() as u64,
            time,
            event_kind,
            status,
            self.signer.runtime_id(),
            self.previous_event_sha256,
            payload_sha256,
            payload,
        );
        event.signature = self.signer.sign_event_payload(&payload_sha256);
        let event_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(event.clone()));
        let event_sha256 = sha256(&event_bytes);
        self.previous_event_sha256 = event_sha256;
        self.events.push(RuntimeLogEntry {
            event,
            event_sha256,
        });
    }
}

pub fn verify_runtime_event_chain(entries: &[RuntimeLogEntry]) -> bool {
    let mut previous = [0u8; 32];
    for (index, entry) in entries.iter().enumerate() {
        if entry.event.seq != index as u64
            || entry.event.previous_event_sha256 != previous
            || entry.event.payload_sha256 != sha256(&entry.event.payload)
        {
            return false;
        }
        let event_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(entry.event.clone()));
        let event_sha256 = sha256(&event_bytes);
        if entry.event_sha256 != event_sha256 {
            return false;
        }
        previous = event_sha256;
    }
    true
}

pub fn runtime_app_projection(
    app_id: [u8; 32],
    release_id: [u8; 32],
    code_sha256: [u8; 32],
    developer_id: [u8; 32],
    manifest_sha256: [u8; 32],
    declared_routes: Vec<edgerun_wire::RuntimeHttpRoute>,
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

#[allow(clippy::too_many_arguments)]
pub fn runtime_capability_grant(
    profile_id: [u8; 32],
    user_id: [u8; 32],
    app_id: [u8; 32],
    release_id: [u8; 32],
    capability_kind: u16,
    operation: u16,
    min_assurance: u16,
    scope_sha256: [u8; 32],
    constraints_sha256: [u8; 32],
    valid_from: u64,
    valid_until: u64,
    user_signature: Vec<u8>,
) -> RuntimeCapabilityGrant {
    let grant_id = runtime_capability_grant_id(
        profile_id,
        user_id,
        app_id,
        release_id,
        capability_kind,
        operation,
        scope_sha256,
        constraints_sha256,
        valid_from,
        valid_until,
    );
    RuntimeCapabilityGrant {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        grant_id,
        profile_id,
        user_id,
        app_id,
        release_id,
        capability_kind,
        operation,
        min_assurance,
        scope_sha256,
        constraints_sha256,
        valid_from,
        valid_until,
        user_signature,
    }
}

pub fn runtime_storage_binding(
    grant: &RuntimeCapabilityGrant,
    namespace: impl Into<Vec<u8>>,
    provider_id: [u8; 32],
    capability_id: [u8; 32],
    backing_kind: u16,
) -> RuntimeStorageBinding {
    let namespace = namespace.into();
    let binding_id =
        runtime_storage_binding_id(grant.grant_id, &namespace, provider_id, capability_id);
    RuntimeStorageBinding {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        binding_id,
        grant_id: grant.grant_id,
        profile_id: grant.profile_id,
        app_id: grant.app_id,
        release_id: grant.release_id,
        namespace,
        provider_id,
        capability_id,
        backing_kind,
        scope_sha256: grant.scope_sha256,
    }
}

pub fn runtime_capability_session(
    grant: &RuntimeCapabilityGrant,
    capability_id: [u8; 32],
    provider_node_id: [u8; 32],
    admission_hash: [u8; 32],
    route_commitment: [u8; 32],
    valid_until: u64,
) -> RuntimeCapabilitySession {
    let session_id = runtime_capability_session_id(
        grant.grant_id,
        grant.app_id,
        grant.release_id,
        capability_id,
        provider_node_id,
        admission_hash,
        route_commitment,
    );
    RuntimeCapabilitySession {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        session_id,
        grant_id: grant.grant_id,
        app_id: grant.app_id,
        release_id: grant.release_id,
        capability_id,
        provider_node_id,
        admission_hash,
        route_commitment,
        valid_until,
        status: RUNTIME_SESSION_STATUS_OPEN,
    }
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    edgerun_crypto::sha256(bytes)
}

fn storage_write_receipt_payload(payload: &[u8]) -> Vec<u8> {
    to_bytes::<WireError>(&StorageWriteReceiptRecord {
        payload_sha256: sha256(payload),
        payload_len: payload.len() as u64,
    })
    .expect("storage write receipt must serialize through rkyv")
    .into_vec()
}

fn storage_response_proof(
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
    sha256(&to_bytes::<WireError>(&record).expect("storage response proof must serialize"))
}

fn capability_denial_proof(
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
    sha256(&to_bytes::<WireError>(&record).expect("capability denial proof must serialize"))
}

fn first_run_decision_matches_app(
    decision: &AppRunPromptDecisionRecord,
    app: &RuntimeAppInstall,
) -> bool {
    decision.abi_version == SDK_WIRE_ABI_VERSION
        && decision.flags & 1 == 1
        && decision.app_id == app.app_id
        && decision.release_id == app.release_id
        && decision.manifest_sha256 == app.manifest_sha256
        && decision.package_sha256 != [0; 32]
        && matches!(
            decision.decision,
            APP_RUN_DECISION_RUN_ONCE | APP_RUN_DECISION_VERIFY_AND_CACHE | APP_RUN_DECISION_CANCEL
        )
        && decision.decision_id
            == app_run_prompt_decision_id(
                decision.profile_id,
                decision.app_id,
                decision.release_id,
                decision.package_sha256,
                decision.manifest_sha256,
                decision.decision,
                decision.decided_at,
            )
}

fn package_cache_matches_app(
    cache: &PackageCacheRecord,
    app: &RuntimeAppInstall,
    decision: &AppRunPromptDecisionRecord,
) -> bool {
    cache.abi_version == SDK_WIRE_ABI_VERSION
        && cache.flags & 1 == 1
        && cache.state == PACKAGE_CACHE_STATE_VERIFIED
        && cache.profile_id == decision.profile_id
        && cache.app_id == app.app_id
        && cache.release_id == app.release_id
        && cache.package_sha256 == decision.package_sha256
        && cache.manifest_sha256 == app.manifest_sha256
        && cache.code_sha256 == app.code_sha256
        && cache.cache_id
            == package_cache_id(
                cache.profile_id,
                cache.app_id,
                cache.release_id,
                cache.package_sha256,
                cache.manifest_sha256,
            )
}

fn route_is_live(valid_from: u64, valid_until: u64, time: u64) -> bool {
    valid_from <= time && time <= valid_until
}

fn expected_runtime_capability_grant_id(grant: &RuntimeCapabilityGrant) -> [u8; 32] {
    runtime_capability_grant_id(
        grant.profile_id,
        grant.user_id,
        grant.app_id,
        grant.release_id,
        grant.capability_kind,
        grant.operation,
        grant.scope_sha256,
        grant.constraints_sha256,
        grant.valid_from,
        grant.valid_until,
    )
}

fn expected_runtime_storage_binding_id(binding: &RuntimeStorageBinding) -> [u8; 32] {
    runtime_storage_binding_id(
        binding.grant_id,
        &binding.namespace,
        binding.provider_id,
        binding.capability_id,
    )
}

fn expected_runtime_capability_session_id(session: &RuntimeCapabilitySession) -> [u8; 32] {
    runtime_capability_session_id(
        session.grant_id,
        session.app_id,
        session.release_id,
        session.capability_id,
        session.provider_node_id,
        session.admission_hash,
        session.route_commitment,
    )
}

fn storage_envelope_shape_is_supported(envelope: &CapabilityEnvelope) -> bool {
    verify_capability_envelope_shape(envelope)
        && envelope.kind == CAPABILITY_PACKET_INVOKE
        && envelope.content_type == CAPABILITY_CONTENT_OBJECT
        && matches!(
            envelope.operation,
            CAPABILITY_OPERATION_OBJECT_GET | CAPABILITY_OPERATION_OBJECT_PUT
        )
}

fn decode_capability_request_wire(request_bytes: &[u8]) -> Result<CapabilityRequest, RuntimeError> {
    let owned = request_bytes.to_vec();
    match from_bytes::<SdkWireRecord, WireError>(&owned)
        .map_err(|_| RuntimeError::InvalidWireRecord)?
    {
        SdkWireRecord::CapabilityRequest(request) => Ok(request),
        _ => Err(RuntimeError::InvalidWireRecord),
    }
}
