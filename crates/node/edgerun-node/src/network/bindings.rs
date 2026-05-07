use alloc::vec::Vec;

use edgerun_protocols::wire::RuntimeProtocolBinding;

/// Transport surface available to this node runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeTransportSurface {
    /// Native host sockets are available.
    NativeSocket,
    /// Browser-style message transport is available; no native ports exist.
    BrowserMessage,
    /// Mesh transport is available; routing is identity/message based.
    Mesh,
    /// In-process tests or embedded runtime.
    InProcess,
    /// No transport can realize this binding.
    Unavailable,
}

/// A requested service binding.
///
/// This is not proof that a port is bound. It is an app/runtime declaration
/// that the node can use as a routing input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceBindingIntent {
    pub binding: RuntimeProtocolBinding,
    pub surface: NodeTransportSurface,
}

/// Runtime decision for a requested network binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceBindingDecision {
    NativeSocket(RuntimeProtocolBinding),
    Routed(RuntimeProtocolBinding),
    Denied(RuntimeProtocolBinding),
}

pub fn binding_intents(
    bindings: &[RuntimeProtocolBinding],
    surface: NodeTransportSurface,
) -> Vec<ServiceBindingIntent> {
    bindings
        .iter()
        .cloned()
        .map(|binding| ServiceBindingIntent { binding, surface })
        .collect()
}

pub fn decide_binding(intent: ServiceBindingIntent) -> ServiceBindingDecision {
    match intent.surface {
        NodeTransportSurface::NativeSocket => ServiceBindingDecision::NativeSocket(intent.binding),
        NodeTransportSurface::BrowserMessage
        | NodeTransportSurface::Mesh
        | NodeTransportSurface::InProcess => ServiceBindingDecision::Routed(intent.binding),
        NodeTransportSurface::Unavailable => ServiceBindingDecision::Denied(intent.binding),
    }
}

pub fn decide_bindings(intents: Vec<ServiceBindingIntent>) -> Vec<ServiceBindingDecision> {
    intents.into_iter().map(decide_binding).collect()
}
