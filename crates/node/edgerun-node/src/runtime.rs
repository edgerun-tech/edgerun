use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use edgerun_protocols::wire::{
    APP_MESSAGE_STATUS_ACCEPTED, APP_MESSAGE_STATUS_DENIED, APP_MESSAGE_STATUS_FORWARDED,
    AppGraphRecord, CAPABILITY_KIND_NETWORK, CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_RECEIVE,
    CAPABILITY_STATUS_OK, CAPABILITY_STATUS_POLICY_DENIED, ROUTE_SCHEME_HTTP, ROUTE_SCHEME_HTTPS,
    RUNTIME_EVENT_APP_INSTALLED, RUNTIME_EVENT_APP_MESSAGE_DISPATCHED,
    RUNTIME_EVENT_APP_MESSAGE_FORWARDED, RUNTIME_EVENT_CAPABILITY_GRANTED,
    RUNTIME_EVENT_CAPABILITY_SESSION_OPENED, RUNTIME_EVENT_HTTP_DISPATCHED,
    RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED, RUNTIME_EVENT_NETWORK_BOUND, RUNTIME_EVENT_ROUTE_GRANTED,
    RUNTIME_EVENT_STORAGE_BOUND, RUNTIME_PROTOCOL_HTTP, RUNTIME_PROTOCOL_HTTPS,
    RUNTIME_SESSION_STATUS_OPEN, RuntimeAppInstall, RuntimeAppMessage, RuntimeCapabilityGrant,
    RuntimeCapabilitySession, RuntimeEvent, RuntimeHttpDispatch, RuntimeHttpRequest,
    RuntimeHttpRoute, RuntimeIdentityRoute, RuntimeNetworkBinding, RuntimeRoutedAppMessage,
    RuntimeStorageBinding, SDK_WIRE_ABI_VERSION, SdkWireRecord, sdk_wire_bytes,
};

use crate::storage::RuntimeStorage;

mod capabilities;
mod deployment_boundary;
mod service_plan;
mod types;
mod wire_builders;
pub use deployment_boundary::{
    RuntimeBoundaryDecision, RuntimeBoundaryIntent, RuntimeBoundarySurface, decide_runtime_boundary,
};
pub use edgerun_protocols::wire::{
    runtime_capability_grant_id, runtime_capability_session_id, runtime_network_binding_id,
    runtime_storage_binding_id,
};
pub use service_plan::{
    RuntimeAliasSpec, RuntimeBootstrapPolicy, RuntimeDeploymentSpec, RuntimeDomainSpec,
    RuntimeMailboxSpec, RuntimeServicePlan, RuntimeWebsiteSpec,
};
pub use types::{
    NoopRuntimeAppSigner, RuntimeAppSigner, RuntimeError, RuntimeLogEntry, RuntimeMessageDelivery,
    RuntimeSigner, UnsignedRuntimeSigner,
};
pub use wire_builders::*;

pub struct RuntimeKernel<S, G = UnsignedRuntimeSigner, K = NoopRuntimeAppSigner> {
    storage: S,
    signer: G,
    app_signer: K,
    apps: BTreeMap<[u8; 32], RuntimeAppInstall>,
    capability_grants: BTreeMap<[u8; 32], RuntimeCapabilityGrant>,
    storage_bindings: BTreeMap<[u8; 32], RuntimeStorageBinding>,
    network_bindings: BTreeMap<[u8; 32], RuntimeNetworkBinding>,
    capability_sessions: BTreeMap<[u8; 32], RuntimeCapabilitySession>,
    identity_routes: BTreeMap<[u8; 32], RuntimeIdentityRoute>,
    routes: Vec<RuntimeHttpRoute>,
    events: Vec<RuntimeLogEntry>,
    previous_event_sha256: [u8; 32],
}

impl<S> RuntimeKernel<S, UnsignedRuntimeSigner, NoopRuntimeAppSigner>
where
    S: RuntimeStorage,
{
    pub fn new(storage: S, runtime_id: [u8; 32]) -> Self {
        Self::with_signer(storage, UnsignedRuntimeSigner::new(runtime_id))
    }
}

