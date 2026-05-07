use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use edgerun_protocols::wire::{
    sdk_wire_bytes, RuntimeAppInstall, RuntimeAppMessage, RuntimeEvent, RuntimeHttpDispatch,
    RuntimeHttpRequest, RuntimeHttpRoute, RuntimeIdentityRoute, RuntimeRoutedAppMessage,
    SdkWireRecord, APP_MESSAGE_STATUS_ACCEPTED, APP_MESSAGE_STATUS_DENIED,
    APP_MESSAGE_STATUS_FORWARDED, CAPABILITY_STATUS_OK, CAPABILITY_STATUS_POLICY_DENIED,
    RUNTIME_EVENT_APP_INSTALLED, RUNTIME_EVENT_APP_MESSAGE_DISPATCHED,
    RUNTIME_EVENT_APP_MESSAGE_FORWARDED, RUNTIME_EVENT_CAPABILITY_DENIED,
    RUNTIME_EVENT_HTTP_DISPATCHED, RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED,
    RUNTIME_EVENT_ROUTE_GRANTED, SDK_WIRE_ABI_VERSION,
};

use crate::storage::RuntimeStorage;

mod capabilities;
mod service_plan;
mod types;
mod wire_builders;
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
        self.identity_routes.insert(
            install.app_id,
            runtime_identity_route(
                install.app_id,
                self.signer.runtime_id(),
                [0; 64],
                0,
                0,
                Vec::new(),
            ),
        );
        let payload = sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(install));
        self.append_event(
            time,
            RUNTIME_EVENT_APP_INSTALLED,
            CAPABILITY_STATUS_OK,
            payload,
        );
        Ok(())
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

    pub fn grant_identity_route(
        &mut self,
        route: RuntimeIdentityRoute,
        time: u64,
    ) -> Result<(), RuntimeError> {
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

    pub fn grant_http_route(
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
        if !declared {
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
        let Some(route) = self.identity_routes.get(&message.to_app_id).cloned() else {
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

    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    fn app_message_is_valid_for_local_delivery(&self, message: &RuntimeAppMessage) -> bool {
        self.apps.contains_key(&message.from_app_id)
            && self.apps.contains_key(&message.to_app_id)
            && message.payload_sha256 == sha256(&message.payload)
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

#[cfg(test)]
mod tests;
