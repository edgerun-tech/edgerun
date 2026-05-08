use alloc::vec::Vec;

use edgerun_protocols::wire::RuntimeProtocolBinding;

use super::{TransportAddress, TransportCarrier, TransportProtocol};

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

/// Concrete bind target for native host adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeSocketBind {
    pub binding: RuntimeProtocolBinding,
    pub address: TransportAddress,
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

pub fn native_socket_bind(decision: &ServiceBindingDecision) -> Option<NativeSocketBind> {
    let ServiceBindingDecision::NativeSocket(binding) = decision else {
        return None;
    };
    let protocol = match binding.protocol {
        edgerun_protocols::wire::RUNTIME_PROTOCOL_DNS_UDP
        | edgerun_protocols::wire::RUNTIME_PROTOCOL_TFTP => TransportProtocol::Datagram,
        _ => TransportProtocol::Stream,
    };
    let endpoint = socket_endpoint(binding);
    Some(NativeSocketBind {
        binding: binding.clone(),
        address: TransportAddress {
            carrier: TransportCarrier::HostSocket,
            protocol,
            endpoint,
        },
    })
}

pub fn native_socket_binds(decisions: &[ServiceBindingDecision]) -> Vec<NativeSocketBind> {
    decisions.iter().filter_map(native_socket_bind).collect()
}

fn socket_endpoint(binding: &RuntimeProtocolBinding) -> Vec<u8> {
    let [a, b, c, d] = binding.bind_ipv4;
    alloc::format!("{a}.{b}.{c}.{d}:{}", binding.port).into_bytes()
}
