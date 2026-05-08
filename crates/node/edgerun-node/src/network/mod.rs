//! Network capability boundary.
//!
//! Protocol services deal in streams and datagrams. The node decides whether a
//! transport is a host socket, bare network frame path, mesh route, email-carried
//! frame, browser IPC route, or another runtime carrier.

mod bare;
mod bindings;
mod host;
mod provider;
mod types;

pub use bare::BareFrameTransport;
pub use bindings::{
    binding_intents, decide_binding, decide_bindings, native_socket_bind,
    native_socket_binding_is_supported, native_socket_binds, NativeSocketBind,
    NodeTransportSurface, ServiceBindingDecision, ServiceBindingIntent,
};
pub use host::{HostConnectFuture, HostSocketTransport};
pub use provider::{
    BoxedNodeStream, NodeDatagram, NodeStream, NodeStreamListener, Ready, RuntimeTransport,
};
pub use types::{
    TransportAddress, TransportCarrier, TransportError, TransportFrame, TransportProtocol,
};
