use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use edgerun_wire::{
    sdk_wire_bytes, CapabilityRequest, CapabilityResponse, CapabilityResponseProofRecord,
    RuntimeAppInstall, RuntimeAppMessage, RuntimeDeploymentConfig, RuntimeDomainConfig,
    RuntimeEvent, RuntimeHttpDispatch, RuntimeHttpRequest, RuntimeHttpRoute, RuntimeIdentityRoute,
    RuntimeProtocolBinding, RuntimeRoutedAppMessage, SdkWireRecord, SigningCapabilityInputRecord,
    SigningResponsePayloadRecord, StorageWriteReceiptRecord, APP_MESSAGE_STATUS_ACCEPTED,
    APP_MESSAGE_STATUS_DENIED, APP_MESSAGE_STATUS_FORWARDED, CAPABILITY_KIND_SIGNING,
    CAPABILITY_KIND_STORAGE, CAPABILITY_OPERATION_READ, CAPABILITY_OPERATION_SIGN,
    CAPABILITY_OPERATION_WRITE, CAPABILITY_STATUS_INVALID_REQUEST, CAPABILITY_STATUS_OK,
    CAPABILITY_STATUS_POLICY_DENIED, ROUTE_SCHEME_HTTPS, RUNTIME_EVENT_APP_INSTALLED,
    RUNTIME_EVENT_APP_MESSAGE_DISPATCHED, RUNTIME_EVENT_APP_MESSAGE_FORWARDED,
    RUNTIME_EVENT_CAPABILITY_DENIED, RUNTIME_EVENT_CAPABILITY_EXECUTED,
    RUNTIME_EVENT_HTTP_DISPATCHED, RUNTIME_EVENT_IDENTITY_ROUTE_GRANTED,
    RUNTIME_EVENT_ROUTE_GRANTED, RUNTIME_PROTOCOL_ACME, RUNTIME_PROTOCOL_DNS_TCP,
    RUNTIME_PROTOCOL_DNS_UDP, RUNTIME_PROTOCOL_HTTP, RUNTIME_PROTOCOL_HTTPS, RUNTIME_PROTOCOL_IMAP,
    RUNTIME_PROTOCOL_IMAPS, RUNTIME_PROTOCOL_LMTP, RUNTIME_PROTOCOL_PROXY, RUNTIME_PROTOCOL_SMTP,
    RUNTIME_PROTOCOL_SUBMISSION, RUNTIME_PROTOCOL_TFTP, SDK_WIRE_ABI_VERSION,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidWireRecord,
    AppNotInstalled,
    RouteDenied,
    RouteNotFound,
    IdentityRouteNotFound,
    SigningDenied,
    SigningProvider(String),
    StorageDenied,
    StorageProvider(String),
}

impl core::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidWireRecord => f.write_str("invalid runtime wire record"),
            Self::AppNotInstalled => f.write_str("app is not installed"),
            Self::RouteDenied => f.write_str("route is not declared by installed app"),
            Self::RouteNotFound => f.write_str("no route matched request"),
            Self::IdentityRouteNotFound => f.write_str("no identity route matched recipient"),
            Self::SigningDenied => f.write_str("signing capability denied"),
            Self::SigningProvider(error) => write!(f, "signing provider failed: {error}"),
            Self::StorageDenied => f.write_str("storage namespace is not granted to app"),
            Self::StorageProvider(error) => write!(f, "storage provider failed: {error}"),
        }
    }
}

impl core::error::Error for RuntimeError {}

pub trait RuntimeStorage {
    fn read(&mut self, namespace: &[u8], key: &[u8]) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn write(&mut self, namespace: &[u8], key: &[u8], value: &[u8]) -> Result<(), RuntimeError>;
}

pub trait RuntimeSigner {
    fn runtime_id(&self) -> [u8; 32];
    fn sign_event_payload(&mut self, payload_sha256: &[u8; 32]) -> Vec<u8>;
}

pub trait RuntimeAppSigner {
    fn app_public_key(&mut self, app_id: &[u8; 32]) -> Result<Option<Vec<u8>>, RuntimeError>;
    fn sign_app_payload(
        &mut self,
        app_id: &[u8; 32],
        payload: &[u8],
    ) -> Result<Vec<u8>, RuntimeError>;
}