impl<S, G> RuntimeKernel<S, G, NoopRuntimeAppSigner>
where
    S: RuntimeStorage,
    G: RuntimeSigner,
{
    pub fn with_signer(storage: S, signer: G) -> Self {
        Self::with_signer_and_app_signer(storage, signer, NoopRuntimeAppSigner)
    }
}

impl<S, G, K> RuntimeKernel<S, G, K>
where
    S: RuntimeStorage,
    G: RuntimeSigner,
    K: RuntimeAppSigner,
{
    pub fn with_signer_and_app_signer(storage: S, signer: G, app_signer: K) -> Self {
        Self {
            storage,
            signer,
            app_signer,
            apps: BTreeMap::new(),
            capability_grants: BTreeMap::new(),
            storage_bindings: BTreeMap::new(),
            network_bindings: BTreeMap::new(),
            capability_sessions: BTreeMap::new(),
            identity_routes: BTreeMap::new(),
            routes: Vec::new(),
            events: Vec::new(),
            previous_event_sha256: [0; 32],
        }
    }

    pub fn install_app(
        &mut self,
        install: RuntimeAppInstall,
        time: u64,
    ) -> Result<(), RuntimeError> {
        self.apps.insert(install.app_id, install.clone());
        let payload = sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(install));
        self.append_event(
            time,
            RUNTIME_EVENT_APP_INSTALLED,
            CAPABILITY_STATUS_OK,
            payload,
        );
        Ok(())
    }

    pub fn install_app_graph(
        &mut self,
        graph: AppGraphRecord,
        time: u64,
    ) -> Result<(), RuntimeError> {
        if !app_graph_runtime_install_is_bound(&graph) {
            return Err(RuntimeError::InvalidWireRecord);
        }
        self.install_app(graph.runtime_install, time)
    }

    pub fn install_app_graph_wire(
        &mut self,
        graph_bytes: &[u8],
        time: u64,
    ) -> Result<(), RuntimeError> {
        let owned = graph_bytes.to_vec();
        match edgerun_protocols::wire::from_bytes::<SdkWireRecord, edgerun_protocols::wire::WireError>(
            &owned,
        )
        .map_err(|_| RuntimeError::InvalidWireRecord)?
        {
            SdkWireRecord::AppGraph(graph) => self.install_app_graph(graph, time),
            _ => Err(RuntimeError::InvalidWireRecord),
        }
    }

    pub fn install_service_plan_apps(
        &mut self,
        plan: &RuntimeServicePlan,
        first_time: u64,
    ) -> Result<usize, RuntimeError> {
        let mut installed = 0usize;
        for app in plan.apps.iter().cloned() {
            if self.apps.contains_key(&app.app_id) {
                continue;
            }
            self.install_app(app, first_time + installed as u64)?;
            installed += 1;
        }
        Ok(installed)
    }

    pub fn install_identity_route(
        &mut self,
        route: RuntimeIdentityRoute,
        time: u64,
    ) -> Result<(), RuntimeError> {
        if route.valid_until < route.valid_from {
            self.append_event(
                time,
                RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeIdentityRoute(route)),
            );
            return Err(RuntimeError::RouteDenied);
        }
        self.identity_routes
            .insert(route.identity_id, route.clone());
        self.append_event(
            time,
            RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeIdentityRoute(route)),
        );
        Ok(())
    }

    pub fn install_http_route(
        &mut self,
        route: RuntimeHttpRoute,
        time: u64,
    ) -> Result<(), RuntimeError> {
        let app = self
            .apps
            .get(&route.app_id)
            .ok_or(RuntimeError::AppNotInstalled)?;
        let declared = app.declared_routes.iter().any(|declared| {
            declared.release_id == route.release_id
                && declared.scheme == route.scheme
                && declared.host == route.host
                && declared.path_prefix == route.path_prefix
        });
        if !declared || !self.http_route_is_bound(&route, time) {
            self.append_event(
                time,
                RUNTIME_EVENT_ROUTE_GRANTED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRoute(route)),
            );
            return Err(RuntimeError::RouteDenied);
        }
        let payload = sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRoute(route.clone()));
        self.routes.push(route);
        self.append_event(
            time,
            RUNTIME_EVENT_ROUTE_GRANTED,
            CAPABILITY_STATUS_OK,
            payload,
        );
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
            .ok_or(RuntimeError::AppNotInstalled)?;
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

    pub fn bind_network_provider(
        &mut self,
        binding: RuntimeNetworkBinding,
        time: u64,
    ) -> Result<(), RuntimeError> {
        if !self.binding_matches_grant(
            binding.grant_id,
            binding.app_id,
            binding.release_id,
            binding.scope_sha256,
            CAPABILITY_KIND_NETWORK,
            time,
        ) || binding.binding_id != expected_runtime_network_binding_id(&binding)
        {
            self.append_event(
                time,
                RUNTIME_EVENT_NETWORK_BOUND,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeNetworkBinding(binding)),
            );
            return Err(RuntimeError::CapabilityDenied);
        }
        self.network_bindings
            .insert(binding.binding_id, binding.clone());
        self.append_event(
            time,
            RUNTIME_EVENT_NETWORK_BOUND,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeNetworkBinding(binding)),
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

    pub fn dispatch_http(
        &mut self,
        request: RuntimeHttpRequest,
        time: u64,
    ) -> Result<RuntimeHttpDispatch, RuntimeError> {
        let request_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeHttpRequest(request.clone()));
        let request_sha256 = sha256(&request_bytes);
        let Some(route) = self.routes.iter().find(|route| {
            route.scheme == request.scheme
                && route.host == request.host
                && request.path.starts_with(&route.path_prefix)
        }) else {
            let dispatch = RuntimeHttpDispatch {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                status: CAPABILITY_STATUS_POLICY_DENIED,
                app_id: [0; 32],
                release_id: [0; 32],
                route_host: request.host,
                route_path_prefix: Vec::new(),
                request_sha256,
            };
            self.append_event(
                time,
                RUNTIME_EVENT_HTTP_DISPATCHED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeHttpDispatch(dispatch.clone())),
            );
            return Err(RuntimeError::RouteNotFound);
        };
        if !self.http_route_is_bound(route, time) {
            let dispatch = RuntimeHttpDispatch {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                status: CAPABILITY_STATUS_POLICY_DENIED,
                app_id: route.app_id,
                release_id: route.release_id,
                route_host: route.host.clone(),
                route_path_prefix: route.path_prefix.clone(),
                request_sha256,
            };
            self.append_event(
                time,
                RUNTIME_EVENT_HTTP_DISPATCHED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeHttpDispatch(dispatch)),
            );
            return Err(RuntimeError::RouteDenied);
        }
        let dispatch = RuntimeHttpDispatch {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            status: CAPABILITY_STATUS_OK,
            app_id: route.app_id,
            release_id: route.release_id,
            route_host: route.host.clone(),
            route_path_prefix: route.path_prefix.clone(),
            request_sha256,
        };
        self.append_event(
            time,
            RUNTIME_EVENT_HTTP_DISPATCHED,
            CAPABILITY_STATUS_OK,
            sdk_wire_bytes(&SdkWireRecord::RuntimeHttpDispatch(dispatch.clone())),
        );
        Ok(dispatch)
    }

    pub fn dispatch_app_message(
        &mut self,
        message: RuntimeAppMessage,
        time: u64,
    ) -> Result<RuntimeAppMessage, RuntimeError> {
        let status = if self.app_message_is_valid_for_local_delivery(&message) {
            APP_MESSAGE_STATUS_ACCEPTED
        } else {
            APP_MESSAGE_STATUS_DENIED
        };
        self.append_event(
            time,
            RUNTIME_EVENT_APP_MESSAGE_DISPATCHED,
            status,
            sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message.clone())),
        );
        if status == APP_MESSAGE_STATUS_ACCEPTED {
            Ok(message)
        } else {
            Err(RuntimeError::AppNotInstalled)
        }
    }

    pub fn route_app_message(
        &mut self,
        message: RuntimeAppMessage,
        time: u64,
    ) -> Result<RuntimeMessageDelivery, RuntimeError> {
        if !self.apps.contains_key(&message.from_app_id)
            || message.payload_sha256 != sha256(&message.payload)
        {
            self.append_event(
                time,
                RUNTIME_EVENT_APP_MESSAGE_DISPATCHED,
                APP_MESSAGE_STATUS_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message)),
            );
            return Err(RuntimeError::AppNotInstalled);
        }
        if self.apps.contains_key(&message.to_app_id) {
            self.append_event(
                time,
                RUNTIME_EVENT_APP_MESSAGE_DISPATCHED,
                APP_MESSAGE_STATUS_ACCEPTED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message.clone())),
            );
            return Ok(RuntimeMessageDelivery::Local(message));
        }
        let Some(route) = self
            .identity_routes
            .get(&message.to_app_id)
            .filter(|route| route_is_live(route.valid_from, route.valid_until, time))
            .cloned()
        else {
            self.append_event(
                time,
                RUNTIME_EVENT_APP_MESSAGE_DISPATCHED,
                APP_MESSAGE_STATUS_DENIED,
                sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message)),
            );
            return Err(RuntimeError::IdentityRouteNotFound);
        };
        let message_bytes = sdk_wire_bytes(&SdkWireRecord::RuntimeAppMessage(message.clone()));
        let routed = RuntimeRoutedAppMessage {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            from_runtime_id: self.signer.runtime_id(),
            to_runtime_id: route.runtime_id,
            to_node_id: route.node_id,
            message_sha256: sha256(&message_bytes),
            message,
        };
        self.append_event(
            time,
            RUNTIME_EVENT_APP_MESSAGE_FORWARDED,
            APP_MESSAGE_STATUS_FORWARDED,
            sdk_wire_bytes(&SdkWireRecord::RuntimeRoutedAppMessage(routed.clone())),
        );
        Ok(RuntimeMessageDelivery::Remote(routed))
    }

    pub fn events(&self) -> &[RuntimeLogEntry] {
        &self.events
    }

    pub fn capability_grants(&self) -> &BTreeMap<[u8; 32], RuntimeCapabilityGrant> {
        &self.capability_grants
    }

    pub fn storage_bindings(&self) -> &BTreeMap<[u8; 32], RuntimeStorageBinding> {
        &self.storage_bindings
    }

    pub fn network_bindings(&self) -> &BTreeMap<[u8; 32], RuntimeNetworkBinding> {
        &self.network_bindings
    }

    pub fn capability_sessions(&self) -> &BTreeMap<[u8; 32], RuntimeCapabilitySession> {
        &self.capability_sessions
    }

    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    fn app_message_is_valid_for_local_delivery(&self, message: &RuntimeAppMessage) -> bool {
        self.apps.contains_key(&message.from_app_id)
            && self.apps.contains_key(&message.to_app_id)
            && message.payload_sha256 == sha256(&message.payload)
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
        if !declared {
            return false;
        }

        self.storage_bindings.values().any(|binding| {
            if binding.app_id != app_id
                || binding.release_id != release_id
                || binding.namespace.as_slice() != namespace
            {
                return false;
            }
            self.grant_allows_binding(
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

    fn http_route_is_bound(&self, route: &RuntimeHttpRoute, time: u64) -> bool {
        let protocol = match route.scheme {
            ROUTE_SCHEME_HTTP => RUNTIME_PROTOCOL_HTTP,
            ROUTE_SCHEME_HTTPS => RUNTIME_PROTOCOL_HTTPS,
            _ => return false,
        };
        self.network_bindings.values().any(|binding| {
            if binding.app_id != route.app_id
                || binding.release_id != route.release_id
                || binding.protocol != protocol
                || binding.origin != route.host
            {
                return false;
            }
            self.grant_allows_binding(
                binding.grant_id,
                route.app_id,
                route.release_id,
                binding.scope_sha256,
                CAPABILITY_KIND_NETWORK,
                CAPABILITY_OPERATION_RECEIVE,
                u16::MAX,
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
        let check = RuntimeGrantCheck {
            app_id,
            release_id,
            scope_sha256,
            capability_kind,
            operation: Some(operation),
            assurance: Some(assurance),
            time,
        };
        self.capability_grants
            .get(&grant_id)
            .is_some_and(|grant| runtime_grant_matches(grant, &check))
    }

    fn capability_is_granted(
        &self,
        app_id: [u8; 32],
        release_id: [u8; 32],
        scope_sha256: [u8; 32],
        capability_kind: u16,
        operation: u16,
        assurance: u16,
        time: u64,
    ) -> bool {
        let check = RuntimeGrantCheck {
            app_id,
            release_id,
            scope_sha256,
            capability_kind,
            operation: Some(operation),
            assurance: Some(assurance),
            time,
        };
        self.capability_grants
            .values()
            .any(|grant| runtime_grant_matches(grant, &check))
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
        let check = RuntimeGrantCheck {
            app_id,
            release_id,
            scope_sha256,
            capability_kind,
            operation: None,
            assurance: None,
            time,
        };
        self.capability_grants
            .get(&grant_id)
            .is_some_and(|grant| runtime_grant_matches(grant, &check))
    }

    fn session_matches_binding(
        &self,
        grant: &RuntimeCapabilityGrant,
        session: &RuntimeCapabilitySession,
    ) -> bool {
        match grant.capability_kind {
            CAPABILITY_KIND_STORAGE => self.storage_bindings.values().any(|binding| {
                binding.grant_id == grant.grant_id && binding.capability_id == session.capability_id
            }),
            CAPABILITY_KIND_NETWORK => self.network_bindings.values().any(|binding| {
                binding.grant_id == grant.grant_id && binding.capability_id == session.capability_id
            }),
            _ => true,
        }
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

fn app_graph_runtime_install_is_bound(graph: &AppGraphRecord) -> bool {
    graph.abi_version == SDK_WIRE_ABI_VERSION
        && graph.flags & 1 == 1
        && graph.runtime_install.abi_version == SDK_WIRE_ABI_VERSION
        && graph.runtime_install.flags & 1 == 1
        && graph.runtime_install.app_id == graph.app_id
        && graph.runtime_install.developer_id == graph.developer_public_key
        && graph.runtime_install.manifest_sha256 == graph.app_manifest_sha256
        && graph.artifacts.iter().any(|artifact| {
            artifact.path.as_slice() == b"app.edapp" && artifact.sha256 == graph.app_manifest_sha256
        })
}

fn route_is_live(valid_from: u64, valid_until: u64, time: u64) -> bool {
    valid_from <= time && time <= valid_until
}

struct RuntimeGrantCheck {
    app_id: [u8; 32],
    release_id: [u8; 32],
    scope_sha256: [u8; 32],
    capability_kind: u16,
    operation: Option<u16>,
    assurance: Option<u16>,
    time: u64,
}

fn runtime_grant_matches(grant: &RuntimeCapabilityGrant, check: &RuntimeGrantCheck) -> bool {
    grant.app_id == check.app_id
        && grant.release_id == check.release_id
        && grant.scope_sha256 == check.scope_sha256
        && grant.capability_kind == check.capability_kind
        && check
            .operation
            .is_none_or(|operation| grant.operation == operation)
        && check
            .assurance
            .is_none_or(|assurance| assurance >= grant.min_assurance)
        && route_is_live(grant.valid_from, grant.valid_until, check.time)
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

fn expected_runtime_network_binding_id(binding: &RuntimeNetworkBinding) -> [u8; 32] {
    runtime_network_binding_id(
        binding.grant_id,
        binding.provider_id,
        binding.capability_id,
        binding.binding_kind,
        binding.protocol,
        binding.port,
        &binding.origin,
        binding.methods_sha256,
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

#[cfg(test)]
mod tests;
