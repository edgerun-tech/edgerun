//! Node IPC routing boundary.
//!
//! The node routes data between app identities, protocol state machines,
//! capability providers, and host transports. It is the authority that decides
//! where bytes go. Apps do not bind ports, open shared memory, or directly own
//! sockets; they send messages and capability requests to this router boundary.

use alloc::vec::Vec;

use edgerun_protocols::wire::{
    CapabilityRequest, CapabilityResponse, RuntimeAppMessage, RuntimeHttpDispatch,
    RuntimeHttpRequest, RuntimeProtocolBinding, RuntimeRoutedAppMessage,
};

use crate::resource::ServiceBindingDecision;
use crate::runtime::{RuntimeError, RuntimeMessageDelivery};

/// Transport-independent input presented to the node router.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeRouteInput {
    /// HTTP bytes have already been parsed into the runtime request shape.
    Http(RuntimeHttpRequest),
    /// App-to-app message by identity.
    AppMessage(RuntimeAppMessage),
    /// App capability invocation, such as storage or signing.
    Capability {
        request_bytes: Vec<u8>,
        provider: Vec<u8>,
    },
    /// Deployment/app requested a protocol binding. The node may realize it as
    /// a native socket, route it through mesh/browser IPC, or deny it.
    ProtocolBinding(RuntimeProtocolBinding),
}

/// Transport-independent router result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeRouteDecision {
    /// Request matched a local app HTTP route.
    Http(RuntimeHttpDispatch),
    /// Message targets a local app managed by this runtime.
    LocalApp(RuntimeAppMessage),
    /// Message targets a remote identity/runtime.
    RemoteApp(RuntimeRoutedAppMessage),
    /// Capability invocation completed.
    Capability(CapabilityResponse),
    /// Protocol binding was accepted, routed, or denied by node policy.
    Resource(ServiceBindingDecision),
    /// The node rejected the input before delivery.
    Rejected(RuntimeError),
}

impl From<Result<RuntimeMessageDelivery, RuntimeError>> for NodeRouteDecision {
    fn from(result: Result<RuntimeMessageDelivery, RuntimeError>) -> Self {
        match result {
            Ok(RuntimeMessageDelivery::Local(message)) => Self::LocalApp(message),
            Ok(RuntimeMessageDelivery::Remote(message)) => Self::RemoteApp(message),
            Err(error) => Self::Rejected(error),
        }
    }
}

impl From<Result<RuntimeHttpDispatch, RuntimeError>> for NodeRouteDecision {
    fn from(result: Result<RuntimeHttpDispatch, RuntimeError>) -> Self {
        match result {
            Ok(dispatch) => Self::Http(dispatch),
            Err(error) => Self::Rejected(error),
        }
    }
}

impl From<Result<CapabilityResponse, RuntimeError>> for NodeRouteDecision {
    fn from(result: Result<CapabilityResponse, RuntimeError>) -> Self {
        match result {
            Ok(response) => Self::Capability(response),
            Err(error) => Self::Rejected(error),
        }
    }
}

impl From<ServiceBindingDecision> for NodeRouteDecision {
    fn from(decision: ServiceBindingDecision) -> Self {
        Self::Resource(decision)
    }
}

pub fn capability_request_from_input(input: &NodeRouteInput) -> Option<(&[u8], &[u8])> {
    match input {
        NodeRouteInput::Capability {
            request_bytes,
            provider,
        } => Some((request_bytes, provider)),
        _ => None,
    }
}

pub fn capability_request_record(input: CapabilityRequest) -> NodeRouteInput {
    NodeRouteInput::Capability {
        request_bytes: edgerun_protocols::wire::sdk_wire_bytes(
            &edgerun_protocols::wire::SdkWireRecord::CapabilityRequest(input),
        ),
        provider: Vec::new(),
    }
}
