//! Remote capability protocol: serialize hardware device access over a transport.
//!
//! ## Quick start
//!
//! ```
//! use edgerun_remote_capability::RemoteCapabilityProvider;
//! use edgerun_remote_capability::MemoryRemoteTransport;
//! ```

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub mod cell {
    pub use core::cell::*;
}

pub mod rc {
    pub use alloc::rc::*;
}

pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
}

pub mod boxed {
    pub use alloc::boxed::Box;
}

pub mod string {
    pub use alloc::string::{String, ToString};
}

pub mod vec {
    pub use alloc::vec::Vec;
}

pub use alloc::format;
pub use core::{cmp, convert, fmt, option, result, slice, str};

pub mod adapters;
pub mod capability_signature;
pub mod policy;
pub mod protocol;
pub mod transport;

// Re-export the core public API.
pub use adapters::{
    BluetoothConnectionRemoteAdapter, BluetoothRemoteAdapter, CameraRemoteAdapter,
    DisplayRemoteAdapter, InputRemoteAdapter, MicrophoneRemoteAdapter, PairedCameraRemoteAdapter,
    SpeakerRemoteAdapter, WifiControlRemoteAdapter, WifiRemoteAdapter,
};
pub use policy::{IntoPolicyWrappedProvider, PolicyWrappedProvider, SessionGrantBinding};
pub use protocol::{
    accept_session_open_unchecked, capability_error_result, capability_remote_envelope,
    default_remote_requester, pump_one_event, serve_one, session_accept_from_grant,
    session_open_as_request, session_reject, CapabilityInvocationFrame, CapabilityRemoteEnvelope,
    CapabilityResultFrame, CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionEvent,
    CapabilitySessionMode, CapabilitySessionOpen, RemoteCapabilityProvider,
    RemoteCapabilityTransport, RemoteInvocationResult,
};
pub use transport::MemoryRemoteTransport;
#[cfg(not(target_os = "none"))]
pub use transport::{accept_tcp, accept_unix, FramedRemoteTransport};

#[cfg(test)]
mod tests;