pub struct UnsignedRuntimeSigner {
    runtime_id: [u8; 32],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopRuntimeAppSigner;

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

impl RuntimeAppSigner for NoopRuntimeAppSigner {
    fn app_public_key(&mut self, _app_id: &[u8; 32]) -> Result<Option<Vec<u8>>, RuntimeError> {
        Ok(None)
    }

    fn sign_app_payload(
        &mut self,
        _app_id: &[u8; 32],
        _payload: &[u8],
    ) -> Result<Vec<u8>, RuntimeError> {
        Err(RuntimeError::SigningDenied)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLogEntry {
    pub event: RuntimeEvent,
    pub event_sha256: [u8; 32],
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeServicePlan {
    pub runtime_id: [u8; 32],
    pub public_ipv4: [u8; 4],
    pub hostname: Vec<u8>,
    pub origin: Vec<u8>,
    pub listeners: Vec<RuntimeProtocolBinding>,
    pub domains: Vec<RuntimeDomainConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeMessageDelivery {
    Local(RuntimeAppMessage),
    Remote(RuntimeRoutedAppMessage),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeBootstrapPolicy {
    pub node_label: &'static str,
    pub stream_id: [u8; 32],
    pub controller_id: [u8; 64],
    pub bootstrap_relays: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeMailboxSpec {
    pub address: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeAliasSpec {
    pub address: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeWebsiteSpec {
    pub domain: &'static str,
    pub repo: &'static str,
    pub commit: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeDomainSpec {
    pub domain: &'static str,
    pub authoritative_dns: bool,
    pub mailboxes: &'static [RuntimeMailboxSpec],
    pub aliases: &'static [RuntimeAliasSpec],
    pub website: Option<RuntimeWebsiteSpec>,
}

impl RuntimeDomainSpec {
    pub fn mail_enabled(&self) -> bool {
        !self.mailboxes.is_empty() || !self.aliases.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeDeploymentSpec {
    pub policy: RuntimeBootstrapPolicy,
    pub public_ipv4: [u8; 4],
    pub hostname: &'static str,
    pub origin: &'static str,
    pub mail_host_prefix: &'static str,
    pub maildir_root: &'static str,
    pub queue_data_root: &'static str,
    pub dkim_domain: &'static str,
    pub dkim_selector: &'static str,
    pub dkim_key_path: &'static str,
    pub tls_cert_path: &'static str,
    pub tls_key_path: &'static str,
    pub acme_account_key_path: &'static str,
    pub runtime_root: &'static str,
    pub derived_db_path: &'static str,
    pub acme_contact: &'static str,
    pub domains: &'static [RuntimeDomainSpec],
    pub external_cnames: &'static [(&'static str, &'static str)],
}

impl RuntimeDeploymentSpec {
    pub fn certificate_domains(&self) -> Vec<String> {
        let mut out = Vec::new();
        for domain in self.domains {
            if domain.mail_enabled() {
                out.push(self.mail_host_for_domain(domain.domain));
            }
            if let Some(site) = domain.website {
                out.push(site.domain.to_string());
            }
            out.push(format_bytes(b"mta-sts.", domain.domain.as_bytes()));
        }
        out.sort();
        out.dedup();
        out
    }

    pub fn local_mail_domains(&self) -> Vec<String> {
        let mut domains = Vec::new();
        for domain in self.domains {
            if domain.mail_enabled() || domain.domain == self.origin {
                domains.push(domain.domain.to_string());
            }
        }
        domains.sort();
        domains.dedup();
        domains
    }

    pub fn mail_host_for_domain(&self, domain: &str) -> String {
        if self.mail_host_prefix.is_empty() {
            domain.to_string()
        } else {
            let mut out = String::new();
            out.push_str(self.mail_host_prefix);
            out.push('.');
            out.push_str(domain);
            out
        }
    }

    pub fn to_wire_config(&self) -> RuntimeDeploymentConfig {
        let mut domains = Vec::new();
        let mut apps = Vec::new();
        for domain in self.domains {
            let mut routes = Vec::new();
            if let Some(site) = domain.website {
                let app_id = sha256(site.domain.as_bytes());
                let release_id = sha256(site.commit.as_bytes());
                routes.push(runtime_http_route(
                    app_id,
                    release_id,
                    ROUTE_SCHEME_HTTPS,
                    site.domain.as_bytes().to_vec(),
                    site.path.as_bytes().to_vec(),
                ));
                apps.push(runtime_app_install(
                    app_id,
                    release_id,
                    sha256(site.repo.as_bytes()),
                    self.policy.stream_id,
                    sha256(site.path.as_bytes()),
                    routes.clone(),
                    vec![site.domain.as_bytes().to_vec()],
                ));
            }
            let mut mailboxes = Vec::new();
            for mailbox in domain.mailboxes {
                mailboxes.push(edgerun_wire::RuntimeMailbox {
                    abi_version: SDK_WIRE_ABI_VERSION,
                    flags: 1,
                    address: mailbox.address.as_bytes().to_vec(),
                    target_app_id: sha256(mailbox.target.as_bytes()),
                });
            }
            domains.push(RuntimeDomainConfig {
                abi_version: SDK_WIRE_ABI_VERSION,
                flags: 1,
                domain: domain.domain.as_bytes().to_vec(),
                authoritative_dns: domain.authoritative_dns,
                mail_enabled: domain.mail_enabled(),
                acme_enabled: domain.authoritative_dns,
                mailboxes,
                routes,
            });
        }
        runtime_deployment_config(
            self.policy.stream_id,
            self.public_ipv4,
            self.hostname.as_bytes().to_vec(),
            self.origin.as_bytes().to_vec(),
            self.acme_contact.as_bytes().to_vec(),
            default_protocol_bindings([0, 0, 0, 0]),
            domains,
            apps,
        )
    }
}

impl RuntimeServicePlan {
    pub fn from_deployment(config: &RuntimeDeploymentConfig) -> Self {
        let mut listeners = config.protocol_bindings.clone();
        if listeners.is_empty() {
            listeners = default_protocol_bindings([0, 0, 0, 0]);
        }
        listeners.sort_by_key(|binding| (binding.protocol, binding.port, binding.bind_ipv4));
        listeners.dedup_by(|a, b| {
            a.protocol == b.protocol
                && a.port == b.port
                && a.bind_ipv4 == b.bind_ipv4
                && a.host == b.host
        });
        Self {
            runtime_id: config.runtime_id,
            public_ipv4: config.public_ipv4,
            hostname: config.hostname.clone(),
            origin: config.origin.clone(),
            listeners,
            domains: config.domains.clone(),
        }
    }

    pub fn requires_dns(&self) -> bool {
        self.listeners.iter().any(|binding| {
            binding.protocol == RUNTIME_PROTOCOL_DNS_UDP
                || binding.protocol == RUNTIME_PROTOCOL_DNS_TCP
        })
    }

    pub fn requires_acme(&self) -> bool {
        self.domains.iter().any(|domain| domain.acme_enabled)
            || self
                .listeners
                .iter()
                .any(|binding| binding.protocol == RUNTIME_PROTOCOL_ACME)
    }

    pub fn mail_domains(&self) -> Vec<Vec<u8>> {
        let mut domains: Vec<Vec<u8>> = self
            .domains
            .iter()
            .filter(|domain| domain.mail_enabled)
            .map(|domain| domain.domain.clone())
            .collect();
        domains.sort();
        domains.dedup();
        domains
    }
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

    pub fn invoke_storage_wire(
        &mut self,
        request_bytes: &[u8],
        provider: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, RuntimeError> {
        let request = decode_capability_request_wire(request_bytes)?;
        let response = self.invoke_storage(request_bytes, &request, provider, time)?;
        Ok(sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response)))
    }

    pub fn invoke_signing_wire(
        &mut self,
        request_bytes: &[u8],
        provider: &[u8],
        time: u64,
    ) -> Result<Vec<u8>, RuntimeError> {
        let request = decode_capability_request_wire(request_bytes)?;
        let response = self.invoke_signing(request_bytes, &request, provider, time)?;
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
        if request.capability_kind != CAPABILITY_KIND_STORAGE {
            let response = self.denied_storage_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }
        let allowed = self.apps.get(&request.app_id).is_some_and(|app| {
            app.storage_namespaces
                .iter()
                .any(|namespace| namespace.as_slice() == request.context.as_slice())
        });
        if !allowed {
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

    fn invoke_signing(
        &mut self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
        time: u64,
    ) -> Result<CapabilityResponse, RuntimeError> {
        if request.capability_kind != CAPABILITY_KIND_SIGNING
            || request.operation != CAPABILITY_OPERATION_SIGN
        {
            let response = self.denied_capability_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }
        let allowed = self.apps.get(&request.app_id).is_some_and(|app| {
            app.release_id == request.release_id
                && request.payload_sha256 == sha256(&request.payload)
        });
        if !allowed {
            let response = self.denied_capability_response(request_bytes, request, provider);
            self.append_event(
                time,
                RUNTIME_EVENT_CAPABILITY_DENIED,
                CAPABILITY_STATUS_POLICY_DENIED,
                sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
            );
            return Ok(response);
        }
        let public_key = match self.app_signer.app_public_key(&request.app_id)? {
            Some(public_key) => public_key,
            None => {
                let response = self.denied_capability_response(request_bytes, request, provider);
                self.append_event(
                    time,
                    RUNTIME_EVENT_CAPABILITY_DENIED,
                    CAPABILITY_STATUS_POLICY_DENIED,
                    sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(response.clone())),
                );
                return Ok(response);
            }
        };
        let signature = self
            .app_signer
            .sign_app_payload(&request.app_id, &signing_capability_input(request))?;
        let payload = signing_response_payload(&public_key, &signature);
        let proof = signing_response_proof(request_bytes, provider, &public_key, &signature);
        let response = CapabilityResponse::ok(
            sha256(request_bytes),
            CAPABILITY_KIND_SIGNING,
            CAPABILITY_OPERATION_SIGN,
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

    fn denied_storage_response(
        &self,
        request_bytes: &[u8],
        request: &CapabilityRequest,
        provider: &[u8],
    ) -> CapabilityResponse {
        self.denied_capability_response(request_bytes, request, provider)
    }

    fn denied_capability_response(
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

fn decode_capability_request_wire(request_bytes: &[u8]) -> Result<CapabilityRequest, RuntimeError> {
    let owned = request_bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|_| RuntimeError::InvalidWireRecord)?
    {
        SdkWireRecord::CapabilityRequest(request) => Ok(request),
        _ => Err(RuntimeError::InvalidWireRecord),
    }
}

#[cfg(feature = "std")]
#[derive(Default)]
pub struct MemoryRuntimeStorage {
    objects: BTreeMap<(Vec<u8>, [u8; 32]), Vec<u8>>,
}

#[cfg(feature = "std")]
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

fn format_bytes(prefix: &[u8], suffix: &[u8]) -> String {
    let mut out = String::new();
    out.push_str(core::str::from_utf8(prefix).unwrap_or(""));
    out.push_str(core::str::from_utf8(suffix).unwrap_or(""));
    out
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    edgerun_crypto::sha256(bytes)
}

pub fn storage_write_receipt_payload(payload: &[u8]) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&StorageWriteReceiptRecord {
        payload_sha256: sha256(payload),
        payload_len: payload.len() as u64,
    })
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
        &edgerun_wire::to_bytes::<edgerun_wire::WireError>(&record)
            .expect("storage response proof must serialize through rkyv"),
    )
}

pub fn signing_capability_input(request: &CapabilityRequest) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&SigningCapabilityInputRecord {
        domain: b"edgerun-runtime.app-signing.v1".to_vec(),
        app_id: request.app_id,
        release_id: request.release_id,
        subject_sha256: request.subject_sha256,
        payload_sha256: request.payload_sha256,
        payload: request.payload.clone(),
    })
    .expect("signing capability input must serialize through rkyv")
    .into_vec()
}

pub fn signing_response_payload(public_key: &[u8], signature: &[u8]) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&SigningResponsePayloadRecord {
        public_key: public_key.to_vec(),
        signature: signature.to_vec(),
    })
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
        &edgerun_wire::to_bytes::<edgerun_wire::WireError>(&record)
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
        &edgerun_wire::to_bytes::<edgerun_wire::WireError>(&record)
            .expect("capability denial proof must serialize through rkyv"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_wire::{
        from_bytes, RuntimeDomainConfig, RuntimeMailbox, HTTP_METHOD_GET, ROUTE_SCHEME_HTTPS,
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
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&write_response_bytes)
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
            from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&read_response_bytes)
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
        let response = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&response_bytes)
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
        let response = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&response_bytes)
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
        let response = from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&response_bytes)
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
}
