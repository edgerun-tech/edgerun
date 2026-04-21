//! Remote capability protocol: serialize hardware device access over a transport.
//!
//! ## Quick start
//!
//! ```
//! use edgerun_remote_capability::RemoteCapabilityProvider;
//! use edgerun_remote_capability::MemoryRemoteTransport;
//! ```

pub mod protocol;
pub mod transport;
pub mod policy;
pub mod adapters;
pub mod capability_signature;

// Re-export the core public API.
pub use protocol::{
    RemoteCapabilityProvider, RemoteCapabilityTransport, RemoteInvocationResult,
    serve_one, pump_one_event, capability_error_result,
    default_remote_requester,
    session_open_as_request, session_accept_from_grant, session_reject,
    accept_session_open_unchecked,
    capability_remote_envelope,
    CapabilityRemoteEnvelope, CapabilityInvocationFrame, CapabilityResultFrame,
    CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionEvent,
    CapabilitySessionMode, CapabilitySessionOpen,
};
pub use transport::{FramedRemoteTransport, accept_unix, accept_tcp, MemoryRemoteTransport};
pub use policy::{PolicyWrappedProvider, IntoPolicyWrappedProvider, SessionGrantBinding};
pub use capability_signature::{
    sign_invocation, sign_request, sign_grant, sign_result, sign_revocation,
    verify_invocation, verify_request, verify_grant, verify_result, verify_revocation,
};
pub use adapters::{
    BluetoothConnectionRemoteAdapter, BluetoothRemoteAdapter,
    CameraRemoteAdapter, PairedCameraRemoteAdapter,
    InputRemoteAdapter, MicrophoneRemoteAdapter,
    SpeakerRemoteAdapter, WifiControlRemoteAdapter, WifiRemoteAdapter,
};

#[cfg(test)]
mod tests;
